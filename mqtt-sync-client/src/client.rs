use std::io;
use std::marker::PhantomData;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::{
    framed::Framed,
    io::{Receiver, Sender, TryClone},
    keepalive::KeepAlive,
    mqtt::{QoS, ReasonCode, Subscription},
    packet::Packet,
    proto::{Disconnect, Protocol, ServerProperties, Subscribe, MQTT_V5},
};

pub struct Client<T, P = MQTT_V5> {
    stream: KeepAlive<Framed<T>>,
    session_reused: bool,
    properties: ServerProperties,
    packet_id: AtomicU16,
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
    pub fn subscribe<'a, I, S>(
        &mut self,
        subscriptions: I,
    ) -> Result<Vec<StdResult<QoS, ReasonCode>>>
    where
        I: IntoIterator<Item = S>,
        S: Into<Subscription<'a>>,
    {
        let packet_id = self.next_packet_id();

        self.stream.send(Packet::Subscribe(
            Subscribe::<'a, P>::new(packet_id, subscriptions).into(),
        ))?;

        match self.stream.receive()? {
            Packet::SubscribeAck(subscribe_ack) if subscribe_ack.packet_id == packet_id => {
                Ok(subscribe_ack.status)
            }
            res => Err(anyhow!("unexpected response: {:?}", res)),
        }
    }
}
