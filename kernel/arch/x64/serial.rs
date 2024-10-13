//! A serial port driver. See https://wiki.osdev.org/Serial_Ports
use core::arch::asm;

const SERIAL0_IOPORT: u16 = 0x3f8;
const THR: u16 = 0;
const DLL: u16 = 0;
const RBR: u16 = 0;
const DLH: u16 = 1;
const IER: u16 = 1;
const FCR: u16 = 2;
const LCR: u16 = 3;
const LSR: u16 = 5;
const TX_READY: u8 = 0x20;

pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nostack),
    );
}

pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nostack),
    );
    value
}

pub struct SerialPort {
    ioport_base: u16,
}

impl SerialPort {
    pub fn init(ioport_base: u16) -> SerialPort {
        let divisor: u16 = 12; // 115200 / 9600 = 12
        unsafe {
            outb(ioport_base + IER, 0x00); // Disable interrupts.
            outb(ioport_base + DLL, (divisor & 0xff) as u8);
            outb(ioport_base + DLH, ((divisor >> 8) & 0xff) as u8);
            outb(ioport_base + LCR, 0x03); // 8n1.
            outb(ioport_base + FCR, 0x01); // Enable FIFO.
            outb(ioport_base + IER, 0x01); // Enable interrupts.
        }

        SerialPort { ioport_base }
    }

    pub fn print_char(&self, ch: u8) {
        unsafe {
            while (inb(self.ioport_base + LSR) & TX_READY) == 0 {}
            outb(self.ioport_base + THR, ch);
        }
    }
}

pub static SERIAL0: spin::Lazy<SerialPort> = spin::Lazy::new(|| SerialPort::init(SERIAL0_IOPORT));
