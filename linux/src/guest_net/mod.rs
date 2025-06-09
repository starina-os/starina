use starina::collections::HashMap;
use starina::prelude::*;
use thiserror::Error;

use crate::guest_memory;

mod packet;
mod packet_builder;
mod packet_parser;

use packet::ArpOp;
pub use packet::IpProto;
pub use packet::Ipv4Addr;
pub use packet::MacAddr;
use packet_builder::PacketBuilder;
use packet_builder::TxPacket;
use packet_parser::PacketParser;
use packet_parser::RxPacket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnKey {
    pub proto: IpProto,
    pub host_port: u16,
    pub guest_port: u16,
}

pub trait PacketReader {
    fn read_bytes(&mut self, read_len: usize) -> Result<&[u8], guest_memory::Error>;
}

pub trait PacketWriter {
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), guest_memory::Error>;
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error("Guest memory error: {0}")]
    GuestMemory(#[from] guest_memory::Error),

    #[error("Unknown connection")]
    UnknownConn,
}

#[derive(Debug, Error)]
pub enum RecvError {
    #[error("Guest memory error: {0}")]
    GuestMemory(#[from] guest_memory::Error),

    #[error("Unknown connection")]
    UnknownConn,

    #[error("Invalid packet: {0}")]
    InvalidPacket(#[from] packet_parser::ParseError),
}

struct Conn {}

pub struct GuestNet {
    host_ip: Ipv4Addr,
    guest_ip: Ipv4Addr,
    guest_mac: MacAddr,
    host_mac: MacAddr,
    gw_ip: Ipv4Addr,
    netmask: Ipv4Addr,
    dns_servers: [Ipv4Addr; 2],
    connections: HashMap<ConnKey, Conn>,
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
            connections: HashMap::new(),
        }
    }

    pub fn build_linux_ip_param(&self) -> String {
        format!(
            "ip={}::{}:{}::eth0:off:{}:{}",
            self.guest_ip,
            self.gw_ip,
            self.netmask,
            self.dns_servers[0],
            self.dns_servers[1]
        )
    }

    /// Writes TCP/UDP payload to the guest.
    pub fn send_to_guest(
        &self,
        mut writer: impl PacketWriter,
        key: &ConnKey,
        data: &[u8],
    ) -> Result<(), SendError> {
        let Some(conn) = self.connections.get(key) else {
            debug_warn!("unknown network connection: {:?}", key);
            return Err(SendError::UnknownConn);
        };

        // Example: Create a dummy TCP packet for port forwarding
        let dummy_tcp_packet = TxPacket::Tcp {
            src_ip: Ipv4Addr::new(192, 168, 1, 1),   // Host IP
            dst_ip: Ipv4Addr::new(192, 168, 1, 100), // Guest IP
            src_port: key.host_port,
            dst_port: key.guest_port,
            seq_num: 0x12345678, // Dummy sequence number
            ack_num: 0x87654321, // Dummy ack number
            flags: 0x18,         // PSH + ACK flags for established connection
            window: 65535,
            payload: data,
        };

        let dst_mac = MacAddr::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x01]); // Guest MAC
        let src_mac = MacAddr::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x02]); // Host MAC
        let builder = PacketBuilder::new(writer, dst_mac, src_mac);
        builder.send(&dummy_tcp_packet).map_err(|_| {
            SendError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;

        Ok(())
    }

    pub fn recv_from_guest(&self, reader: impl PacketReader) -> Result<(), RecvError> {
        let packet = match PacketParser::parse(reader) {
            Ok(packet) => packet,
            Err(e) => {
                debug_warn!("failed to parse packet: {:?}", e);
                // Do not return error, just ignore the packet.
                return Ok(());
            }
        };

        match packet {
            RxPacket::Arp {
                operation,
                sender_ip,
                target_ip,
                ..
            } => {
                info!(
                    "ARP {}: {} -> {}",
                    if operation == ArpOp::Request {
                        "Request"
                    } else {
                        "Reply"
                    },
                    sender_ip,
                    target_ip,
                );
            }
            RxPacket::Tcp {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                flags,
                payload,
                ..
            } => {
                info!(
                    "TCP {:b}: [{}:{} -> {}:{}] {} bytes payload",
                    flags,
                    src_ip,
                    src_port,
                    dst_ip,
                    dst_port,
                    payload.len()
                );
            }
            RxPacket::Udp {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                payload,
                ..
            } => {
                info!(
                    "UDP packet: [{}:{} -> {}:{}] {} bytes payload",
                    src_ip,
                    src_port,
                    dst_ip,
                    dst_port,
                    payload.len()
                );
            }
            RxPacket::UnknownIpv4 {
                src_ip,
                dst_ip,
                ip_proto,
                payload,
                ..
            } => {
                info!(
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
                info!(
                    "Unknown ethernet packet (type 0x{:04x}): {} bytes",
                    ether_type, payload_len
                );
            }
        }
        Ok(())
    }
}
