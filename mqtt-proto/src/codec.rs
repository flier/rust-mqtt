use std::io;
use std::marker::PhantomData;

use bytes::BytesMut;
use nom::Err;

use tokio_io::codec::{Decoder, Encoder};
use tokio_proto::multiplex::RequestId;

use core::{decode_variable_length_usize, Packet, ReadPacketExt, WritePacketExt};

/// MQTT protocol codec
#[derive(Debug, Default)]
pub struct Codec<'a>(PhantomData<&'a u8>);

impl<'a> Encoder for Codec<'a> {
    type Item = (RequestId, Option<Packet<'a>>);
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        if let (packet_id, Some(packet)) = msg {
            let mut bytes = Vec::new();
            let packet_size = bytes.write_packet(&packet)?;

            trace!("encode packet #{} to {} bytes", packet_id, packet_size);

            buf.extend(&bytes[..packet_size]);
        }

        Ok(())
    }
}

impl<'a> Decoder for Codec<'a> {
    type Item = (RequestId, Packet<'a>);
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        if let Ok((_, size)) = decode_variable_length_usize(&buf[1..]) {
            match buf.split_to(size).read_packet() {
                Ok(packet) => {
                    trace!("decode {} bytes to packet", size);

                    Ok(Some((
                        RequestId::from(packet.packet_id().unwrap_or_default()),
                        packet.into_owned(),
                    )))
                }
                Err(Err::Incomplete(_)) => {
                    trace!("skip incomplete packet");

                    Ok(None)
                }
                Err(Err::Error(err)) | Err(Err::Failure(err)) => {
                    let err = err.into_error_kind();
                    let err = err.description();

                    warn!("fail to decode packet, {}", err);

                    Err(io::Error::new(io::ErrorKind::Other, err))
                }
            }
        } else {
            Ok(None)
        }
    }
}
