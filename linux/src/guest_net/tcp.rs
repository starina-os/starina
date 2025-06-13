use starina::collections::HashMap;
use starina::collections::VecDeque;
use starina::prelude::*;

use super::ConnKey;
use super::IpProto;
use super::Ipv4Addr;
use super::PacketWriter;
use super::SendError;
use super::packet_builder::PacketBuilder;
use super::packet_builder::TxPacket;
use super::packet_parser::TcpRx;

// TCP flags.
pub const TCP_FIN: u8 = 0x01;
pub const TCP_SYN: u8 = 0x02;
pub const TCP_RST: u8 = 0x04;
pub const TCP_PSH: u8 = 0x08;
pub const TCP_ACK: u8 = 0x10;

/// TCP connection states following the standard TCP state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum TcpConnState {
    SynSent,
    Established,
    FinWait1,
    FinWait2,
    Closed,
}

/// TCP connection with state management and data queuing.
pub struct TcpConn {
    state: TcpConnState,
    seq: u32,
    ack: u32,
    window: u16,
    queued_data: VecDeque<Vec<u8>>,
    pending_flags: u8,
    forwarder: Box<dyn FnMut(&ConnKey, &[u8])>,
}

impl TcpConn {
    pub fn new_syn_sent(initial_seq: u32, forwarder: Box<dyn FnMut(&ConnKey, &[u8])>) -> Self {
        Self {
            state: TcpConnState::SynSent,
            seq: initial_seq.wrapping_add(1), // SYN consumes one sequence number
            ack: 0,
            window: 65535,
            queued_data: VecDeque::new(),
            pending_flags: 0,
            forwarder,
        }
    }

    pub fn is_established(&self) -> bool {
        matches!(self.state, TcpConnState::Established)
    }

    pub fn queue_data(&mut self, data: Vec<u8>) {
        if !self.is_established() {
            self.queued_data.push_back(data);
        }
    }

    pub fn advance_seq(&mut self, bytes: u32) {
        self.seq = self.seq.wrapping_add(bytes);
    }

    pub fn update_ack(&mut self, ack: u32) {
        self.ack = ack;
    }

    pub fn set_established(&mut self, ack: u32) {
        self.state = TcpConnState::Established;
        self.ack = ack;
    }

    pub fn close(&mut self) {
        self.state = TcpConnState::Closed;
    }

    pub fn set_pending_flags(&mut self, flags: u8) {
        self.pending_flags |= flags;
    }

    pub fn has_pending_replies(&self) -> bool {
        self.pending_flags != 0
    }
}

pub struct TcpManager {
    connections: HashMap<ConnKey, TcpConn>,
    guest_ip: Ipv4Addr,
    guest_mac: super::MacAddr,
    host_mac: super::MacAddr,
}

impl TcpManager {
    pub fn new(guest_ip: Ipv4Addr, guest_mac: super::MacAddr, host_mac: super::MacAddr) -> Self {
        Self {
            connections: HashMap::new(),
            guest_ip,
            guest_mac,
            host_mac,
        }
    }

    /// Initiates a TCP connection to the guest by setting SYN flag to be sent later.
    pub fn connect_to_guest(
        &mut self,
        connkey: ConnKey,
        forwarder: Box<dyn FnMut(&ConnKey, &[u8])>,
    ) {
        trace!("Connecting to guest: {:?}", connkey);

        // Generate initial sequence number.
        let initial_seq = 1;

        // Create and store connection in SYN_SENT state with pending SYN flag.
        let mut conn = TcpConn::new_syn_sent(initial_seq, forwarder);
        conn.set_pending_flags(TCP_SYN);
        self.connections.insert(connkey, conn);

        trace!(
            "TCP SYN queued for {}:{}, initial_seq={}",
            self.guest_ip, connkey.guest_port, initial_seq
        );
    }

