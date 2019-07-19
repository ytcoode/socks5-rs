#![deny(warnings)]

use std::io;
use std::net::TcpListener;
use std::thread;

mod client;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:9999")?;

    for r in listener.incoming() {
        let stream = r?;
        thread::spawn(move || {
            client::handle_client(stream);
        });
    }

    Ok(())
}
