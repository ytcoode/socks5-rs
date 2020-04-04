use std::io;
use std::io::ErrorKind;
use std::io::IoSlice;
use std::io::IoSliceMut;
use std::io::Read;
use std::io::Write;
use std::ops::Index;

const CAP: usize = 4096; // 确保空间足够大到存放任意消息

pub struct Buf {
    buf: [u8; CAP],
    idx: usize,
    len: usize,
}

impl Index<usize> for Buf {
    type Output = u8;

    fn index(&self, mut i: usize) -> &Self::Output {
        assert!(i < self.len);
        i += self.idx;
        if i >= CAP {
            i -= CAP;
        }
        &self.buf[i]
    }
}

impl Buf {
    pub fn new() -> Self {
        Buf {
            buf: [0; CAP],
            idx: 0,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        return self.len;
    }
}

impl Buf {
    pub fn read_u8(&mut self) -> u8 {
        assert!(self.len >= 1);
        let r = self.buf[self.idx];
        self.skip(1);
        r
    }

    pub fn read_u16(&mut self) -> u16 {
        (self.read_u8() as u16) << 8 | self.read_u8() as u16
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = self.read_u8();
        }
    }

    pub fn skip(&mut self, n: usize) {
        assert!(n <= self.len);
        self.idx += n;
        if self.idx >= CAP {
            self.idx -= CAP;
        }
        self.len -= n;
    }

    pub fn write_u8(&mut self, v: u8) {
        assert!(self.len < CAP);
        let mut i = self.idx + self.len;
        if i >= CAP {
            i -= CAP;
        }
        self.buf[i] = v;
        self.len += 1;
    }
}

impl Buf {
    // 从Read中读数据到buf中
    pub fn read<R: Read>(&mut self, r: &mut R) -> io::Result<bool> {
        loop {
            let mut bs = self.io_slice_read();
            if bs.len() == 0 {
                // buf满了，此时buf内部肯定已包含该状态所需的所有数据
                assert_eq!(self.len, CAP);
                break;
            }
            match r.read_vectored(&mut bs[..]) {
                Ok(0) => break, // 即使socket已经关闭，我们也要先处理完buf内的数据
                Ok(n) => {
                    self.len += n;
                    assert!(self.len <= CAP);
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(true),
                Err(e) => return Err(e),
            }
        }
        Ok(false)
    }

    // 从buf中写数据到Write中
    pub fn write<W: Write>(&mut self, w: &mut W) -> io::Result<bool> {
        loop {
            let bs = self.io_slice_write();
            if bs.len() == 0 {
                assert_eq!(self.len, 0);
                return Ok(false);
            }
            match w.write_vectored(&bs[..]) {
                Ok(n) => self.skip(n),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(true),
                Err(e) => return Err(e),
            }
        }
    }

    pub fn copy<R: Read, W: Write>(&mut self, r: &mut R, w: &mut W) -> io::Result<bool> {
        loop {
            let ea = self.read(r)?;
            if self.len == 0 {
                return Ok(ea);
            }

            self.write(w)?;
            if self.len > 0 {
                return Ok(true);
            }
        }
    }

    fn io_slice_read(&mut self) -> Vec<IoSliceMut> {
        // Assert Currently length less than CAP.
        assert!(self.len <= CAP);
        if self.len == CAP {
            return vec![];
        }

        // New a Vec that Size is 2.
        let mut bs = Vec::with_capacity(2);
        // ???
        let i1 = self.idx;
        // ???
        let i2 = self.idx + self.len;

        if i2 < CAP {
            // split_at_mut: Divides one mutable slice into two at an index.
            // split_at_mut 一个切片分成两个，以索引为界。
            let (b1, b2) = self.buf.split_at_mut(i2);
            bs.push(IoSliceMut::new(b2));
            if i1 > 0 {
                bs.push(IoSliceMut::new(&mut b1[..i1]));
            }
        } else {
            bs.push(IoSliceMut::new(&mut self.buf[i2 - CAP..i1]));
        }
        bs
    }

    fn io_slice_write(&self) -> Vec<IoSlice> {
        if self.len == 0 {
            return vec![];
        }

        let mut bs = Vec::with_capacity(2);
        let i1 = self.idx;
        let mut i2 = self.idx + self.len;

        if i2 <= CAP {
            bs.push(IoSlice::new(&self.buf[i1..i2]));
        } else {
            bs.push(IoSlice::new(&self.buf[i1..]));
            i2 -= CAP;
            if i2 > 0 {
                bs.push(IoSlice::new(&self.buf[..i2]));
            }
        }
        bs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let b = Buf::new();
        assert_eq!(b.idx, 0);
        assert_eq!(b.len, 0);
        assert_eq!(b.buf.len(), CAP);
    }

    #[test]
    fn io_slice_mut() {
        assert!(CAP >= 20);
        let mut b = Buf::new();
        b.idx = 10;
        b.len = 10;

        let mut v = b.io_slice_read();
        let n = io::repeat(1).read_vectored(&mut v[..]).unwrap();
        assert_eq!(n, CAP - 10);

        for i in 0..10 {
            assert_eq!(b.buf[i], 1);
        }

        for i in 10..20 {
            assert_eq!(b.buf[i], 0);
        }

        for i in 20..CAP {
            assert_eq!(b.buf[i], 1);
        }
    }
}
