use std::io;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use anyhow::Result;

use crate::{
    framed::Framed,
    io::{Sender, TryClone},
    keepalive::KeepAlive,
    proto::{Message, ServerProperties, MQTT_V5},
    session::Session,
};

pub struct Client<'a, T, P = MQTT_V5> {
    session: Session<'a, KeepAlive<Framed<T>>, P>,
    session_reused: bool,
    properties: ServerProperties,
}

impl<'a, T, P> From<Client<'a, T, P>> for Session<'a, KeepAlive<Framed<T>>, P> {
    fn from(client: Client<'a, T, P>) -> Self {
        client.session
    }
}

impl<'a, T, P> Deref for Client<'a, T, P> {
    type Target = Session<'a, KeepAlive<Framed<T>>, P>;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl<'a, T, P> DerefMut for Client<'a, T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl<'a, T, P> Client<'a, T, P>
where
    T: 'static + Sender + TryClone + Send,
{
    pub fn new(
        framed: Framed<T>,
        keep_alive: Option<Duration>,
        session_reused: bool,
        properties: ServerProperties,
    ) -> Self {
        let keep_alive = properties.keep_alive.or(keep_alive);

        Client {
            session: Session::new(KeepAlive::new(framed, keep_alive)),
            session_reused,
            properties,
        }
    }
}

impl<'a, T, P> Client<'a, T, P>
where
    T: 'static + Sender + TryClone + Send,
{
    pub fn disconnect(self) -> Result<()> {
        self.session.disconnect()
    }
}

impl<'a, T, P> Client<'a, T, P>
where
    T: io::Read,
{
    pub fn messages(self) -> Messages<'a, T, P> {
        Messages(self)
    }
}

#[repr(transparent)]
pub struct Messages<'a, T, P>(Client<'a, T, P>);

impl<'a, T, P> From<Messages<'a, T, P>> for Client<'a, T, P> {
    fn from(messages: Messages<'a, T, P>) -> Self {
        messages.0
    }
}

impl<'a, T, P> Deref for Messages<'a, T, P> {
    type Target = Client<'a, T, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T, P> DerefMut for Messages<'a, T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T, P> Iterator for Messages<'a, T, P>
where
    T: io::Read,
{
    type Item = Result<Message<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.session.next()
    }
}
