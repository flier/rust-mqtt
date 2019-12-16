use core::mem::MaybeUninit;
use core::slice;
use std::io;

use bytes::{Buf, BufMut, BytesMut};
use hexplay::HexViewBuilder;

use crate::{
    io::{ReadExt, TryClone, WriteExt},
    mqtt::ProtocolVersion,
    packet::{self, Packet},
};

pub struct Framed<T> {
    inner: T,
    eof: bool,
    readable: bool,
    buffer: BytesMut,
    protocol_version: ProtocolVersion,
}

const INITIAL_CAPACITY: usize = 8 * 1024;

impl<R> Framed<R> {
    pub fn new(inner: R, protocol_version: ProtocolVersion) -> Self {
        Self::with_capacity(inner, protocol_version, INITIAL_CAPACITY)
    }

    pub fn with_capacity(inner: R, protocol_version: ProtocolVersion, capacity: usize) -> Self {
        Self::with_buffer(inner, protocol_version, BytesMut::with_capacity(capacity))
    }

    pub fn with_buffer(inner: R, protocol_version: ProtocolVersion, mut buffer: BytesMut) -> Self {
        if buffer.capacity() < INITIAL_CAPACITY {
            buffer.reserve(INITIAL_CAPACITY - buffer.capacity());
        }

        Framed {
            inner,
            eof: false,
            readable: !buffer.is_empty(),
            buffer,
            protocol_version,
        }
    }

    fn parse_packet<'a, 'b: 'a>(&'a mut self) -> io::Result<Option<Packet<'b>>> {
        let input = self.buffer.bytes();
        let input = unsafe { slice::from_raw_parts(input.as_ptr(), input.len()) };
        let res = packet::parse(input, self.protocol_version);

        match res {
            Ok((remaining, packet)) => {
                let read = self.buffer.len() - remaining.len();

                trace!(
                    "read {:#?} packet from {} bytes:\n{}",
                    packet,
                    read,
                    HexViewBuilder::new(&input[..read]).finish()
                );

                self.buffer.advance(read);
                Ok(Some(packet))
            }
            Err(err) if self.eof => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("fail to parse packet, {:?}", err),
            )),
            _ => {
                self.readable = false;
                Ok(None)
            }
        }
    }

    fn fill_buf(&mut self) -> io::Result<()>
    where
        R: io::Read,
    {
        self.buffer.reserve(1);
        let read = unsafe {
            let b = self.buffer.bytes_mut();

            for x in &mut b[..] {
                *x.as_mut_ptr() = 0;
            }

            // Convert to `&mut [u8]`
            let b = &mut *(b as *mut [MaybeUninit<u8>] as *mut [u8]);

            self.inner.read(b)?
        };

        unsafe {
            self.buffer.advance_mut(read);
        }

        if read == 0 {
            self.eof = true;
        }
        self.readable = true;

        Ok(())
    }
}

impl<R> ReadExt for Framed<R>
where
    R: io::Read,
{
    fn receive(&mut self) -> io::Result<Packet> {
        loop {
            if self.readable {
                if let Some(packet) = self.parse_packet()? {
                    return Ok(packet);
                }
            }

            self.fill_buf()?;
        }
    }
}

impl<W> WriteExt for Framed<W>
where
    W: WriteExt,
{
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()> {
        self.inner.send(packet)
    }
}

impl<T> TryClone for Framed<T>
where
    T: TryClone,
{
    type Error = T::Error;

    fn try_clone(&self) -> Result<Self, Self::Error> {
        self.inner.try_clone().map(|inner| Framed {
            inner,
            eof: false,
            readable: false,
            buffer: BytesMut::with_capacity(self.buffer.capacity()),
            protocol_version: self.protocol_version,
        })
    }
}
