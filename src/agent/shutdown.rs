use super::Agent;
use mio::*;
use std::io;
use std::io::Error;
use std::io::ErrorKind;

pub fn shutdown(c: &mut Agent, _r: &Registry) -> io::Result<()> {
    let s1 = &mut c.s1;
    let s2 = c.s2.as_mut().unwrap();

    let ea = c.b1.copy(s1, s2)?;
    if c.b1.len() > 0 || ea {
        return Ok(());
    }

    let ea = c.b2.copy(s2, s1)?;
    if c.b2.len() > 0 || ea {
        return Ok(());
    }

    Err(Error::from(ErrorKind::Other)) // TODO
}
