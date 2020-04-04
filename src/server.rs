use crate::agent::*;
use crate::util;
use mio::net::TcpListener;
use mio::*;
use slab::Slab;
use std::io;
use std::io::ErrorKind;

pub fn accept(
    listener: &TcpListener,
    slab: &mut Slab<Agent>,
    registry: &Registry,
) -> io::Result<()> {
    loop {
        match listener.accept() {
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(e) => panic!("listener.accept: {:?}", e),
            Ok((s, _)) => {
                let e = slab.vacant_entry();
                let t = util::key_to_token(e.key());
                s.set_nodelay(true)?;
                registry.register(&s, Token(t), Interests::READABLE | Interests::WRITABLE)?;
                e.insert(Agent::new(s, t));
            }
        }
    }
}
