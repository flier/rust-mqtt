use std::io;
use std::marker::PhantomData;

use crate::{mqtt::Property, packet::Packet, proto::Disconnect, sync::io::WriteExt};

pub struct Client<T, P> {
    stream: T,
    session_reused: bool,
    phantom: PhantomData<P>,
}

impl<T, P> Client<T, P> {
    pub fn new(stream: T, session_reused: bool, properties: Option<Vec<Property>>) -> Self {
        Client {
            stream,
            session_reused,
            phantom: PhantomData,
        }
    }

    pub fn disconnect(mut self) -> io::Result<()>
    where
        T: io::Write,
    {
        self.stream
            .send(Packet::Disconnect(Disconnect::<P>::new().into()))
    }
}
