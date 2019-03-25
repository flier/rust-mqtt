use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;
use time::now;

use slab::Slab;

use crate::core::{PacketId, QoS};
use crate::errors::{ErrorKind::*, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<'a> {
    topic: Cow<'a, str>,
    payload: Cow<'a, [u8]>,
    ts: i64,
}

pub trait MessageForwarder {
    fn forward(&mut self, message: Message) -> Result<()>;
}

#[derive(Debug)]
pub enum SendState<'a> {
    Sending(Message<'a>),
    Received(PacketId),
}

#[derive(Debug)]
pub struct MessageSender<'a> {
    messages: Slab<SendState<'a>>,
}

impl<'a> Default for MessageSender<'a> {
    fn default() -> Self {
        MessageSender {
            messages: Slab::with_capacity(64),
        }
    }
}

impl<'a> Deref for MessageSender<'a> {
    type Target = Slab<SendState<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl<'a> MessageSender<'a> {
    pub fn publish(&mut self, topic: Cow<'a, str>, payload: Cow<'a, [u8]>) -> Result<PacketId> {
        let entry = self.messages.vacant_entry();
        let packet_id = entry.key() as PacketId + 1;

        entry.insert(SendState::Sending(Message {
            topic,
            payload,
            ts: now().to_timespec().sec,
        }));

        Ok(packet_id)
    }

    pub fn on_publish_ack(&mut self, packet_id: PacketId) -> Result<PacketId> {
        let key = packet_id as usize - 1;

        if !self.messages.contains(key) {
            Err(InvalidPacketId.into())
        } else {
            match self.messages.remove(key) {
                SendState::Sending(Message { .. }) => {
                    trace!("message #{} acked", packet_id);

                    Ok(packet_id)
                }
                _ => Err(UnexpectedState.into()),
            }
        }
    }

    pub fn on_publish_received(&mut self, packet_id: PacketId) -> Result<PacketId> {
        let state = self.messages.get_mut(packet_id as usize - 1);

        match state {
            Some(&mut SendState::Sending(Message { .. })) => {
                trace!("message #{} received", packet_id);

                *state.unwrap() = SendState::Received(packet_id);

                Ok(packet_id)
            }
            None => Err(InvalidPacketId.into()),
            _ => Err(UnexpectedState.into()),
        }
    }

    pub fn on_publish_complete(&mut self, packet_id: PacketId) -> Result<PacketId> {
        let key = packet_id as usize - 1;

        match self.messages.get(key) {
            Some(&SendState::Received(id)) if id == packet_id => {
                trace!("message #{} compileted", packet_id);

                self.messages.remove(key);

                Ok(packet_id)
            }
            None => Err(InvalidPacketId.into()),
            _ => Err(UnexpectedState.into()),
        }
    }
}

#[derive(Debug, Default)]
pub struct MessageReceiver<'a> {
    messages: HashMap<PacketId, Message<'a>>,
}

impl<'a> Deref for MessageReceiver<'a> {
    type Target = HashMap<PacketId, Message<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl<'a> MessageReceiver<'a> {
    pub fn on_publish(
        &mut self,
        _dup: bool,
        _retain: bool,
        qos: QoS,
        packet_id: Option<PacketId>,
        topic: Cow<'a, str>,
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
                            topic,
                            payload,
                            ts: now().to_timespec().sec,
                        },
                    );
                }

                Ok(packet_id)
            }
            _ => Err(ProtocolViolation.into()),
        }
    }

    pub fn on_publish_release(&mut self, packet_id: PacketId) -> Result<Message> {
        self.messages
            .remove(&packet_id)
            .ok_or_else(|| InvalidPacketId.into())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::errors::ErrorKind::InvalidPacketId;

    #[test]
    fn test_message_sender() {
        let mut sender = MessageSender::default();

        assert_eq!(sender.len(), 0);

        let foo = sender
            .publish(Cow::from("topic"), Cow::from(&b"foo"[..]))
            .unwrap();
        let bar = sender
            .publish(Cow::from("topic"), Cow::from(&b"bar"[..]))
            .unwrap();

        assert_eq!(foo, 1);
        assert_eq!(bar, 2);
        assert_eq!(sender.len(), 2);

        assert_matches!(sender[foo as usize - 1], SendState::Sending(Message { .. }));
        assert_matches!(sender.on_publish_ack(foo), Ok(1));
        assert!(sender.get(foo as usize - 1).is_none());

        assert_matches!(sender[bar as usize - 1], SendState::Sending(Message { .. }));
        assert_matches!(sender.on_publish_received(bar), Ok(2));
        assert_matches!(sender[bar as usize - 1], SendState::Received(_));
        assert_matches!(sender.on_publish_complete(bar), Ok(2));
        assert!(sender.get(bar as usize - 1).is_none());

        assert_matches!(
            sender
                .on_publish_ack(foo)
                .err()
                .unwrap()
                .downcast_ref()
                .unwrap(),
            &InvalidPacketId
        );
        assert_matches!(
            sender
                .on_publish_received(foo)
                .err()
                .unwrap()
                .downcast_ref()
                .unwrap(),
            &InvalidPacketId
        );
        assert_matches!(
            sender
                .on_publish_complete(foo)
                .err()
                .unwrap()
                .downcast_ref()
                .unwrap(),
            &InvalidPacketId
        );
    }

    #[test]
    fn test_message_receiver() {
        let mut receiver = MessageReceiver::default();

        assert!(receiver.is_empty());

        assert_matches!(
            receiver.on_publish(
                false,
                false,
                QoS::AtMostOnce,
                None,
                Cow::from("topic"),
                Cow::from(&b"a"[..])
            ),
            Ok(None)
        );
        assert_matches!(
            receiver.on_publish(
                false,
                false,
                QoS::AtLeastOnce,
                Some(123),
                Cow::from("topic"),
                Cow::from(&b"b"[..])
            ),
            Ok(Some(123))
        );
        assert_matches!(
            receiver.on_publish(
                false,
                false,
                QoS::ExactlyOnce,
                Some(456),
                Cow::from("topic"),
                Cow::from(&b"c"[..])
            ),
            Ok(Some(456))
        );

        assert_eq!(receiver.len(), 1);
        assert!(receiver.get(&123).is_none());
        assert_matches!(receiver.get(&456), Some(&Message { .. }));

        assert_matches!(
            receiver
                .on_publish_release(123)
                .unwrap_err()
                .downcast_ref()
                .unwrap(),
            &InvalidPacketId
        );
        assert_matches!(receiver.on_publish_release(456), Ok(Message { .. }));

        assert!(receiver.is_empty());
    }
}
