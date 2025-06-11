use starina::collections::HashMap;
use starina::collections::VecDeque;
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

// TCP flags
const TCP_FIN: u8 = 0x01;
const TCP_SYN: u8 = 0x02;
const TCP_RST: u8 = 0x04;
const TCP_PSH: u8 = 0x08;
const TCP_ACK: u8 = 0x10;

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

/// TCP connection states following the standard TCP state machine
#[derive(Debug, Clone, PartialEq)]
pub enum TcpConnState {
    SynSent,
    Established,
    FinWait1,
    FinWait2,
    Closed,
}

/// TCP connection with state management and data queuing
struct TcpConn {
    state: TcpConnState,
    seq: u32,
    ack: u32,
    initial_seq: u32,
    window: u16,
    queued_data: VecDeque<Vec<u8>>,
    pending_flags: u8,
    forwarder: Box<dyn FnMut(&ConnKey, &[u8])>,
}

impl TcpConn {
    fn new_syn_sent(initial_seq: u32, forwarder: Box<dyn FnMut(&ConnKey, &[u8])>) -> Self {
        Self {
            state: TcpConnState::SynSent,
            seq: initial_seq.wrapping_add(1), // SYN consumes one sequence number
            ack: 0,
            initial_seq,
            window: 65535,
            queued_data: VecDeque::new(),
            pending_flags: 0,
            forwarder,
        }
    }

    fn is_established(&self) -> bool {
        matches!(self.state, TcpConnState::Established)
    }

    fn queue_data(&mut self, data: Vec<u8>) {
        if !self.is_established() {
            self.queued_data.push_back(data);
        }
    }

    fn advance_seq(&mut self, bytes: u32) {
        self.seq = self.seq.wrapping_add(bytes);
    }

    fn update_ack(&mut self, ack: u32) {
        self.ack = ack;
    }

    fn set_established(&mut self, ack: u32) {
        self.state = TcpConnState::Established;
        self.ack = ack;
    }

    fn close(&mut self) {
        self.state = TcpConnState::Closed;
    }

    fn set_pending_flags(&mut self, flags: u8) {
        self.pending_flags |= flags;
    }

    fn has_pending_replies(&self) -> bool {
        self.pending_flags != 0
    }
}

