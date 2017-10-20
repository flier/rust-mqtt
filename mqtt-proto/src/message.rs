use std::borrow::Cow;
use std::collections::HashMap;
use time::now;

use slab::Slab;

use core::{PacketId, QoS};
use errors::{ErrorKind, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<'a> {
    packet_id: PacketId,
    topic: Cow<'a, str>,
    payload: Cow<'a, [u8]>,
    ts: i64,
}

pub trait MessageForwarder {
    fn forward(&mut self, message: Message) -> Result<()>;
}

#[derive(Debug)]
enum SendState<'a> {
    Sending(Message<'a>),
    Received(PacketId),
}

#[derive(Debug)]
pub struct MessageSender<'a> {
    messages: Slab<SendState<'a>>,
}

impl<'a> Default for MessageSender<'a> {
    fn default() -> Self {
        MessageSender { messages: Slab::with_capacity(64) }
    }
}

impl<'a> MessageSender<'a> {
    pub fn publish(&mut self, topic: Cow<'a, str>, payload: Cow<'a, [u8]>) -> Result<PacketId> {
        let entry = self.messages.vacant_entry();
        let packet_id = entry.key() as PacketId;

        entry.insert(SendState::Sending(Message {
            packet_id,
            topic,
            payload,
            ts: now().to_timespec().sec,
        }));

        Ok(packet_id)
    }

    pub fn on_publish_ack(&mut self, packet_id: PacketId) -> Result<PacketId> {
        match self.messages.remove(packet_id as usize) {
            SendState::Sending(Message { packet_id, .. }) => {
                trace!("message #{} acked", packet_id);

                Ok(packet_id)
            }
            _ => bail!(ErrorKind::UnexpectedState),
        }
    }

    pub fn on_publish_received(&mut self, packet_id: PacketId) -> Result<PacketId> {
        let state = self.messages.get_mut(packet_id as usize);

        match state {
            Some(&mut SendState::Sending(Message { packet_id, .. })) => {
                trace!("message #{} received", packet_id);

                *state.unwrap() = SendState::Received(packet_id);

                Ok(packet_id)
            }
            _ => bail!(ErrorKind::UnexpectedState),
        }
    }

    pub fn on_publish_complete(&mut self, packet_id: PacketId) -> Result<PacketId> {
        match self.messages.get(packet_id as usize) {
            Some(&SendState::Received(packet_id)) => {
                trace!("message #{} compileted", packet_id);

                self.messages.remove(packet_id as usize);

                Ok(packet_id)
            }
            _ => bail!(ErrorKind::UnexpectedState),
        }
    }
}

#[derive(Debug, Default)]
pub struct MessageReceiver<'a> {
    messages: HashMap<PacketId, Message<'a>>,
}

impl<'a> MessageReceiver<'a> {
    pub fn on_publish(
        &mut self,
        dup: bool,
        retain: bool,
        qos: QoS,
        topic: Cow<'a, str>,
        packet_id: Option<PacketId>,
        payload: Cow<'a, [u8]>,
    ) -> Result<Option<PacketId>> {
        match qos {
            QoS::AtMostOnce => Ok(None),
            QoS::AtLeastOnce if packet_id.is_some() => {
                // the Server MUST store the Application Message and its QoS,
                // so that it can be delivered to future subscribers
                // whose subscriptions match its topic name [MQTT-3.3.1-5].
                Ok(packet_id)
            }
            QoS::ExactlyOnce if packet_id.is_some() => {
                if let Some(packet_id) = packet_id {
                    self.messages.insert(
                        packet_id,
                        Message {
                            packet_id,
                            topic,
                            payload,
                            ts: now().to_timespec().sec,
                        },
                    );
                }

                Ok(packet_id)
            }
            _ => bail!(ErrorKind::ProtocolViolation),
        }
    }

    pub fn on_publish_release(&mut self, packet_id: PacketId) -> Result<Message> {
        self.messages.remove(&packet_id).ok_or_else(|| {
            ErrorKind::UnexpectedState.into()
        })
    }
}
