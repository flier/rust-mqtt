use std::io;
use std::iter;
use std::marker::PhantomData;

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tokio_codec::Framed;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_proto::multiplex::{ClientProto, ServerProto};

use crate::codec::Codec;
use crate::core::Packet;

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
        Ok(Framed::new(io, Codec::default()))
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
        Ok(Framed::new(io, Codec::default()))
    }
}

/// A unique Client identifier for the Client
pub fn client_id(size: usize) -> String {
    let mut rng = thread_rng();

    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(size)
        .collect()
}
