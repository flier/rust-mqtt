use std::io;
use std::marker::PhantomData;

use bytes::BytesMut;
use nom::{IError, IResult};

use tokio_io::codec::{Decoder, Encoder};
use tokio_proto::multiplex::RequestId;

use core::{Packet, ReadPacketExt, WritePacketExt, decode_variable_length_usize};

/// MQTT protocol codec
#[derive(Debug, Default)]
pub struct Codec<'a>(PhantomData<&'a u8>);

impl<'a> Encoder for Codec<'a> {
    type Item = (RequestId, Packet<'a>);
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        let (packet_id, packet) = msg;
        let mut bytes = Vec::new();
        let packet_size = bytes.write_packet(&packet)?;

        trace!("encode packet #{} to {} bytes", packet_id, packet_size);

        buf.extend(&bytes[..packet_size]);

        Ok(())
    }
}

impl<'a> Decoder for Codec<'a> {
    type Item = (RequestId, Packet<'a>);
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        if let IResult::Done(_, size) = decode_variable_length_usize(&buf[1..]) {
            match buf.split_to(size).read_packet() {
                Ok(packet) => {
                    trace!("decode {} bytes to packet", size);

                    Ok(Some((
                        RequestId::from(packet.packet_id().unwrap_or(0)),
                        packet.into_owned(),
                    )))
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
