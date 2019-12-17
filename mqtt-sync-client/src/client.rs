use std::collections::VecDeque;
use std::io;
use std::iter::IntoIterator;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::{
    framed::Framed,
    io::{Receiver, Sender, TryClone},
    keepalive::KeepAlive,
    mqtt::Subscription,
    packet::Packet,
    proto::{
        Disconnect, Message, Protocol, ServerProperties, Subscribe, Subscribed, Unsubscribe,
        Unsubscribed, MQTT_V5,
    },
};

pub struct Client<T, P = MQTT_V5> {
    stream: KeepAlive<Framed<T>>,
    session_reused: bool,
    properties: ServerProperties,
    packet_id: AtomicU16,
    pending_messages: VecDeque<Message>,
    phantom: PhantomData<P>,
}

impl<T, P> Client<T, P> {
    fn next_packet_id(&self) -> u16 {
        self.packet_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl<T, P> Client<T, P>
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
            stream: KeepAlive::new(framed, keep_alive),
            session_reused,
            properties,
            packet_id: AtomicU16::new(1),
            pending_messages: VecDeque::new(),
            phantom: PhantomData,
        }
    }

    pub fn disconnect(mut self) -> Result<()> {
        self.stream
            .send(Packet::Disconnect(Disconnect::<P>::default().into()))
    }
}

impl<T, P> Client<T, P>
where
    T: 'static + io::Write + io::Read + TryClone + Send,
    P: Protocol,
{
    pub fn subscribe<'a, I, S>(&mut self, subscriptions: I) -> Result<Subscribed>
    where
        I: IntoIterator<Item = S>,
        S: Into<Subscription<'a>>,
    {
        let packet_id = self.next_packet_id();

        self.stream.send(Packet::Subscribe(
            Subscribe::<'a, P>::new(packet_id, subscriptions).into(),
        ))?;

        loop {
            match self.stream.receive()? {
                Packet::SubscribeAck(subscribe_ack) if subscribe_ack.packet_id == packet_id => {
                    return Ok(subscribe_ack.into())
                }
                Packet::Publish(publish) => self.pending_messages.push_back(publish.into()),
                res => return Err(anyhow!("unexpected response: {:?}", res)),
            }
        }
    }

    pub fn unsubscribe<'a, I>(&mut self, topic_filters: I) -> Result<Unsubscribed>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let packet_id = self.next_packet_id();

        self.stream.send(Packet::Unsubscribe(
            Unsubscribe::<'a, P>::new(packet_id, topic_filters).into(),
        ))?;

        loop {
            match self.stream.receive()? {
                Packet::UnsubscribeAck(unsubscribe_ack)
                    if unsubscribe_ack.packet_id == packet_id =>
                {
                    return Ok(unsubscribe_ack.into())
                }
                Packet::Publish(publish) => self.pending_messages.push_back(publish.into()),
                res => return Err(anyhow!("unexpected response: {:?}", res)),
            }
        }
    }
}

impl<T, P> Client<T, P> {
    pub fn messages(&mut self) -> Messages<T, P> {
        Messages { client: self }
    }
}

pub struct Messages<'a, T, P> {
    client: &'a mut Client<T, P>,
}

impl<'a, T, P> Iterator for Messages<'a, T, P>
where
    T: io::Read,
{
    type Item = Result<Message>;

    fn next(&mut self) -> Option<Self::Item> {
        self.client
            .pending_messages
            .pop_front()
            .map(Ok)
            .or_else(|| {
                Some(
                    self.client
                        .stream
                        .receive()
                        .and_then(|packet| match packet {
                            Packet::Publish(publish) => Ok(publish.into()),
                            res => Err(anyhow!("unexpected response: {:?}", res)),
                        }),
                )
            })
    }
}
