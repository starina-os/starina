use starina_api::collections::vec_deque::VecDeque;
use starina_api::prelude::vec::Vec;
use starina_api::prelude::*;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::time::Instant;

/// Represents a right to receive a RX packet.
pub struct RxTokenImpl(Vec<u8>);

impl smoltcp::phy::RxToken for RxTokenImpl {
    /// Smoltcp wants to receive a packet.
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        // Simply pass the buffer to smoltcp.
        f(&mut self.0)
    }
}

pub struct TxTokenImpl<'a>(&'a mut NetDevice);

impl<'a> smoltcp::phy::TxToken for TxTokenImpl<'a> {
    /// Smoltcp wants to transmit a packet of `len` bytes.
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        assert!(len <= self.0.tx_buf.len());

        // Let smoltcp fill a packet to transmit.
        let ret = f(&mut self.0.tx_buf[..len]);

        (self.0.transmit)(&self.0.tx_buf[..len]);
        ret
    }
}

/// A network device implementation for smoltcp.
pub struct NetDevice {
    transmit: Box<dyn Fn(&[u8])>,
    tx_buf: Vec<u8>,
    rx_queue: VecDeque<Vec<u8>>,
}

impl NetDevice {
    pub fn new(transmit: Box<dyn Fn(&[u8])>) -> NetDevice {
        NetDevice {
            transmit,
            tx_buf: vec![0; 1514],
            rx_queue: VecDeque::new(),
        }
    }

    pub fn receive_pkt(&mut self, pkt: &[u8]) {
        self.rx_queue.push_back(pkt.to_vec());
    }
}

impl smoltcp::phy::Device for NetDevice {
    type RxToken<'a> = RxTokenImpl;
    type TxToken<'a> = TxTokenImpl<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = smoltcp::phy::Medium::Ethernet;
        caps.max_transmission_unit = 1514;
        caps
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        self.rx_queue
            .pop_front()
            .map(|pkt| (RxTokenImpl(pkt), TxTokenImpl(self)))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(TxTokenImpl(self))
    }
}
