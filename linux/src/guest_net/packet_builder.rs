use core::mem;

use thiserror::Error;

use super::PacketWriter;
use super::packet::ArpOp;
use super::packet::EtherType;
use super::packet::Ipv4Addr;
use super::packet::MacAddr;
use crate::guest_memory;

/// Simplified packet for transmission
#[derive(Debug)]
pub enum TxPacket<'a> {
    Arp {
        operation: ArpOp,
        sender_hw_addr: MacAddr,
        sender_ip: Ipv4Addr,
        target_hw_addr: MacAddr,
        target_ip: Ipv4Addr,
    },
    Tcp {
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window: u16,
        payload: &'a [u8],
    },
    Udp {
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        payload: &'a [u8],
    },
}

#[repr(C)]
struct EthernetHeaderRaw {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ether_type: u16,
}

#[repr(C)]
struct ArpPacketRaw {
    hw_type: u16,
    proto_type: u16,
    hw_len: u8,
    proto_len: u8,
    operation: u16,
    sender_hw_addr: [u8; 6],
    sender_proto_addr: [u8; 4],
    target_hw_addr: [u8; 6],
    target_proto_addr: [u8; 4],
}

#[repr(C)]
struct Ipv4HeaderRaw {
    version_ihl: u8,
    dscp_ecn: u8,
    total_len: u16,
    id: u16,
    flags_frag: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
}

#[repr(C)]
struct UdpHeaderRaw {
    src_port: u16,
    dst_port: u16,
    length: u16,
    checksum: u16,
}

