use std::io;
use std::rc::Rc;
use std::marker::PhantomData;

use bytes::{Bytes, BytesMut};
use nom::{IResult, IError};

use tokio_io::codec::{Encoder, Decoder};

use core::{Packet, WritePacketExt, ReadPacketExt, decode_variable_length_usize};

/// MQTT protocol codec
pub struct Codec<'a>(PhantomData<&'a u8>);

impl<'a> Encoder for Codec<'a> {
    type Item = Packet<'a>;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        let mut bytes = Vec::new();
        let packet_size = bytes.write_packet(&msg)?;

        trace!("encode packet to {} bytes", packet_size);

        buf.extend(&bytes[..packet_size]);

        Ok(())
    }
}

impl<'a> Decoder for Codec<'a> {
    type Item = Packet<'a>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Packet<'a>>> {
        if let IResult::Done(_, size) = decode_variable_length_usize(&buf[1..]) {
            match buf.split_to(size).read_packet() {
                Ok(packet) => {
                    trace!("decode {} bytes to packet", size);

                    Ok(Some(packet.into_owned()))
                }
                Err(IError::Incomplete(_)) => {
                    trace!("skip incomplete packet");

                    Ok(None)
                }
                Err(IError::Error(err)) => {
                    warn!("fail to decode packet, {}", err);

                    Err(io::Error::new(io::ErrorKind::Other, err.description()))
                }
            }
        } else {
            Ok(None)
        }
    }
}
