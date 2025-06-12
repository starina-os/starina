use starina::prelude::*;
use thiserror::Error;

use crate::guest_memory;

mod packet;
mod packet_builder;
mod packet_parser;
mod tcp;

use packet::ArpOp;
pub use packet::IpProto;
pub use packet::Ipv4Addr;
pub use packet::MacAddr;
use packet_builder::PacketBuilder;
use packet_builder::TxPacket;
use packet_parser::PacketParser;
use packet_parser::RxEthPacket;
use packet_parser::RxPacket;
use tcp::TcpManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnKey {
    pub proto: IpProto,
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
    pub guest_port: u16,
}

pub trait PacketReader {
    fn read_bytes(&mut self, read_len: usize) -> Result<&[u8], guest_memory::Error>;
}

pub trait PacketWriter {
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), guest_memory::Error>;
    fn written_len(&self) -> usize;
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error("Guest memory error: {0}")]
    GuestMemory(#[from] guest_memory::Error),
    #[error("Packet building failed: {0}")]
    PacketBuild(#[from] packet_builder::BuildError),
    #[error("Unknown connection")]
    UnknownConn,
}

#[derive(Debug, Error)]
pub enum RecvError {
    #[error("Guest memory error: {0}")]
    GuestMemory(#[from] guest_memory::Error),
    #[error("Invalid packet: {0}")]
    InvalidPacket(#[from] packet_parser::ParseError),
    #[error("ARP request for invalid IP")]
    InvalidArpTarget,
}

pub struct GuestNet {
    host_ip: Ipv4Addr,
    guest_ip: Ipv4Addr,
    guest_mac: MacAddr,
    host_mac: MacAddr,
    gw_ip: Ipv4Addr,
    netmask: Ipv4Addr,
    dns_servers: [Ipv4Addr; 2],
    tcp_manager: TcpManager,
    pending_arp_reply: bool,
    next_host_port: u16,
}

impl GuestNet {
    pub fn new(
        host_ip: Ipv4Addr,
        guest_ip: Ipv4Addr,
        guest_mac: MacAddr,
        host_mac: MacAddr,
        gw_ip: Ipv4Addr,
        netmask: Ipv4Addr,
        dns_servers: [Ipv4Addr; 2],
    ) -> Self {
        Self {
            host_ip,
            guest_ip,
            guest_mac,
            host_mac,
            gw_ip,
            netmask,
            dns_servers,
            tcp_manager: TcpManager::new(guest_ip, guest_mac, host_mac),
            pending_arp_reply: false,
            next_host_port: 40000,
        }
    }

    pub fn build_linux_ip_param(&self) -> String {
        format!(
            "ip={}::{}:{}::eth0:off:{}:{}",
            self.guest_ip, self.gw_ip, self.netmask, self.dns_servers[0], self.dns_servers[1]
        )
    }

    /// Initiates a TCP connection to the guest by setting SYN flag to be sent later.
    /// Returns the ConnKey for the new connection.
    pub fn connect_to_guest(
        &mut self,
        guest_port: u16,
        proto: IpProto,
        forwarder: Box<dyn FnMut(&ConnKey, &[u8])>,
    ) -> ConnKey {
        // Use a standard remote IP for virtual connections
        let remote_ip = Ipv4Addr::new(10, 123, 123, 123);

        // Assign a unique remote port by checking if the connection already exists
        let connkey = loop {
            let remote_port = self.next_host_port;
            self.next_host_port = self.next_host_port.wrapping_add(1);

            let connkey = ConnKey {
                proto,
                remote_ip,
                remote_port,
                guest_port,
            };

            if !self.tcp_manager.has_connection(&connkey) {
                break connkey;
            }
        };

        self.tcp_manager.connect_to_guest(connkey, forwarder);
        connkey
    }

    /// Writes TCP payload to the guest using proper connection state.
    pub fn send_to_guest(
        &mut self,
        writer: impl PacketWriter,
        key: &ConnKey,
        data: &[u8],
    ) -> Result<Option<usize /* packet len */>, SendError> {
        self.tcp_manager.send_to_guest(writer, key, data)
    }

    pub fn recv_from_guest(&mut self, reader: impl PacketReader) -> Result<(), RecvError> {
        let eth_packet = match PacketParser::parse(reader) {
            Ok(packet) => packet,
            Err(e) => {
                debug_warn!("failed to parse packet: {:?}", e);
                // Do not return error, just ignore the packet.
                return Ok(());
            }
        };

        // Centralized MAC address verification - ignore packet if verification fails.
        if !self.verify_mac_addresses(&eth_packet) {
            return Ok(());
        }

        match eth_packet.packet {
            RxPacket::Arp(ref arp) => {
                self.handle_arp_packet(arp, &eth_packet)?;
            }
            RxPacket::Tcp(tcp) => {
                trace!(
                    "TCP {:08b}: [{}:{} -> {}:{}] seq={} ack={} {} bytes",
                    tcp.flags,
                    tcp.src_ip,
                    tcp.src_port,
                    tcp.dst_ip,
                    tcp.dst_port,
                    tcp.seq_num,
                    tcp.ack_num,
                    tcp.payload.len()
                );

                // Handle TCP packet and update connection state.
                self.tcp_manager.handle_tcp_packet(&tcp);
            }
            RxPacket::UnknownIpv4 {
                src_ip,
                dst_ip,
                ip_proto,
                payload,
                ..
            } => {
                trace!(
                    "Unknown IPv4 packet (protocol {}): {} -> {}, {} bytes payload",
                    ip_proto,
                    src_ip,
                    dst_ip,
                    payload.len()
                );
            }
            RxPacket::UnknownEth {
                ether_type,
                payload_len,
                ..
            } => {
                trace!(
                    "Unknown ethernet packet (type 0x{:04x}): {} bytes",
                    ether_type, payload_len
                );
            }
        }
        Ok(())
    }

    pub fn has_pending_packets(&self) -> bool {
        self.pending_arp_reply || self.tcp_manager.has_pending_packets()
    }

    pub fn send_pending_packet(
        &mut self,
        writer: impl PacketWriter,
    ) -> Result<Option<usize /* packet len */>, SendError> {
        if self.pending_arp_reply {
            let arp_reply = TxPacket::Arp {
                operation: ArpOp::Reply,
                sender_hw_addr: self.host_mac,
                sender_ip: self.host_ip,
                target_hw_addr: self.guest_mac,
                target_ip: self.guest_ip,
            };

            let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
            let written_len = builder.send(&arp_reply)?;

            self.pending_arp_reply = false;
            return Ok(Some(written_len));
        }

        // Send TCP packets if needed.
        if self.tcp_manager.has_pending_packets() {
            return self.tcp_manager.send_pending_packet(writer);
        }

        Ok(None)
    }

    /// Centralized MAC address verification for all packet types.
    /// Returns true if packet should be processed, false if it should be ignored.
    fn verify_mac_addresses(&self, eth_packet: &RxEthPacket) -> bool {
        let mut valid = true;

        // Verify source MAC is from guest.
        if eth_packet.src_mac != self.guest_mac {
            debug_warn!(
                "Packet with invalid src_mac: expected {:?}, got {:?} - ignoring packet",
                self.guest_mac,
                eth_packet.src_mac
            );
            valid = false;
        }

        // For most packets, dst_mac should be host_mac.
        // Exception: ARP requests can be broadcast.
        let is_arp_broadcast =
            matches!(&eth_packet.packet, RxPacket::Arp(arp) if arp.operation == ArpOp::Request);

        if !is_arp_broadcast && eth_packet.dst_mac != self.host_mac {
            debug_warn!(
                "Packet with invalid dst_mac: expected {:?}, got {:?} - ignoring packet",
                self.host_mac,
                eth_packet.dst_mac
            );
            valid = false;
        } else if is_arp_broadcast
            && eth_packet.dst_mac != MacAddr::BROADCAST
            && eth_packet.dst_mac != self.host_mac
        {
            debug_warn!(
                "ARP packet with unexpected dst_mac: expected {:?} or {:?}, got {:?} - ignoring packet",
                MacAddr::BROADCAST,
                self.host_mac,
                eth_packet.dst_mac
            );
            valid = false;
        }

        valid
    }

    /// Handle incoming ARP packet with validation and processing.
    fn handle_arp_packet(
        &mut self,
        arp: &packet_parser::ArpRx,
        eth_packet: &RxEthPacket,
    ) -> Result<(), RecvError> {
        // For ARP requests, dst_mac is usually broadcast (ff:ff:ff:ff:ff:ff).
        if arp.operation == ArpOp::Request && eth_packet.dst_mac != MacAddr::BROADCAST {
            debug_warn!(
                "ARP request with non-broadcast ethernet dst_mac: expected {:?}, got {:?}",
                MacAddr::BROADCAST,
                eth_packet.dst_mac
            );
        }

        trace!(
            "ARP {}: Who has {}? Tell {} (sender_hw={:?}, target_hw={:?})",
            if arp.operation == ArpOp::Request {
                "Request"
            } else {
                "Reply"
            },
            arp.target_ip,
            arp.sender_ip,
            arp.sender_hw_addr,
            arp.target_hw_addr,
        );

        // Cross-verify Ethernet src_mac matches ARP sender_hw_addr.
        if eth_packet.src_mac != arp.sender_hw_addr {
            debug_warn!(
                "ARP packet inconsistency: ethernet src_mac ({:?}) != ARP sender_hw_addr ({:?})",
                eth_packet.src_mac,
                arp.sender_hw_addr
            );
        }

        // Verify hardware addresses.
        match arp.operation {
            ArpOp::Request => {
                // For ARP requests:
                // - sender_hw_addr should be the guest's MAC address.
                // - target_hw_addr should be zeroes (00:00:00:00:00:00) since it's unknown.
                if arp.sender_hw_addr != self.guest_mac {
                    debug_warn!(
                        "ARP request with invalid sender_hw_addr: expected {:?}, got {:?}",
                        self.guest_mac,
                        arp.sender_hw_addr
                    );
                }

                if arp.target_hw_addr != MacAddr::ZERO {
                    debug_warn!(
                        "ARP request with non-zero target_hw_addr: expected {:?}, got {:?}",
                        MacAddr::ZERO,
                        arp.target_hw_addr
                    );
                }

                self.handle_arp_request(arp.target_ip, arp.sender_ip)?;
            }
            ArpOp::Reply => {
                debug_warn!("ARP reply received - not expected in this implementation");
            }
        }

        Ok(())
    }

    /// Handle ARP request - only allow resolving host IP's MAC address.
    fn handle_arp_request(
        &mut self,
        target_ip: Ipv4Addr,
        sender_ip: Ipv4Addr,
    ) -> Result<(), RecvError> {
        if target_ip != self.host_ip {
            debug_warn!(
                "ARP request for non-host IP: {} (host IP: {})",
                target_ip,
                self.host_ip
            );
            return Err(RecvError::InvalidArpTarget);
        }

        if sender_ip != self.guest_ip {
            debug_warn!(
                "ARP request for non-guest IP: {} (guest IP: {})",
                sender_ip,
                self.guest_ip
            );
            return Err(RecvError::InvalidArpTarget);
        }

        trace!(
            "ARP request for host IP {} from {} - sending ARP reply with MAC {}",
            target_ip, sender_ip, self.host_mac
        );

        self.pending_arp_reply = true;
        Ok(())
    }
}