#[repr(C)]
struct TcpHeaderRaw {
    src_port: u16,
    dst_port: u16,
    seq_num: u32,
    ack_num: u32,
    data_offset_flags: u16,
    window: u16,
    checksum: u16,
    urgent_ptr: u16,
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Guest memory error: {0}")]
    GuestMemory(#[from] guest_memory::Error),

    #[error("Invalid packet type for building")]
    InvalidPacket,

    #[error("Invalid header: {0}")]
    InvalidHeader(&'static str),
}

/// Simple packet builder for ethernet frames
pub struct PacketBuilder<W: PacketWriter> {
    writer: W,
    dst_mac: MacAddr,
    src_mac: MacAddr,
}

impl<W: PacketWriter> PacketBuilder<W> {
    pub fn new(writer: W, dst_mac: MacAddr, src_mac: MacAddr) -> Self {
        Self {
            writer,
            dst_mac,
            src_mac,
        }
    }

    /// Build an ethernet frame and write to the writer.
    pub fn send(mut self, packet: &TxPacket) -> Result<usize, BuildError> {
        match packet {
            TxPacket::Arp {
                operation,
                sender_hw_addr,
                sender_ip,
                target_hw_addr,
                target_ip,
            } => {
                self.write_eth_header(EtherType::Arp as u16)?;
                self.write_arp_packet(
                    *operation,
                    *sender_hw_addr,
                    *sender_ip,
                    *target_hw_addr,
                    *target_ip,
                )?;
            }
            TxPacket::Tcp {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                seq_num,
                ack_num,
                flags,
                window,
                payload,
            } => {
                let total_len = 20 + 20 + payload.len() as u16; // IP header + TCP header + payload
                self.write_eth_header(EtherType::Ipv4 as u16)?;
                self.write_ipv4_header(6, total_len, (*src_ip).into(), (*dst_ip).into())?; // protocol 6 = TCP
                self.write_tcp_header(*src_port, *dst_port, *seq_num, *ack_num, *flags, *window)?;
                self.writer.write_bytes(payload)?;
            }
            TxPacket::Udp {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                payload,
            } => {
                let udp_len = 8 + payload.len() as u16; // UDP header + payload
                let total_len = 20 + udp_len; // IP header + UDP packet
                self.write_eth_header(EtherType::Ipv4 as u16)?;
                self.write_ipv4_header(17, total_len, (*src_ip).into(), (*dst_ip).into())?; // protocol 17 = UDP
                self.write_udp_header(*src_port, *dst_port, udp_len)?;
                self.writer.write_bytes(payload)?;
            }
        }
        Ok(self.writer.written_len())
    }

    fn write_eth_header(&mut self, ether_type: u16) -> Result<(), BuildError> {
        let raw = EthernetHeaderRaw {
            dst_mac: self.dst_mac.into(),
            src_mac: self.src_mac.into(),
            ether_type: ether_type.to_be(),
        };
        let bytes = unsafe { mem::transmute::<EthernetHeaderRaw, [u8; 14]>(raw) };
        self.writer.write_bytes(&bytes)?;
        Ok(())
    }

    fn write_arp_packet(
        &mut self,
        operation: ArpOp,
        sender_hw_addr: MacAddr,
        sender_ip: Ipv4Addr,
        target_hw_addr: MacAddr,
        target_ip: Ipv4Addr,
    ) -> Result<(), BuildError> {
        let raw = ArpPacketRaw {
            hw_type: 1u16.to_be(),         // Ethernet
            proto_type: 0x0800u16.to_be(), // IPv4
            hw_len: 6,
            proto_len: 4,
            operation: (operation as u16).to_be(),
            sender_hw_addr: sender_hw_addr.into(),
            sender_proto_addr: sender_ip.into(),
            target_hw_addr: target_hw_addr.into(),
            target_proto_addr: target_ip.into(),
        };
        let bytes = unsafe { mem::transmute::<ArpPacketRaw, [u8; 28]>(raw) };
        self.writer.write_bytes(&bytes)?;
        Ok(())
    }

    fn write_ipv4_header(
        &mut self,
        protocol: u8,
        total_len: u16,
        src_ip: [u8; 4],
        dst_ip: [u8; 4],
    ) -> Result<(), BuildError> {
        let raw = Ipv4HeaderRaw {
            version_ihl: (4 << 4) | 5, // version=4, header_len=20 bytes (5 * 4)
            dscp_ecn: 0,
            total_len: total_len.to_be(),
            id: 0u16.to_be(),
            flags_frag: 0u16.to_be(),
            ttl: 64,
            protocol,
            checksum: 0u16.to_be(),
            src_ip,
            dst_ip,
        };
        let bytes = unsafe { mem::transmute::<Ipv4HeaderRaw, [u8; 20]>(raw) };
        self.writer.write_bytes(&bytes)?;
        Ok(())
    }

    fn write_udp_header(
        &mut self,
        src_port: u16,
        dst_port: u16,
        length: u16,
    ) -> Result<(), BuildError> {
        let raw = UdpHeaderRaw {
            src_port: src_port.to_be(),
            dst_port: dst_port.to_be(),
            length: length.to_be(),
            checksum: 0u16.to_be(), // Checksum can be 0 for UDP
        };
        let bytes = unsafe { mem::transmute::<UdpHeaderRaw, [u8; 8]>(raw) };
        self.writer.write_bytes(&bytes)?;
        Ok(())
    }

    fn write_tcp_header(
        &mut self,
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window: u16,
    ) -> Result<(), BuildError> {
        let data_offset_flags = (5u16 << 12) | (flags as u16); // data_offset=5 (20 bytes header)
        let raw = TcpHeaderRaw {
            src_port: src_port.to_be(),
            dst_port: dst_port.to_be(),
            seq_num: seq_num.to_be(),
            ack_num: ack_num.to_be(),
            data_offset_flags: data_offset_flags.to_be(),
            window: window.to_be(),
            checksum: 0u16.to_be(), // Checksum will be calculated later if needed
            urgent_ptr: 0u16.to_be(),
        };
        let bytes = unsafe { mem::transmute::<TcpHeaderRaw, [u8; 20]>(raw) };
        self.writer.write_bytes(&bytes)?;
        Ok(())
    }
}
