use crate::buf::Buf;
use crate::util;
use mio::event::Event;
use mio::net::TcpStream;
use mio::*;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use State::*;

mod negotiate;
mod relay;
mod shutdown;

#[derive(Debug)]
pub enum State {
    // 选择方法请求
    SelectMethodReq,
    // 选择方法回复
    SelectMethodReply,
    // 连接请求
    ConnectReq,
    // 连接返回
    ConnectReply,
    // 代理中
    Relay,

    //关闭
    Shutdown,
}

pub struct Client {
    // This Should be Client Connection.
    s1: TcpStream,
    // This Should be Client Buffer.
    b1: Buf,

    // This Should be Server Connection.
    s2: Option<TcpStream>, // 如果为None表示还没有建立到target的连接
    // This Should be Server Buffer.
    b2: Buf,

    // ???
    token: usize,

    // Proxy Stage.
    state: State,
}

impl Client {
    pub fn new(s1: TcpStream, token: usize) -> Self {
        Client {
            s1,
            b1: Buf::new(),

            s2: None,
            b2: Buf::new(),

            token,
            state: SelectMethodReq,
        }
    }

    fn set_state(&mut self, state: State) {
        //        println!("state: {:?} -> {:?}", self.state, state);
        self.state = state;
    }

    pub fn handle(&mut self, t: usize, e: &Event, r: &Registry) -> io::Result<()> {
        assert!(t == self.token || t == util::peer_token(self.token));

        // we can read from socket.
        if e.is_readable() {
            match self.state {
                SelectMethodReq => negotiate::select_method_req(self, r)?,
                SelectMethodReply => (),
                ConnectReq => negotiate::connect_req(self, r)?,
                ConnectReply => (),
                Relay => relay::relay_in(self, r, t)?,
                Shutdown => shutdown::shutdown(self, r)?,
            }
        }

        // we can write to from socket.
        if e.is_writable() {
            match self.state {
                SelectMethodReq => (),
                SelectMethodReply => negotiate::select_method_reply(self, r)?,
                ConnectReq => (),
                ConnectReply => negotiate::connect_reply(self, r)?,
                Relay => relay::relay_out(self, r, t)?,
                Shutdown => shutdown::shutdown(self, r)?,
            }
        }

        // hup is a Exception.
        if e.is_hup() {
            return Err(Error::new(ErrorKind::UnexpectedEof, "hup"));
        }

        if e.is_error() {
            return Err(Error::new(ErrorKind::UnexpectedEof, "err"));
        }

        Ok(())
    }
}
