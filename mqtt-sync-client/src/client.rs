use std::io;
use std::marker::PhantomData;
use std::time::Duration;

use crate::{
    io::WriteExt,
    packet::Packet,
    proto::{Disconnect, ServerProperties},
};

pub struct Client<T, P> {
    stream: T,
    keep_alive: Option<Duration>,
    session_reused: bool,
    properties: ServerProperties,
    phantom: PhantomData<P>,
}

impl<T, P> Client<T, P> {
    pub fn new(
        stream: T,
        keep_alive: Option<Duration>,
        session_reused: bool,
        properties: ServerProperties,
    ) -> Self {
        Client {
            stream,
            keep_alive: properties.keep_alive.or(keep_alive),
            session_reused,
            properties,
            phantom: PhantomData,
        }
    }

    pub fn disconnect(mut self) -> io::Result<()>
    where
        T: io::Write,
    {
        self.stream
            .send(Packet::Disconnect(Disconnect::<P>::default().into()))
    }
}