    /// Writes TCP payload to the guest using proper connection state.
    pub fn send_to_guest(
        &mut self,
        writer: impl PacketWriter,
        key: &ConnKey,
        data: &[u8],
    ) -> Result<usize /* packet len */, SendError> {
        let Some(conn) = self.connections.get_mut(key) else {
            debug_warn!("unknown network connection: {:?}", key);
            return Err(SendError::UnknownConn);
        };

        // If connection not established, queue the data and try to send pending packets first.
        if !conn.is_established() {
            conn.queue_data(data.to_vec());
            trace!("Queued {} bytes for non-established connection", data.len());

            // If there are pending flags (like SYN), send them first
            if conn.has_pending_replies() {
                return self.send_pending_packet(writer);
            }

            return Err(SendError::NoSendingPackets);
        }

        // Send data with proper TCP headers for established connection.
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
        let written_len = builder.send(&tcp_packet)?;

        // Advance sequence number by data length.
        conn.advance_seq(data.len() as u32);

        Ok(written_len)
    }

    /// Check if a connection exists.
    pub fn has_connection(&self, key: &ConnKey) -> bool {
        self.connections.contains_key(key)
    }

    pub fn has_pending_packets(&self) -> bool {
        self.connections.values().any(|conn| {
            (!conn.queued_data.is_empty() && conn.is_established()) || conn.has_pending_replies()
        })
    }

    pub fn send_pending_packet(
        &mut self,
        writer: impl PacketWriter,
    ) -> Result<usize /* packet len */, SendError> {
        // First, send any pending replies.
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

        // Then send queued data for established connections.
        for (key, conn) in self.connections.iter_mut() {
            if !conn.queued_data.is_empty() {
                let data = conn.queued_data.pop_front().unwrap();
                let key = *key;
                return self.send_to_guest(writer, &key, &data);
            }
        }

        Err(SendError::NoSendingPackets)
    }

    fn send_tcp_reply(
        &mut self,
        writer: impl PacketWriter,
        key: &ConnKey,
        flags: u8,
        seq: u32,
        ack: u32,
    ) -> Result<usize /* packet len */, SendError> {
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
        let written_len = builder.send(&tcp_packet)?;

        Ok(written_len)
    }

    /// Handle incoming TCP packet and manage connection state.
    pub fn handle_tcp_packet(&mut self, tcp: &TcpRx) {
        // Find the connection by reversing the address mapping.
        // (guest->host in packet becomes host->guest in our ConnKey).
        let conn_key = ConnKey {
            proto: IpProto::Tcp,
            remote_ip: tcp.dst_ip, // Destination in packet is our "remote" IP
            remote_port: tcp.dst_port,
            guest_port: tcp.src_port, // Source in packet is the guest port
        };

        let Some(conn) = self.connections.get_mut(&conn_key) else {
            debug_warn!(
                "TCP packet for unknown connection: {:?}, available: {:?}",
                conn_key,
                self.connections.keys().collect::<Vec<_>>()
            );
            return;
        };

        // Extract TCP flags.
        let syn = (tcp.flags & TCP_SYN) != 0;
        let ack = (tcp.flags & TCP_ACK) != 0;
        let fin = (tcp.flags & TCP_FIN) != 0;
        let rst = (tcp.flags & TCP_RST) != 0;

        match conn.state {
            TcpConnState::SynSent => {
                if rst {
                    trace!("Connection reset by guest");
                    conn.close();
                    return;
                }

                if syn && ack {
                    trace!("SYN-ACK received, connection established");
                    conn.set_established(tcp.seq_num.wrapping_add(1)); // ACK the SYN
                    // Queue ACK to complete handshake.
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
                    conn.update_ack(tcp.seq_num.wrapping_add(1)); // ACK the FIN
                    // Queue ACK and FIN.
                    conn.set_pending_flags(TCP_FIN | TCP_ACK);
                    return;
                }

                // Update ACK for any data received.
                if !tcp.payload.is_empty() {
                    conn.update_ack(tcp.seq_num.wrapping_add(tcp.payload.len() as u32));
                    (conn.forwarder)(&conn_key, &tcp.payload);

                    // Queue ACK for received data.
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
                    conn.update_ack(tcp.seq_num.wrapping_add(1)); // ACK the FIN
                    // Queue final ACK.
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