pub struct GuestNet {
    host_ip: Ipv4Addr,
    guest_ip: Ipv4Addr,
    guest_mac: MacAddr,
    host_mac: MacAddr,
    gw_ip: Ipv4Addr,
    netmask: Ipv4Addr,
    dns_servers: [Ipv4Addr; 2],
    connections: HashMap<ConnKey, TcpConn>,
    needs_reply_host_arp_request: bool,
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
            needs_reply_host_arp_request: false,
        }
    }

    pub fn build_linux_ip_param(&self) -> String {
        format!(
            "ip={}::{}:{}::eth0:off:{}:{}",
            self.guest_ip, self.gw_ip, self.netmask, self.dns_servers[0], self.dns_servers[1]
        )
    }

    /// Initiates a TCP connection to the guest by sending SYN
    pub fn connect_to_guest(
        &mut self,
        writer: impl PacketWriter,
        connkey: ConnKey,
        forwarder: Box<dyn FnMut(&ConnKey, &[u8])>,
    ) -> Result<Option<usize /* packet len */>, SendError> {
        trace!("Connecting to guest: {:?}", connkey);

        // Generate initial sequence number (simple approach)
        use core::sync::atomic::AtomicU32;
        use core::sync::atomic::Ordering;
        static INITIAL_SEQ: AtomicU32 = AtomicU32::new(0x12345678);
        let initial_seq = INITIAL_SEQ.fetch_add(1000, Ordering::Relaxed);

        // Create and store connection in SYN_SENT state
        let conn = TcpConn::new_syn_sent(initial_seq, forwarder);
        self.connections.insert(connkey, conn);

        // Send SYN packet
        let syn_packet = TxPacket::Tcp {
            src_ip: connkey.remote_ip,
            dst_ip: self.guest_ip,
            src_port: connkey.remote_port,
            dst_port: connkey.guest_port,
            seq_num: initial_seq,
            ack_num: 0,
            flags: TCP_SYN,
            window: 65535,
            payload: &[],
        };

        let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
        let written_len = builder.send(&syn_packet).map_err(|_| {
            SendError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;
        trace!(
            "TCP SYN sent to {}:{}, initial_seq={}",
            self.guest_ip, connkey.guest_port, initial_seq
        );

        Ok(Some(written_len))
    }

    /// Writes TCP payload to the guest using proper connection state
    pub fn send_to_guest(
        &mut self,
        writer: impl PacketWriter,
        key: &ConnKey,
        data: &[u8],
    ) -> Result<Option<usize /* packet len */>, SendError> {
        let Some(conn) = self.connections.get_mut(key) else {
            debug_warn!("unknown network connection: {:?}", key);
            return Err(SendError::UnknownConn);
        };

        // If connection not established, queue the data
        if !conn.is_established() {
            conn.queue_data(data.to_vec());
            trace!("Queued {} bytes for non-established connection", data.len());
            return Ok(None);
        }

        // Send data with proper TCP headers for established connection
        let tcp_packet = TxPacket::Tcp {
            src_ip: key.remote_ip,
            dst_ip: self.guest_ip,
            src_port: key.remote_port,
            dst_port: key.guest_port,
            seq_num: conn.seq,
            ack_num: conn.ack,
            flags: if data.is_empty() {
                TCP_ACK
            } else {
                TCP_PSH | TCP_ACK
            },
            window: conn.window,
            payload: data,
        };

        let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
        let written_len = builder.send(&tcp_packet).map_err(|_| {
            SendError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;

        // Advance sequence number by data length
        conn.advance_seq(data.len() as u32);

        Ok(Some(written_len))
    }

    /// Check if a connection exists
    pub fn has_connection(&self, key: &ConnKey) -> bool {
        self.connections.contains_key(key)
    }

    pub fn recv_from_guest(&mut self, reader: impl PacketReader) -> Result<(), RecvError> {
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
                trace!(
                    "ARP {}: Who has {}? Tell {}",
                    if operation == ArpOp::Request {
                        "Request"
                    } else {
                        "Reply"
                    },
                    target_ip,
                    sender_ip,
                );

                match operation {
                    ArpOp::Request => self.handle_arp_request(target_ip, sender_ip)?,
                    ArpOp::Reply => panic!("ARP reply received - not expected"),
                }
            }
            RxPacket::Tcp {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                seq_num,
                ack_num,
                flags,
                payload,
                ..
            } => {
                trace!(
                    "TCP {:08b}: [{}:{} -> {}:{}] seq={} ack={} {} bytes",
                    flags,
                    src_ip,
                    src_port,
                    dst_ip,
                    dst_port,
                    seq_num,
                    ack_num,
                    payload.len()
                );

                // Handle TCP packet and update connection state
                self.handle_tcp_packet(
                    src_ip, dst_ip, src_port, dst_port, seq_num, ack_num, flags, &payload,
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
                trace!(
                    "UDP packet: [{}:{} -> {}:{}] {} bytes payload",
                    src_ip,
                    src_port,
                    dst_ip,
                    dst_port,
                    payload.len(),
                );
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

    /// Handle ARP request - only allow resolving host IP's MAC address
    fn handle_arp_request(
        &mut self,
        target_ip: Ipv4Addr,
        sender_ip: Ipv4Addr,
    ) -> Result<(), RecvError> {
        if target_ip != self.host_ip {
            panic!(
                "ARP request for non-host IP: {} (host IP: {})",
                target_ip, self.host_ip
            );
        }

        if sender_ip != self.guest_ip {
            panic!(
                "ARP request for non-guest IP: {} (guest IP: {})",
                sender_ip, self.guest_ip
            );
        }

        trace!(
            "ARP request for host IP {} from {} - sending ARP reply with MAC {}",
            target_ip, sender_ip, self.host_mac
        );

        self.needs_reply_host_arp_request = true;
        Ok(())
    }

    pub fn needs_reply_host_arp_request(&self) -> bool {
        self.needs_reply_host_arp_request
    }

    pub fn reply_host_arp_request(
        &mut self,
        writer: impl PacketWriter,
    ) -> Result<usize /* packet len */, RecvError> {
        let arp_reply = TxPacket::Arp {
            operation: ArpOp::Reply,
            sender_hw_addr: self.host_mac,
            sender_ip: self.host_ip,
            target_hw_addr: self.guest_mac,
            target_ip: self.guest_ip,
        };

        let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
        let written_len = builder.send(&arp_reply).map_err(|_| {
            RecvError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;

        self.needs_reply_host_arp_request = false;
        Ok(written_len)
    }

    pub fn has_pending_queued_tcp_packets(&self) -> bool {
        self.connections.values().any(|conn| {
            (!conn.queued_data.is_empty() && conn.is_established()) || conn.has_pending_replies()
        })
    }

    pub fn send_queued_tcp_packets(
        &mut self,
        writer: impl PacketWriter,
    ) -> Result<Option<usize /* packet len */>, SendError> {
        // First, send any pending replies
        for (key, conn) in self.connections.iter_mut() {
            if conn.pending_flags != 0 {
                let key = *key;
                let seq = conn.seq;
                let ack = conn.ack;
                let flags = conn.pending_flags;
                conn.pending_flags = 0;
                return self.send_tcp_reply(writer, &key, flags, seq, ack);
            }
        }

        // Then send queued data for established connections
        for (key, conn) in self.connections.iter_mut() {
            if !conn.queued_data.is_empty() {
                let data = conn.queued_data.pop_front().unwrap();
                let key = *key;
                return self.send_to_guest(writer, &key, &data);
            }
        }

        Ok(None)
    }

    fn send_tcp_reply(
        &mut self,
        writer: impl PacketWriter,
        key: &ConnKey,
        flags: u8,
        seq: u32,
        ack: u32,
    ) -> Result<Option<usize /* packet len */>, SendError> {
        let tcp_packet = TxPacket::Tcp {
            src_ip: key.remote_ip,
            dst_ip: self.guest_ip,
            src_port: key.remote_port,
            dst_port: key.guest_port,
            seq_num: seq,
            ack_num: ack,
            flags,
            window: 65535,
            payload: &[],
        };

        let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
        let written_len = builder.send(&tcp_packet).map_err(|_| {
            SendError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;

        Ok(Some(written_len))
    }

    /// Handle incoming TCP packet and manage connection state
    fn handle_tcp_packet(
        &mut self,
        _src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        _ack_num: u32,
        flags: u8,
        payload: &[u8],
    ) {
        // Find the connection by reversing the address mapping
        // (guest->host in packet becomes host->guest in our ConnKey)
        let conn_key = ConnKey {
            proto: IpProto::Tcp,
            remote_ip: dst_ip, // Destination in packet is our "remote" IP
            remote_port: dst_port,
            guest_port: src_port, // Source in packet is the guest port
        };

        let Some(conn) = self.connections.get_mut(&conn_key) else {
            debug_warn!(
                "TCP packet for unknown connection: {:?}, available: {:?}",
                conn_key,
                self.connections.keys().collect::<Vec<_>>()
            );
            return;
        };

        // Extract TCP flags
        let syn = (flags & TCP_SYN) != 0;
        let ack = (flags & TCP_ACK) != 0;
        let fin = (flags & TCP_FIN) != 0;
        let rst = (flags & TCP_RST) != 0;

        match conn.state {
            TcpConnState::SynSent => {
                if rst {
                    trace!("Connection reset by guest");
                    conn.close();
                    return;
                }

                if syn && ack {
                    trace!("SYN-ACK received, connection established");
                    conn.set_established(seq_num.wrapping_add(1)); // ACK the SYN
                    // Queue ACK to complete handshake
                    conn.set_pending_flags(TCP_ACK);
                }
            }

            TcpConnState::Established => {
                if rst {
                    trace!("Connection reset by guest");
                    conn.close();
                    return;
                }

                if fin {
                    trace!("FIN received, guest closing connection");
                    conn.state = TcpConnState::FinWait1;
                    conn.update_ack(seq_num.wrapping_add(1)); // ACK the FIN
                    // Queue ACK and FIN
                    conn.set_pending_flags(TCP_FIN | TCP_ACK);
                    return;
                }

                // Update ACK for any data received
                if !payload.is_empty() {
                    conn.update_ack(seq_num.wrapping_add(payload.len() as u32));

                    trace!(
                        "Received {} bytes of data: {:?}",
                        payload.len(),
                        core::str::from_utf8(payload).unwrap_or(&format!("{:02x?}", payload))
                    );

                    (conn.forwarder)(&conn_key, payload);

                    // Queue ACK for received data
                    conn.set_pending_flags(TCP_ACK);
                }
            }

            TcpConnState::FinWait1 => {
                if ack {
                    trace!("ACK for FIN received, moving to FinWait2");
                    conn.state = TcpConnState::FinWait2;
                }
            }

            TcpConnState::FinWait2 => {
                if fin {
                    trace!("Final FIN received, connection closed");
                    conn.update_ack(seq_num.wrapping_add(1)); // ACK the FIN
                    // Queue final ACK
                    conn.set_pending_flags(TCP_ACK);
                    conn.close();
                }
            }

            TcpConnState::Closed => {
                debug_warn!("Received packet for closed connection");
            }
        }
    }
}
