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
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
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
#[derive(Debug)]
struct TcpConn {
    state: TcpConnState,
    seq: u32,
    ack: u32,
    initial_seq: u32,
    window: u16,
    queued_data: Vec<Vec<u8>>,
}

impl TcpConn {
    fn new_syn_sent(initial_seq: u32) -> Self {
        Self {
            state: TcpConnState::SynSent,
            seq: initial_seq.wrapping_add(1), // SYN consumes one sequence number
            ack: 0,
            initial_seq,
            window: 65535,
            queued_data: Vec::new(),
        }
    }

    fn is_established(&self) -> bool {
        matches!(self.state, TcpConnState::Established)
    }

    fn queue_data(&mut self, data: Vec<u8>) {
        if !self.is_established() {
            self.queued_data.push(data);
        }
    }

    fn get_queued_data(&mut self) -> Vec<Vec<u8>> {
        let data = self.queued_data.clone();
        self.queued_data.clear();
        data
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
            self.guest_ip, self.gw_ip, self.netmask, self.dns_servers[0], self.dns_servers[1]
        )
    }

    /// Initiates a TCP connection to the guest by sending SYN
    pub fn connect_to_guest(
        &mut self,
        writer: impl PacketWriter,
        connkey: ConnKey,
    ) -> Result<(), SendError> {
        trace!("Connecting to guest: {:?}", connkey);
        self.connect_to_guest_with_seq(writer, connkey, 1000)
    }

    fn connect_to_guest_with_seq(
        &mut self,
        writer: impl PacketWriter,
        connkey: ConnKey,
        _initial_seq: u32,
    ) -> Result<(), SendError> {
        // Generate initial sequence number (simple approach)
        use core::sync::atomic::AtomicU32;
        use core::sync::atomic::Ordering;
        static INITIAL_SEQ: AtomicU32 = AtomicU32::new(0x12345678);
        let initial_seq = INITIAL_SEQ.fetch_add(1000, Ordering::Relaxed);
        self.connect_to_guest_internal(writer, connkey, initial_seq)
    }

    fn connect_to_guest_internal(
        &mut self,
        writer: impl PacketWriter,
        connkey: ConnKey,
        initial_seq: u32,
    ) -> Result<(), SendError> {
        // Create and store connection in SYN_SENT state
        let conn = TcpConn::new_syn_sent(initial_seq);
        self.connections.insert(connkey, conn);

        // Send SYN packet
        let syn_packet = TxPacket::Tcp {
            src_ip: connkey.remote_ip,
            dst_ip: self.guest_ip,
            src_port: connkey.remote_port,
            dst_port: connkey.guest_port,
            seq_num: initial_seq,
            ack_num: 0,
            flags: 0x02, // SYN flag
            window: 65535,
            payload: &[],
        };

        let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
        builder.send(&syn_packet).map_err(|_| {
            SendError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;
        trace!(
            "TCP SYN sent to {}:{}, initial_seq={}",
            self.guest_ip, connkey.guest_port, initial_seq
        );

        Ok(())
    }

    /// Writes TCP payload to the guest using proper connection state
    pub fn send_to_guest(
        &mut self,
        writer: impl PacketWriter,
        key: &ConnKey,
        data: &[u8],
    ) -> Result<(), SendError> {
        let Some(conn) = self.connections.get_mut(key) else {
            debug_warn!("unknown network connection: {:?}", key);
            return Err(SendError::UnknownConn);
        };

        // If connection not established, queue the data
        if !conn.is_established() {
            conn.queue_data(data.to_vec());
            trace!("Queued {} bytes for non-established connection", data.len());
            return Ok(());
        }

        // Send data with proper TCP headers for established connection
        let tcp_packet = TxPacket::Tcp {
            src_ip: key.remote_ip,
            dst_ip: self.guest_ip,
            src_port: key.remote_port,
            dst_port: key.guest_port,
            seq_num: conn.seq,
            ack_num: conn.ack,
            flags: if data.is_empty() { 0x10 } else { 0x18 }, // ACK or PSH+ACK
            window: conn.window,
            payload: data,
        };

        let builder = PacketBuilder::new(writer, self.guest_mac, self.host_mac);
        builder.send(&tcp_packet).map_err(|_| {
            SendError::GuestMemory(guest_memory::Error::Invalipaddress(
                starina::address::GPAddr::new(0),
            ))
        })?;

        // Advance sequence number by data length
        conn.advance_seq(data.len() as u32);

        Ok(())
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
        let syn = (flags & 0x02) != 0;
        let ack = (flags & 0x10) != 0;
        let fin = (flags & 0x01) != 0;
        let rst = (flags & 0x04) != 0;

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
                    // TODO: Send ACK to complete handshake
                    // TODO: Flush any queued data
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
                    // TODO: Send ACK and FIN
                    return;
                }

                // Update ACK for any data received
                if !payload.is_empty() {
                    conn.update_ack(seq_num.wrapping_add(payload.len() as u32));
                    trace!("Received {} bytes of data", payload.len());
                    // TODO: Send ACK for received data
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
                    conn.close();
                    // TODO: Send final ACK
                }
            }

            TcpConnState::Closed => {
                debug_warn!("Received packet for closed connection");
            }
        }
    }
}
