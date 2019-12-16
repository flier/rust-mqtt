use std::io;
use std::marker::PhantomData;
use std::time::Duration;

use crate::{
    framed::Framed,
    io::{TryClone, WriteExt},
    keepalive::KeepAlive,
    packet::Packet,
    proto::{Disconnect, ServerProperties, MQTT_V5},
};

pub struct Client<T, P = MQTT_V5> {
    stream: KeepAlive<Framed<T>>,
    session_reused: bool,
    properties: ServerProperties,
    phantom: PhantomData<P>,
}

impl<T, P> Client<T, P>
where
    T: 'static + WriteExt + TryClone + Send,
{
    pub fn new(
        framed: Framed<T>,
        keep_alive: Option<Duration>,
        session_reused: bool,
        properties: ServerProperties,
    ) -> Self {
        let keep_alive = properties.keep_alive.or(keep_alive);

        Client {
            stream: KeepAlive::new(framed, keep_alive),
            session_reused,
            properties,
            phantom: PhantomData,
        }
    }

    pub fn disconnect(mut self) -> io::Result<()> {
        self.stream
            .send(Packet::Disconnect(Disconnect::<P>::default().into()))
    }
}
