use core::fmt::Display;

/// MAC address wrapper with convenient utilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    /// Broadcast MAC address (FF:FF:FF:FF:FF:FF)
    pub const BROADCAST: MacAddr = MacAddr([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

    /// Zero MAC address (00:00:00:00:00:00)
    pub const ZERO: MacAddr = MacAddr([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    /// Create a new MAC address from bytes
    pub const fn new(bytes: [u8; 6]) -> Self {
        MacAddr(bytes)
    }
}

impl Display for MacAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddr {
    fn from(bytes: [u8; 6]) -> Self {
        MacAddr(bytes)
    }
}

impl From<MacAddr> for [u8; 6] {
    fn from(mac: MacAddr) -> Self {
        mac.0
    }
}

/// IPv4 address wrapper with convenient utilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ipv4Addr([u8; 4]);

impl Ipv4Addr {
    /// Localhost address (127.0.0.1)
    pub const LOCALHOST: Ipv4Addr = Ipv4Addr([127, 0, 0, 1]);

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Ipv4Addr = Ipv4Addr([255, 255, 255, 255]);

    /// Unspecified address (0.0.0.0)
    pub const UNSPECIFIED: Ipv4Addr = Ipv4Addr([0, 0, 0, 0]);

    /// Create a new IPv4 address from bytes
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4Addr([a, b, c, d])
    }
}

impl Display for Ipv4Addr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl From<[u8; 4]> for Ipv4Addr {
    fn from(bytes: [u8; 4]) -> Self {
        Ipv4Addr(bytes)
    }
}

impl From<Ipv4Addr> for [u8; 4] {
    fn from(ip: Ipv4Addr) -> Self {
        ip.0
    }
}

/// Ethernet frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtherType {
    Ipv4 = 0x0800,
    Arp = 0x0806,
}

impl EtherType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0800 => Some(EtherType::Ipv4),
            0x0806 => Some(EtherType::Arp),
            _ => None,
        }
    }
}

/// ARP operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpOp {
    Request = 1,
    Reply = 2,
}

impl ArpOp {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(ArpOp::Request),
            2 => Some(ArpOp::Reply),
            _ => None,
        }
    }
}

/// IP protocol numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpProto {
    Tcp = 6,
    Udp = 17,
}

impl IpProto {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            6 => Some(IpProto::Tcp),
            17 => Some(IpProto::Udp),
            _ => None,
        }
    }
}
