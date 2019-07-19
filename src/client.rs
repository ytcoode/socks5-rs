use std::io::Read;
use std::io::Write;
use std::net::Ipv4Addr;
use std::net::TcpStream;

const VERSION: u8 = 0x05;

pub fn handle_client(mut s: TcpStream) {
    s.set_nodelay(true).expect("set_nodelay failed");

    /*

    +----+----------+----------+
    |VER | NMETHODS | METHODS  |
    +----+----------+----------+
    | 1  |    1     | 1 to 255 |
    +----+----------+----------+

    */

    let mut buf = [0; 2];
    s.read_exact(&mut buf).unwrap();

    if buf[0] != VERSION {
        panic!("illegal version");
    }

    let mut vec = vec![0; buf[1] as usize];
    s.read_exact(&mut vec).unwrap();

    // TODO select a valid method

    /*

    +----+--------+
    |VER | METHOD |
    +----+--------+
    | 1  |   1    |
    +----+--------+

    */

    s.write_all(&[VERSION, 0]).unwrap();

    /*

    +----+-----+-------+------+----------+----------+
    |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
    +----+-----+-------+------+----------+----------+
    | 1  |  1  | X'00' |  1   | Variable |    2     |
    +----+-----+-------+------+----------+----------+

     */

    let mut buf = [0; 4];
    s.read_exact(&mut buf).unwrap();

    if buf[0] != VERSION {
        panic!("illegal version");
    }

    if buf[1] != 1 {
        panic!("CMD must be connect");
    }

    if buf[2] != 0 {
        panic!("illegal rsv");
    }

    if buf[3] != 1 {
        panic!("atyp must be ipv4");
    }

    let mut buf = [0; 6];
    s.read_exact(&mut buf).unwrap();

    let ip = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
    let port = (buf[4] as u16) << 8 | buf[5] as u16;

    let _t = TcpStream::connect((ip, port)).unwrap();

    unimplemented!();
}
