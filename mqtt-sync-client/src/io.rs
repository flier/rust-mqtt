use std::error::Error as StdError;
use std::io;
use std::net::TcpStream;

use anyhow::Result;
use hexplay::HexViewBuilder;

use crate::packet::{Packet, WriteTo};

pub trait Receiver {
    fn receive(&mut self) -> io::Result<Packet>;
}

pub trait Sender {
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()>;
}

impl<W> Sender for W
where
    W: io::Write,
{
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()> {
        let packet = packet.into();
        let mut buf = Vec::with_capacity(packet.size());
        packet.write_to(&mut buf);
        self.write_all(&buf)?;
        trace!(
            "write {:#?} packet to {} bytes:\n{}",
            packet,
            buf.len(),
            HexViewBuilder::new(&buf).finish()
        );
        Ok(())
    }
}

pub trait TryClone: Sized {
    type Error: StdError;

    fn try_clone(&self) -> Result<Self, Self::Error>;
}

impl TryClone for TcpStream {
    type Error = io::Error;

    fn try_clone(&self) -> Result<Self, Self::Error> {
        TcpStream::try_clone(self)
    }
}
