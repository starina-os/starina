#![no_std]
#![no_main]

mod virtio_net;

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::environ::Device;
use ftl_autogen::idl::ethernet_device::Rx;
use ftl_autogen::idl::Message;
use virtio_net::VirtioNet;

#[derive(Debug)]
enum Context {
    Startup,
    Interrupt,
    Tcpip,
}

/// Parses `virtio_mmio.device=512@0xfeb00000:5` into `(0xfeb00000, 5)`.
fn parse_boot_args(boot_args: &str) -> Option<(u64 /* paddr */, u32 /* irq */)> {
    for param in boot_args.split_whitespace() {
        let mut parts = param.split('=');
        let key = parts.next()?;
        let value = parts.next()?;

        if key == "virtio_mmio.device" {
            let mut parts = value.split("@0x");
            parts.next()?;
            let reg_and_irq = parts.next()?;

            let mut parts = reg_and_irq.split(":");
            let paddr_str = parts.next()?;
            let irq_str = parts.next()?;

            let paddr = u64::from_str_radix(paddr_str, 16).ok()?;
            let irq = irq_str.parse::<u32>().ok()?;
            return Some((paddr, irq));
        }
    }

    None
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("starting");
    let startup_ch = env.take_channel("dep:startup").unwrap();

    let mut virtio_net = match env.devices("virtio,mmio") {
        Some(devices) if !devices.is_empty() => VirtioNet::new(devices),
        _ => {
            let boot_args = env.string("boot_args").unwrap();
            let (reg, irq) = parse_boot_args(boot_args).unwrap();

            let devices = &[Device {
                name: "virtio,mmio".to_string(),
                compatible: "virtio,mmio".to_string(),
                reg,
                interrupts: Some(vec![irq]),
            }];

            VirtioNet::new(devices)
        }
    };

    trace!("device init");

    trace!("device init OK");
    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();
    mainloop
        .add_interrupt(virtio_net.take_interrupt().unwrap(), Context::Interrupt)
        .unwrap();

    trace!("ready");
    let mut tcpip_ch = None;
    loop {
        match mainloop.next() {
            Event::Message {
                ctx: Context::Startup,
                message: Message::NewClient(m),
                ..
            } => {
                let ch = m.handle.take::<Channel>().unwrap();
                let (sender, receiver) = ch.split();
                tcpip_ch = Some(sender.clone());

                mainloop
                    .add_channel((sender, receiver), Context::Tcpip)
                    .unwrap();
            }
            Event::Message {
                ctx: Context::Tcpip,
                message: Message::Tx(m),
                ..
            } => {
                trace!("sending {} bytes", m.payload.len());
                virtio_net.transmit(m.payload.as_slice());
            }
            Event::Interrupt {
                ctx: Context::Interrupt,
                interrupt,
            } => {
                virtio_net.handle_interrupt(|payload| {
                    trace!("received {} bytes", payload.len());

                    let Some(tcpip_ch) = tcpip_ch.as_ref() else {
                        debug_warn!("no tcpip ch, droppping packet...");
                        return;
                    };

                    let rx = Rx {
                        payload: payload.try_into().unwrap(),
                    };

                    if let Err(err) = tcpip_ch.send(rx) {
                        warn!("failed to forward RX packet, dropping: {:?}", err);
                    }
                });

                interrupt.acknowledge().unwrap();
            }
            ev => {
                warn!("unhandled event: {:?}", ev);
            }
        }
    }
}
