#![no_std]
#![no_main]

mod virtio_net;

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_autogen::idl::ethernet_device::Rx;
use ftl_autogen::idl::Message;
use virtio_net::VirtioNet;

#[derive(Debug)]
enum Context {
    Startup,
    Interrupt,
    Tcpip,
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("starting");
    let startup_ch = env.take_channel("dep:startup").unwrap();

    let devices = env.devices("virtio,mmio").unwrap();
    let mut virtio_net = VirtioNet::new(devices);

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();
    mainloop
        .add_interrupt(virtio_net.take_interrupt().unwrap(), Context::Interrupt)
        .unwrap();

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
