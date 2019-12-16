use std::io;

use hexplay::HexViewBuilder;

use crate::mqtt::ProtocolVersion;
use crate::packet::{Packet, WriteTo};

pub trait WriteExt {
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()>;
}

impl<T: io::Write> WriteExt for T {
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

pub trait ReadExt {
    fn receive<'a>(
        &mut self,
        buf: &'a mut [u8],
        protocol_version: ProtocolVersion,
    ) -> io::Result<(&'a [u8], Packet<'a>)>;
}

impl<T: io::Read> ReadExt for T {
    fn receive<'a>(
        &mut self,
        buf: &'a mut [u8],
        protocol_version: ProtocolVersion,
    ) -> io::Result<(&'a [u8], Packet<'a>)> {
        let read = self.read(buf)?;
        let input = &buf[..read];

        let (remaining, packet) = packet::parse::<()>(input, protocol_version).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("parse packet failed, {:?}", err),
            )
        })?;
        trace!(
            "read {:#?} packet from {} bytes:\n{}",
            packet,
            read,
            HexViewBuilder::new(input).finish()
        );

        Ok((remaining, packet))
    }
}
