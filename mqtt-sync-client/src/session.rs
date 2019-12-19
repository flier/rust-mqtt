use std::collections::vec_deque::VecDeque;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU16, Ordering};

use anyhow::{anyhow, Context, Result};

use crate::{
    io::{Receiver, Sender},
    mqtt::{PacketId, QoS, ReasonCode, Subscription},
    packet::Packet,
    proto::{
        Disconnect, Message, Protocol, Publish, Published, Subscribe, Subscribed, Unsubscribe,
        Unsubscribed, MQTT_V5,
    },
};

pub struct Session<'a, T, P = MQTT_V5> {
    stream: T,
    packet_id: AtomicU16,
    pending_messages: VecDeque<Message<'a>>,
    phantom: PhantomData<P>,
}

impl<'a, T, P> Session<'a, T, P> {
    pub fn new(stream: T) -> Self {
        Session {
            stream,
            packet_id: AtomicU16::new(1),
            pending_messages: VecDeque::new(),
            phantom: PhantomData,
        }
    }

    pub fn next_packet_id(&self) -> PacketId {
        self.packet_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl<'a, T, P> Session<'a, T, P>
where
    T: Sender,
{
    pub fn disconnect(mut self) -> Result<()> {
        self.stream
            .send(Packet::Disconnect(Disconnect::<P>::default().into()))
    }
}

impl<'a, T, P> Session<'a, T, P>
where
    T: Receiver,
{
    fn wait_for<F, O>(&mut self, mut f: F) -> Result<O>
    where
        F: FnMut(Packet) -> Wait<O>,
        O: 'a,
    {
        loop {
            let packet = self.stream.receive()?;

            match f(packet) {
                Wait::Break(res) => return res,
                Wait::Continue(packet) => match packet {
                    Packet::Publish(publish) => {
                        self.pending_messages
                            .push_back(Message::from(publish).to_owned());
                    }

                    Packet::Disconnect(disconnect) => {
                        return Err(disconnect.reason_code.unwrap_or_default()).context("subscribe")
                    }

                    res => return Err(anyhow!("unexpected response: {:?}", res)),
                },
            }
        }
    }
}

impl<'a, T, P> Iterator for Session<'a, T, P>
where
    T: Receiver,
{
    type Item = Result<Message<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pending_messages.pop_front().map(Ok).or_else(|| {
            Some(self.stream.receive().and_then(|packet| match packet {
                Packet::Publish(publish) => Ok(Message::from(publish).to_owned()),
                res => Err(anyhow!("unexpected response: {:?}", res)),
            }))
        })
    }
}

enum Wait<'a, O> {
    Break(Result<O>),
    Continue(Packet<'a>),
}

impl<'a, T, P> Session<'a, T, P>
where
    T: Receiver + Sender,
    P: Protocol,
{
    pub fn subscribe<I, S>(&mut self, subscriptions: I) -> Result<Subscribed>
    where
        I: IntoIterator<Item = S>,
        S: Into<Subscription<'a>>,
    {
        let packet_id = self.next_packet_id();

        self.stream.send(Packet::Subscribe(
            Subscribe::<'a, P>::new(packet_id, subscriptions).into(),
        ))?;

        self.wait_for(|packet| match packet {
            Packet::SubscribeAck(subscribe_ack) if subscribe_ack.packet_id == packet_id => {
                Wait::Break(Ok(subscribe_ack.into()))
            }
            _ => Wait::Continue(packet),
        })
    }

    pub fn unsubscribe<I>(&mut self, topic_filters: I) -> Result<Unsubscribed>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let packet_id = self.next_packet_id();

        self.stream.send(Packet::Unsubscribe(
            Unsubscribe::<'a, P>::new(packet_id, topic_filters).into(),
        ))?;

        self.wait_for(|packet| match packet {
            Packet::UnsubscribeAck(unsubscribe_ack) if unsubscribe_ack.packet_id == packet_id => {
                Wait::Break(Ok(unsubscribe_ack.into()))
            }
            _ => Wait::Continue(packet),
        })
    }

    pub fn publish(&mut self, message: Message) -> Result<Published> {
        let mut publish = Publish::<P>::new(&message);

        let packet_id = if message.qos > QoS::AtMostOnce {
            let packet_id = self.next_packet_id();
            publish.with_packet_id(packet_id);
            Some(packet_id)
        } else {
            None
        };

        self.stream.send(Packet::Publish(publish.into()))?;

        match message.qos {
            QoS::AtMostOnce => Ok(Published::default()),
            QoS::AtLeastOnce => self.wait_for(|packet| match packet {
                Packet::PublishAck(publish_ack) if publish_ack.packet_id == packet_id.unwrap() => {
                    Wait::Break(match publish_ack.reason_code.unwrap_or_default() {
                        ReasonCode::Success => Ok(publish_ack.into()),
                        code => Err(code).context("publish_ack"),
                    })
                }
                _ => Wait::Continue(packet),
            }),
            QoS::ExactlyOnce => self.wait_for(|packet| match packet {
                Packet::PublishReceived(publish_received)
                    if publish_received.packet_id == packet_id.unwrap() =>
                {
                    Wait::Break(match publish_received.reason_code.unwrap_or_default() {
                        ReasonCode::Success => Ok(publish_received.into()),
                        code => Err(code).context("publish_received"),
                    })
                }
                _ => Wait::Continue(packet),
            }),
        }
    }
}
