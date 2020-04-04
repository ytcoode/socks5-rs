#![deny(warnings)]

use mio::net::TcpListener;
use mio::*;
use net2::unix::UnixTcpBuilderExt;
use net2::TcpBuilder;
use slab::Slab;
use std::env;
use std::io;
use std::io::ErrorKind;

mod buf;
mod agent;
mod server;
mod util;

const SERVER: Token = Token(0);

fn main() -> io::Result<()> {
    // Address
    let mut addr = "0.0.0.0:".to_owned();
    // Port Params
    if let Some(port) = env::args().nth(1) {
        addr.push_str(&port);
    } else {
        // if params is null
        addr.push_str("9999");
    }

    // is IPv4 ?
    let sock = TcpBuilder::new_v4()?;

    if cfg!(unix) {
        sock.reuse_address(true)?;
        sock.reuse_port(true)?;
    }

    sock.bind(addr)?;
    let listener = sock.listen(1024)?;
    let listener = TcpListener::from_std(listener)?;
    // let listener = TcpListener::bind(addr)?;

    let mut poll = Poll::new()?;
    let mut slab = Slab::new();
    let mut events = Events::with_capacity(1024);

    poll.registry()
        .register(&listener, SERVER, Interests::READABLE)?;

    loop {
        poll.poll(&mut events, None)?;
        for event in events.iter() {
            match event.token() {
                SERVER => {
                    server::accept(&listener, &mut slab, poll.registry())?;
                },
                Token(t) => {
                    let k = util::token_to_key(t);
                    if let Some(c) = slab.get_mut(k) {
                        if let Err(e) = c.handle(t, event, poll.registry()) {
                            slab.remove(k);
                            if e.kind() != ErrorKind::Other {
                                println!("ERR: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}
