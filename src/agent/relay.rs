use super::shutdown;
use super::Agent;
use super::State;
use crate::util;
use mio::*;
use std::io;
use std::net::Shutdown;

pub fn relay_in(c: &mut Agent, r: &Registry, token: usize) -> io::Result<()> {
    let (s1, s2, b) = if token == c.token {
        (&mut c.s1, c.s2.as_mut().unwrap(), &mut c.b1)
    } else if let Some(s2) = &mut c.s2 {
        (s2, &mut c.s1, &mut c.b2)
    } else {
        unreachable!()
    };

    let ea = b.copy(s1, s2)?;
    if b.len() > 0 || ea {
        return Ok(()); // write EAGAIN || read EAGAIN
    }

    // s1被关闭了

    // 关闭s2的写
    s2.shutdown(Shutdown::Write)?;

    // 取消s1的epollin事件
    r.reregister(&s1, Token(token), Interests::WRITABLE)?;

    // 取消s2的epollout事件
    r.reregister(&s2, Token(util::peer_token(token)), Interests::READABLE)?;

    c.set_state(State::Shutdown);
    shutdown::shutdown(c, r)
}

pub fn relay_out(c: &mut Agent, r: &Registry, token: usize) -> io::Result<()> {
    relay_in(c, r, util::peer_token(token))
}
