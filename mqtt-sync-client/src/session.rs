use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicU16, Ordering};

use anyhow::{anyhow, Context, Result};

use crate::{
    io::{Receiver, Sender},
    mqtt::{PacketId, QoS, ReasonCode, Subscription},
    packet::Packet,
    proto::{Message, Protocol, Published, Subscribed, Unsubscribed, MQTT_V5},
};

pub struct Session<'a, T, P = MQTT_V5> {
    stream: T,
    packet_id: AtomicU16,
    received_messages: VecDeque<Message<'a>>,
    unacknowledged_messages: HashMap<PacketId, Unacknowledged<'a>>,
    phantom: PhantomData<P>,
}

enum Unacknowledged<'a> {
    Published(Message<'a>),
    Received(Message<'a>),
    Released(Message<'a>),
}

impl<'a, T, P> Session<'a, T, P> {
    pub fn new(stream: T) -> Self {
        Session {
            stream,
            packet_id: AtomicU16::new(1),
            received_messages: VecDeque::new(),
            unacknowledged_messages: HashMap::new(),
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
    pub fn disconnect(&mut self, reason_code: ReasonCode) -> Result<()> {
        self.stream.send(proto::disconnect::<P>(reason_code))
    }
}

impl<'a, T, P> Session<'a, T, P>
where
    T: Receiver + Sender,
{
    fn wait_for<F, O>(&mut self, mut f: F) -> Result<O>
    where
        F: FnMut(Packet) -> Wait<O>,
        O: 'a,
    {
        let res = loop {
            let packet = self.stream.receive()?;

            let response: Option<Packet> = match f(packet) {
                Wait::Break(res) => break res,
                Wait::Continue(packet) => match packet {
                    Packet::Publish(publish) => {
                        let response = match publish.qos {
                            QoS::AtMostOnce => None,
                            QoS::AtLeastOnce => Some(publish.ack().into()),
                            QoS::ExactlyOnce => Some(publish.received().into()),
                        };
                        let message = Message::from(publish).to_owned();

                        self.received_messages.push_back(message);

                        response
                    }

                    Packet::PublishAck(publish_ack) => {
                        match self.unacknowledged_messages.remove(&publish_ack.packet_id) {
                            Some(Unacknowledged::Published(message)) => {
                                trace!(
                                    "#{} message acknowledged: {:?}",
                                    publish_ack.packet_id,
                                    message
                                );

                                None
                            }
                            Some(_) => break Err(ReasonCode::ProtocolError),
                            None => break Err(ReasonCode::PacketIdNotFound),
                        }
                    }

                    Packet::PublishReceived(publish_received) => {
                        match self
                            .unacknowledged_messages
                            .remove(&publish_received.packet_id)
                        {
                            Some(Unacknowledged::Published(message)) => {
                                trace!(
                                    "#{} message received: {:?}",
                                    publish_received.packet_id,
                                    message
                                );

                                self.unacknowledged_messages.insert(
                                    publish_received.packet_id,
                                    Unacknowledged::Received(message),
                                );

                                Some(publish_received.release().into())
                            }
                            Some(_) => break Err(ReasonCode::ProtocolError),
                            None => break Err(ReasonCode::PacketIdNotFound),
                        }
                    }

                    Packet::PublishRelease(publish_release) => {
                        match self
                            .unacknowledged_messages
                            .remove(&publish_release.packet_id)
                        {
                            Some(Unacknowledged::Published(message)) => {
                                trace!(
                                    "#{} message released: {:?}",
                                    publish_release.packet_id,
                                    message
                                );

                                self.unacknowledged_messages.insert(
                                    publish_release.packet_id,
                                    Unacknowledged::Released(message),
                                );

                                Some(publish_release.complete().into())
                            }
                            Some(_) => break Err(ReasonCode::ProtocolError),
                            None => break Err(ReasonCode::PacketIdNotFound),
                        }
                    }

                    Packet::PublishComplete(publish_complete) => {
                        match self
                            .unacknowledged_messages
                            .remove(&publish_complete.packet_id)
                        {
                            Some(Unacknowledged::Received(message)) => {
                                trace!(
                                    "#{} message completed: {:?}",
                                    publish_complete.packet_id,
                                    message
                                );

                                None
                            }
                            Some(_) => break Err(ReasonCode::ProtocolError),
                            None => break Err(ReasonCode::PacketIdNotFound),
                        }
                    }

                    Packet::Disconnect(disconnect) => {
                        break Err(disconnect.reason_code.unwrap_or_default())
                    }

                    res => {
                        warn!("unexpected packet: {:?}", res);

                        break Err(ReasonCode::ProtocolError);
                    }
                },
            };

            if let Some(packet) = response {
                self.stream.send(packet)?;
            }
        };

        if let Err(code) = res {
            self.disconnect(code)?;
        }

        res.context("wait_for")
    }
}

impl<'a, T, P> Iterator for Session<'a, T, P>
where
    T: Receiver,
{
    type Item = Result<Message<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.received_messages.pop_front().map(Ok).or_else(|| {
            Some(self.stream.receive().and_then(|packet| match packet {
                Packet::Publish(publish) => Ok(Message::from(publish).to_owned()),
                res => Err(anyhow!("unexpected response: {:?}", res)),
            }))
        })
    }
}

enum Wait<'a, O> {
    Break(StdResult<O, ReasonCode>),
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

        self.stream
            .send(proto::subscribe::<'a, P, _, _>(packet_id, subscriptions))?;

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

        self.stream
            .send(proto::unsubscribe::<P, _>(packet_id, topic_filters))?;

        self.wait_for(|packet| match packet {
            Packet::UnsubscribeAck(unsubscribe_ack) if unsubscribe_ack.packet_id == packet_id => {
                Wait::Break(Ok(unsubscribe_ack.into()))
            }
            _ => Wait::Continue(packet),
        })
    }

    pub fn publish(&mut self, message: Message<'a>) -> Result<Published> {
        let qos = message.qos;
        let mut publish = proto::publish::<P>(&message);

        let packet_id = if qos > QoS::AtMostOnce {
            let packet_id = self.next_packet_id();
            publish.with_packet_id(packet_id);
            packet_id
        } else {
            0
        };

        self.stream.send(publish)?;

        match qos {
            QoS::AtMostOnce => Ok(Published::default()),
            QoS::AtLeastOnce => {
                self.unacknowledged_messages
                    .insert(packet_id, Unacknowledged::Published(message));

                self.wait_for(|packet| match packet {
                    Packet::PublishAck(publish_ack) if publish_ack.packet_id == packet_id => {
                        Wait::Break(match publish_ack.reason_code.unwrap_or_default() {
                            ReasonCode::Success => Ok(publish_ack.into()),
                            code => Err(code),
                        })
                    }
                    _ => Wait::Continue(packet),
                })
            }
            QoS::ExactlyOnce => {
                self.unacknowledged_messages
                    .insert(packet_id, Unacknowledged::Published(message));

                self.wait_for(|packet| match packet {
                    Packet::PublishReceived(publish_received)
                        if publish_received.packet_id == packet_id =>
                    {
                        Wait::Break(match publish_received.reason_code.unwrap_or_default() {
                            ReasonCode::Success => Ok(()),
                            code => Err(code),
                        })
                    }
                    _ => Wait::Continue(packet),
                })?;

                self.stream.send(mqtt::PublishRelease {
                    packet_id,
                    reason_code: None,
                    properties: None,
                })?;

                self.wait_for(|packet| match packet {
                    Packet::PublishComplete(publish_complete)
                        if publish_complete.packet_id == packet_id =>
                    {
                        Wait::Break(match publish_complete.reason_code.unwrap_or_default() {
                            ReasonCode::Success => Ok(()),
                            code => Err(code),
                        })
                    }
                    _ => Wait::Continue(packet),
                })?;

                Ok(Published::default())
            }
        }
    }
}
