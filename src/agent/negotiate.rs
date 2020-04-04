use super::relay;
use super::Agent;
use super::State;
use crate::util;
use mio::net::TcpStream;
use mio::*;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::str;

const VERSION: u8 = 0x05;

pub fn select_method_req(c: &mut Agent, r: &Registry) -> io::Result<()> {
    // b is Buf.
    let b = &mut c.b1;
    // read bytes to b from TcpStream.
    // ea is Successful ?
    let ea = b.read(&mut c.s1)?;

    // bytes length.
    let len = b.len();
    // is bytes is Zero.
    if len == 0 && !ea {
        return Err(Error::new(ErrorKind::UnexpectedEof, "select_method_req"));
    }

    /*

    +----+----------+----------+
    |VER | NMETHODS | METHODS  |
    +----+----------+----------+
    | 1  |    1     | 1 to 255 |
    +----+----------+----------+

    */

    // Currently Connention Version.
    let current_version =  &b[0];
    // Currently Connention NMETHODS.
    let current_nmethods = &b[1];

   // if length less than 2， then Bytes only Version, NMETHODS is empty.
   // 如果长度小于2,那就是只有版本号，但是没有NMETHODS, 那是不行的。
    if len < 2 {
        return Ok(());
    }

    // b[0] must is Version.
    if current_version != &VERSION {
        return Err(Error::new(ErrorKind::InvalidData, "version"));
    }

    // n is NMETHODS + 2.
    let n = 2 + *current_nmethods as usize;

    if len < n {
        return Ok(()); // 数据还不够，等待epollin
    }

    b.skip(n); // TODO select a valid method

    /*

    +----+--------+
    |VER | METHOD |
    +----+--------+
    | 1  |   1    |
    +----+--------+

     */

    c.b2.write_u8(VERSION);
    c.b2.write_u8(0);

    // Set Status to "Method Reply" 
    c.set_state(State::SelectMethodReply);
    select_method_reply(c, r)
}

pub fn select_method_reply(c: &mut Agent, r: &Registry) -> io::Result<()> {
    // 将 Client TCPStream 的数据写入Client的buf2
    c.b2.write(&mut c.s1)?;
    if c.b2.len() > 0 {
        return Ok(()); // 还有剩余字节没写完，等待epollout
    }

    // 所有数据写完，进入下一状态
    c.set_state(State::ConnectReq);
    connect_req(c, r)
}

pub fn connect_req(c: &mut Agent, r: &Registry) -> io::Result<()> {
    let b = &mut c.b1;
    let ea = b.read(&mut c.s1)?;

    let len = b.len();
    if len == 0 && !ea {
        return Err(Error::new(ErrorKind::UnexpectedEof, "connect_req"));
    }

    /*

    +----+-----+-------+------+----------+----------+
    |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
    +----+-----+-------+------+----------+----------+
    | 1  |  1  | X'00' |  1   | Variable |    2     |
    +----+-----+-------+------+----------+----------+

     */

    if len < 4 {
        return Ok(());
    }

    if b[0] != VERSION {
        return Err(Error::new(ErrorKind::InvalidData, "version"));
    }

    if b[1] != 1 {
        return Err(Error::new(ErrorKind::InvalidData, "CMD must be CONNECT"));
    }

    if b[2] != 0 {
        return Err(Error::new(ErrorKind::InvalidData, "RSV must be 0"));
    }

    let addr: SocketAddr;

    match b[3] {
        1 => {
            if len < 4 + 4 + 2 {
                return Ok(());
            }
            b.skip(4);
            let mut ip = [0; 4];
            b.read_exact(&mut ip);
            let port = b.read_u16();
            addr = (ip, port).into();
        }
        4 => {
            if len < 4 + 16 + 2 {
                return Ok(());
            }
            b.skip(4);
            let mut ip = [0; 16];
            b.read_exact(&mut ip);
            let port = b.read_u16();
            addr = (ip, port).into();
        }
        3 => {
            if len < 5 {
                return Ok(());
            }
            let n = b[4] as usize;
            if len < 5 + n + 2 {
                return Ok(());
            }
            b.skip(5);

            let mut dn = vec![0; n];
            b.read_exact(&mut dn[..]);

            match str::from_utf8(&dn[..]) {
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, "domain name")),
                Ok(s) => {
                    let port = b.read_u16();
                    let mut iter = (s, port).to_socket_addrs()?; // TODO 异步解析dns
                    match iter.next() {
                        None => return Err(Error::new(ErrorKind::NotFound, "domain name")),
                        Some(a) => addr = a,
                    }
                }
            }
        }
        _ => return Err(Error::new(ErrorKind::InvalidData, "ATYP")),
    }

    let s = TcpStream::connect(addr)?;
    s.set_nodelay(true)?;

    r.register(
        &s,
        Token(util::peer_token(c.token)),
        Interests::READABLE | Interests::WRITABLE,
    )?;

    c.s2 = Some(s);

    /*

    +----+-----+-------+------+----------+----------+
    |VER | REP |  RSV  | ATYP | BND.ADDR | BND.PORT |
    +----+-----+-------+------+----------+----------+
    | 1  |  1  | X'00' |  1   | Variable |    2     |
    +----+-----+-------+------+----------+----------+

     */

    assert_eq!(c.b2.len(), 0);

    c.b2.write_u8(VERSION);
    c.b2.write_u8(0);
    c.b2.write_u8(0);
    c.b2.write_u8(1); // ipv4

    // ip
    c.b2.write_u8(0);
    c.b2.write_u8(0);
    c.b2.write_u8(0);
    c.b2.write_u8(0);

    // port
    c.b2.write_u8(0);
    c.b2.write_u8(0);

    c.set_state(State::ConnectReply);
    connect_reply(c, r)
}

pub fn connect_reply(c: &mut Agent, r: &Registry) -> io::Result<()> {
    c.b2.write(&mut c.s1)?;
    if c.b2.len() > 0 {
        return Ok(()); // 还有剩余字节没写完，等待epollout
    }

    // 所有数据写完，进入下一状态
    c.set_state(State::Relay);
    relay::relay_in(c, r, c.token)
}
