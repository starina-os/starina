use starina::prelude::*;
use thiserror::Error;

use super::PacketReader;
use super::packet::ArpOp;
use super::packet::EtherType;
use super::packet::IpProto;
use super::packet::Ipv4Addr;
use super::packet::MacAddr;
use crate::guest_memory;

/// Parsed ethernet packet with inlined header fields
#[derive(Debug)]
pub enum RxPacket {
    Arp {
        // Ethernet header fields
        dst_mac: MacAddr,
        src_mac: MacAddr,
        // ARP packet fields (only interesting ones)
        operation: ArpOp,
        sender_hw_addr: MacAddr,
        sender_ip: Ipv4Addr,
        target_hw_addr: MacAddr,
        target_ip: Ipv4Addr,
    },
    Tcp {
        // Ethernet header fields
        dst_mac: MacAddr,
        src_mac: MacAddr,
        // IPv4 header fields (only interesting ones)
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        // TCP header fields (only interesting ones)
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window: u16,
        payload: Vec<u8>,
    },
    Udp {
        // Ethernet header fields
        dst_mac: MacAddr,
        src_mac: MacAddr,
        // IPv4 header fields (only interesting ones)
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        // UDP header fields (only interesting ones)
        src_port: u16,
        dst_port: u16,
        payload: Vec<u8>,
    },
    UnknownIpv4 {
        // Ethernet header fields
        dst_mac: MacAddr,
        src_mac: MacAddr,
        // IPv4 header fields (only interesting ones)
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        ip_proto: u8,
        payload: Vec<u8>,
    },
    UnknownEth {
        dst_mac: MacAddr,
        src_mac: MacAddr,
        ether_type: u16,
        payload_len: usize,
    },
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Guest memory error: {0}")]
    GuestMemory(#[from] guest_memory::Error),
    #[error("Invalid packet: {0}")]
    InvalidPacket(&'static str),
    #[error("Unknown ARP operation: {0}")]
    UnknownArpOperation(u16),
}

/// Simple packet parser for ethernet frames
pub struct PacketParser;

impl PacketParser {
    /// Parse an ethernet frame from the reader
    pub fn parse(mut reader: impl PacketReader) -> Result<RxPacket, ParseError> {
        // Parse ethernet header (14 bytes)
        let (dst_mac, src_mac, ether_type_raw) = Self::parse_ethernet_header(&mut reader)?;
        let ether_type = EtherType::from_u16(ether_type_raw);

        match ether_type {
            Some(EtherType::Arp) => {
                let (operation, sender_hw_addr, sender_ip, target_hw_addr, target_ip) =
                    Self::parse_arp_packet(&mut reader)?;
                Ok(RxPacket::Arp {
                    dst_mac,
                    src_mac,
                    operation,
                    sender_hw_addr,
                    sender_ip,
                    target_hw_addr,
                    target_ip,
                })
            }
            Some(EtherType::Ipv4) => {
                let (src_ip, dst_ip, ip_proto, header_len, total_len) =
                    Self::parse_ipv4_header(&mut reader)?;
                let remaining_payload_len = total_len as usize - (header_len as usize);

                match IpProto::from_u8(ip_proto) {
                    Some(IpProto::Tcp) => {
                        let (src_port, dst_port, seq_num, ack_num, flags, window, tcp_header_len) =
                            Self::parse_tcp_header(&mut reader)?;
                        let payload_len = remaining_payload_len - (tcp_header_len as usize);
                        let payload = if payload_len > 0 {
                            reader.read_bytes(payload_len)?.to_vec()
                        } else {
                            Vec::new()
                        };
                        Ok(RxPacket::Tcp {
                            dst_mac,
                            src_mac,
                            src_ip,
                            dst_ip,
                            src_port,
                            dst_port,
                            seq_num,
                            ack_num,
                            flags,
                            window,
                            payload,
                        })
                    }
                    Some(IpProto::Udp) => {
                        let (src_port, dst_port, udp_length) = Self::parse_udp_header(&mut reader)?;
                        let payload_len = udp_length.saturating_sub(8) as usize; // UDP header is 8 bytes
                        let payload = if payload_len > 0 {
                            reader.read_bytes(payload_len)?.to_vec()
                        } else {
                            Vec::new()
                        };
                        Ok(RxPacket::Udp {
                            dst_mac,
                            src_mac,
                            src_ip,
                            dst_ip,
                            src_port,
                            dst_port,
                            payload,
                        })
                    }
                    None => {
                        // Unknown IP protocol, read remaining payload
                        let payload = if remaining_payload_len > 0 {
                            reader.read_bytes(remaining_payload_len)?.to_vec()
                        } else {
                            Vec::new()
                        };
                        Ok(RxPacket::UnknownIpv4 {
                            dst_mac,
                            src_mac,
                            src_ip,
                            dst_ip,
                            ip_proto,
                            payload,
                        })
                    }
                }
            }
            None => {
                let payload = reader.read_bytes(1500)?; // Max ethernet payload
                Ok(RxPacket::UnknownEth {
                    dst_mac,
                    src_mac,
                    ether_type: ether_type_raw,
                    payload_len: payload.len(),
                })
            }
        }
    }

    fn parse_ethernet_header(
        reader: &mut impl PacketReader,
    ) -> Result<(MacAddr, MacAddr, u16), ParseError> {
        let eth_data = reader.read_bytes(14)?;

        if eth_data.len() < 14 {
            return Err(ParseError::InvalidPacket("Ethernet header too short"));
        }

        let mut dst_mac = [0u8; 6];
        let mut src_mac = [0u8; 6];

        dst_mac.copy_from_slice(&eth_data[0..6]);
        src_mac.copy_from_slice(&eth_data[6..12]);

        let ether_type_raw = u16::from_be_bytes([eth_data[12], eth_data[13]]);

        Ok((
            MacAddr::from(dst_mac),
            MacAddr::from(src_mac),
            ether_type_raw,
        ))
    }

    fn parse_arp_packet(
        reader: &mut impl PacketReader,
    ) -> Result<(ArpOp, MacAddr, Ipv4Addr, MacAddr, Ipv4Addr), ParseError> {
        let arp_data = reader.read_bytes(28)?; // ARP packet is 28 bytes

        if arp_data.len() < 28 {
            return Err(ParseError::InvalidPacket("ARP packet too short"));
        }

        let hw_type = u16::from_be_bytes([arp_data[0], arp_data[1]]);
        let proto_type = u16::from_be_bytes([arp_data[2], arp_data[3]]);
        let hw_len = arp_data[4];
        let proto_len = arp_data[5];

        let operation_raw = u16::from_be_bytes([arp_data[6], arp_data[7]]);
        let operation =
            ArpOp::from_u16(operation_raw).ok_or(ParseError::UnknownArpOperation(operation_raw))?;

        let mut sender_hw_addr = [0u8; 6];
        let mut sender_proto_addr = [0u8; 4];
        let mut target_hw_addr = [0u8; 6];
        let mut target_proto_addr = [0u8; 4];

        sender_hw_addr.copy_from_slice(&arp_data[8..14]);
        sender_proto_addr.copy_from_slice(&arp_data[14..18]);
        target_hw_addr.copy_from_slice(&arp_data[18..24]);
        target_proto_addr.copy_from_slice(&arp_data[24..28]);

        Ok((
            operation,
            MacAddr::from(sender_hw_addr),
            Ipv4Addr::from(sender_proto_addr),
            MacAddr::from(target_hw_addr),
            Ipv4Addr::from(target_proto_addr),
        ))
    }

    fn parse_ipv4_header(
        reader: &mut impl PacketReader,
    ) -> Result<(Ipv4Addr, Ipv4Addr, u8, u8, u16), ParseError> {
        let ip_data = reader.read_bytes(20)?; // Minimum IPv4 header is 20 bytes

        if ip_data.len() < 20 {
            return Err(ParseError::InvalidPacket("IPv4 header too short"));
        }

        let version_ihl = ip_data[0];
        let version = version_ihl >> 4;
        let header_len = (version_ihl & 0x0f) * 4; // IHL is in 4-byte words

        if version != 4 {
            return Err(ParseError::InvalidPacket("Not IPv4"));
        }

        let total_len = u16::from_be_bytes([ip_data[2], ip_data[3]]);
        let proto = ip_data[9];

        let mut src_ip = [0u8; 4];
        let mut dst_ip = [0u8; 4];
        src_ip.copy_from_slice(&ip_data[12..16]);
        dst_ip.copy_from_slice(&ip_data[16..20]);

        // If header is longer than 20 bytes, skip the options
        if header_len > 20 {
            let options_len = header_len as usize - 20;
            reader.read_bytes(options_len)?;
        }

        Ok((
            Ipv4Addr::from(src_ip),
            Ipv4Addr::from(dst_ip),
            proto,
            header_len,
            total_len,
        ))
    }

    fn parse_udp_header(reader: &mut impl PacketReader) -> Result<(u16, u16, u16), ParseError> {
        let udp_data = reader.read_bytes(8)?; // UDP header is 8 bytes

        if udp_data.len() < 8 {
            return Err(ParseError::InvalidPacket("UDP header too short"));
        }

        let src_port = u16::from_be_bytes([udp_data[0], udp_data[1]]);
        let dst_port = u16::from_be_bytes([udp_data[2], udp_data[3]]);
        let length = u16::from_be_bytes([udp_data[4], udp_data[5]]);
        let _checksum = u16::from_be_bytes([udp_data[6], udp_data[7]]);

        Ok((src_port, dst_port, length))
    }

    fn parse_tcp_header(
        reader: &mut impl PacketReader,
    ) -> Result<(u16, u16, u32, u32, u8, u16, u8), ParseError> {
        let tcp_data = reader.read_bytes(20)?; // Minimum TCP header is 20 bytes

        if tcp_data.len() < 20 {
            return Err(ParseError::InvalidPacket("TCP header too short"));
        }

        let src_port = u16::from_be_bytes([tcp_data[0], tcp_data[1]]);
        let dst_port = u16::from_be_bytes([tcp_data[2], tcp_data[3]]);
        let seq_num = u32::from_be_bytes([tcp_data[4], tcp_data[5], tcp_data[6], tcp_data[7]]);
        let ack_num = u32::from_be_bytes([tcp_data[8], tcp_data[9], tcp_data[10], tcp_data[11]]);

        let data_offset_flags = u16::from_be_bytes([tcp_data[12], tcp_data[13]]);
        let header_len = ((data_offset_flags >> 12) * 4) as u8; // Data offset is in 4-byte words
        let flags = (data_offset_flags & 0xff) as u8;

        let window = u16::from_be_bytes([tcp_data[14], tcp_data[15]]);
        let _checksum = u16::from_be_bytes([tcp_data[16], tcp_data[17]]);

        // If TCP header is longer than 20 bytes, skip the options
        if header_len > 20 {
            let options_len = header_len as usize - 20;
            reader.read_bytes(options_len)?;
        }

        Ok((
            src_port, dst_port, seq_num, ack_num, flags, window, header_len,
        ))
    }
}
