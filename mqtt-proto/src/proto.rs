use std::io;
use std::marker::PhantomData;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;
use tokio_proto::multiplex::{ClientProto, ServerProto};

use codec::Codec;
use core::Packet;

/// MQTT protocol codec
#[derive(Debug, Default)]
pub struct MQTT<'a>(PhantomData<&'a u8>);

impl<'a, T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for MQTT<'a>
where
    Self: 'static,
{
    type Request = Packet<'a>;
    type Response = Option<Packet<'a>>;
    type Transport = Framed<T, Codec<'a>>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(Codec::default()))
    }
}

impl<'a, T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for MQTT<'a>
where
    Self: 'static,
{
    type Request = Option<Packet<'a>>;
    type Response = Packet<'a>;
    type Transport = Framed<T, Codec<'a>>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(Codec::default()))
    }
}
