use std::borrow::Cow;
use std::rc::Rc;
use std::time::Duration;

use slab::Slab;

use core::*;
use error::*;
use transport::{self, Transport};

#[derive(Debug, PartialEq, Clone)]
pub struct Message<'a> {
    pub topic: Cow<'a, str>,
    pub payload: Cow<'a, [u8]>,
    pub qos: QoS,
}

#[derive(Debug, PartialEq, Clone)]
enum Waiting<'a> {
    PublishAck {
        packet_id: PacketId,
        msg: Rc<Message<'a>>,
    },
    PublishComplete {
        packet_id: PacketId,
    },
    SubscribeAck {
        packet_id: PacketId,
        topic_filters: &'a [(&'a str, QoS)],
    },
    UnsubscribeAck {
        packet_id: PacketId,
        topic_filters: &'a [&'a str],
    },
}

pub trait Handler {
    fn on_received_message(&mut self, msg: &Message);

    fn on_subscribed_topic(&mut self, topics: &[(&str, SubscribeReturnCode)]);

    fn on_unsubscribed_topic(&mut self, topics: &[&str]);
}

#[derive(Debug)]
pub struct Session<'a, H: 'a + Handler> {
    handler: &'a mut H,

    // QoS 1 and QoS 2 messages which have been sent to the Server,
    // but have not been completely acknowledged.
    waiting_reply: Slab<Waiting<'a>>,
}

impl<'a, H: Handler> Session<'a, H> {
    pub fn new(handler: &'a mut H) -> Self {
        Session {
            handler: handler,
            waiting_reply: Slab::with_capacity(256),
        }
    }

    pub fn reset(&mut self) {
        self.waiting_reply.clear();
    }

    pub fn connect(
        &'a mut self,
        client_id: &'a ClientId,
        clean_session: bool,
        keep_alive: u16,
        auth: Option<(&'a str, &'a [u8])>,
        last_will: Option<Rc<Message<'a>>>,
    ) -> Packet<'a> {
        Packet::Connect {
            protocol: Default::default(),
            clean_session: clean_session,
            keep_alive: keep_alive,
            last_will: last_will.map(|msg| LastWill {
                topic: msg.topic.clone().into(),
                message: msg.payload.clone().into(),
                qos: msg.qos,
                retain: false,
            }),
            client_id: client_id.as_ref().into(),
            username: auth.map(|(username, _)| username.into()),
            password: auth.map(|(_, password)| password.into()),
        }
    }

    fn delivery_retry(&mut self) -> Vec<Packet<'a>> {
        self.waiting_reply
            .iter()
            .map(|(_, waiting)| match *waiting {
                Waiting::PublishAck { packet_id, ref msg } => Packet::Publish {
                    dup: false,
                    retain: false,
                    qos: msg.qos,
                    topic: msg.topic.clone().into(),
                    packet_id: Some(packet_id),
                    payload: msg.payload.clone().into(),
                },
                Waiting::PublishComplete { packet_id } => Packet::PublishRelease {
                    packet_id: packet_id,
                },
                Waiting::SubscribeAck {
                    packet_id,
                    ref topic_filters,
                } => Packet::Subscribe {
                    packet_id: packet_id,
                    topic_filters: topic_filters
                        .iter()
                        .map(|&(filter, qos)| (filter.into(), qos))
                        .collect(),
                },
                Waiting::UnsubscribeAck {
                    packet_id,
                    ref topic_filters,
                } => Packet::Unsubscribe {
                    packet_id: packet_id,
                    topic_filters: topic_filters.iter().map(|&filter| filter.into()).collect(),
                },
            })
            .collect()
    }

    pub fn ping(&mut self) -> Packet<'a> {
        Packet::PingRequest
    }

    pub fn disconnect(&mut self) -> Packet<'a> {
        Packet::Disconnect
    }

    pub fn publish(&mut self, msg: Rc<Message<'a>>) -> Packet<'a> {
        Packet::Publish {
            dup: false,
            retain: false,
            qos: msg.qos,
            topic: msg.topic.clone().into(),
            packet_id: match msg.qos {
                QoS::AtLeastOnce | QoS::ExactlyOnce => Some(self.wait_reply(msg.clone())),
                _ => None,
            },
            payload: msg.payload.clone().into(),
        }
    }

    fn wait_reply(&mut self, msg: Rc<Message<'a>>) -> PacketId {
        let entry = self.waiting_reply.vacant_entry();
        let packet_id = entry.key() as PacketId;

        entry.insert(Waiting::PublishAck {
            packet_id: packet_id,
            msg: msg.clone(),
        });

        packet_id
    }

    fn on_publish_ack(&mut self, packet_id: PacketId) -> Option<Packet<'a>> {
        if self.waiting_reply.contains(packet_id as usize) {
            debug!("message {} acknowledged", packet_id);

            self.waiting_reply.remove(packet_id as usize);
        } else {
            warn!("unexpected packet id {}", packet_id);
        }

        None
    }

    fn on_publish_received(&mut self, packet_id: PacketId) -> Option<Packet<'a>> {
        if let Some(entry) = self.waiting_reply.get_mut(packet_id as usize) {
            debug!("message {} received at server side", packet_id);

            *entry = Waiting::PublishComplete {
                packet_id: packet_id,
            };

            Some(Packet::PublishRelease {
                packet_id: packet_id,
            })
        } else {
            warn!("unexpected packet id {}", packet_id);

            None
        }
    }

    fn on_publish_complete(&mut self, packet_id: PacketId) -> Option<Packet<'a>> {
        if self.waiting_reply.contains(packet_id as usize) {
            debug!("message {} completed", packet_id);

            self.waiting_reply.remove(packet_id as usize);
        } else {
            warn!("unexpected packet id {}", packet_id)
        }

        None
    }

    fn on_publish(
        &mut self,
        _dup: bool,
        _retain: bool,
        packet_id: Option<PacketId>,
        msg: Rc<Message<'a>>,
    ) -> Option<Packet<'a>> {
        self.handler.on_received_message(&msg);

        packet_id.and_then(|packet_id| self.send_reply(packet_id, msg))
    }

    fn send_reply(&mut self, packet_id: PacketId, msg: Rc<Message<'a>>) -> Option<Packet<'a>> {
        match msg.qos {
            QoS::AtLeastOnce => Some(Packet::PublishAck {
                packet_id: packet_id,
            }),
            QoS::ExactlyOnce => Some(Packet::PublishReceived {
                packet_id: packet_id,
            }),
            _ => None,
        }
    }

    fn on_publish_release(&mut self, packet_id: PacketId) -> Option<Packet<'a>> {
        Some(Packet::PublishComplete {
            packet_id: packet_id,
        })
    }

    pub fn subscribe(&mut self, topic_filters: &'a [(&'a str, QoS)]) -> Packet<'a> {
        let entry = self.waiting_reply.vacant_entry();
        let packet_id = entry.key() as PacketId;

        entry.insert(Waiting::SubscribeAck {
            packet_id: packet_id,
            topic_filters: topic_filters,
        });

        Packet::Subscribe {
            packet_id: packet_id,
            topic_filters: topic_filters
                .iter()
                .map(|&(filter, qos)| (filter.into(), qos))
                .collect(),
        }
    }

    fn on_subscribe_ack(&mut self, packet_id: PacketId, status: &[SubscribeReturnCode]) {
        if let Waiting::SubscribeAck { topic_filters, .. } =
            self.waiting_reply.remove(packet_id as usize)
        {
            debug!("subscribe {} acked", packet_id);

            let status = topic_filters
                .iter()
                .map(|&(topic, _)| topic)
                .zip(status.iter().map(|code| *code))
                .collect::<Vec<(&str, SubscribeReturnCode)>>();

            self.handler.on_subscribed_topic(status.as_slice());
        } else {
            warn!("unexpected packet id {}", packet_id);
        }
    }

    pub fn unsubscribe(&mut self, topic_filters: &'a [&'a str]) -> Packet<'a> {
        let entry = self.waiting_reply.vacant_entry();
        let packet_id = entry.key() as PacketId;

        entry.insert(Waiting::UnsubscribeAck {
            packet_id: packet_id,
            topic_filters: topic_filters,
        });

        Packet::Unsubscribe {
            packet_id: packet_id,
            topic_filters: topic_filters.iter().map(|&filter| filter.into()).collect(),
        }
    }

    fn on_unsubscribe_ack(&mut self, packet_id: PacketId) {
        if let Waiting::UnsubscribeAck { topic_filters, .. } =
            self.waiting_reply.remove(packet_id as usize)
        {
            debug!("unsubscribe {} acked", packet_id);

            self.handler.on_unsubscribed_topic(topic_filters)
        } else {
            warn!("unexpected packet id {}", packet_id)
        }
    }
}

pub struct Client<'a, T: Transport, H: 'a + Handler> {
    transport: T,
    session: Session<'a, H>,
    client_id: ClientId,
    keep_alive: Duration,
}

impl<'a, T: Transport, H: 'a + Handler> Client<'a, T, H> {
    pub fn close(&mut self) -> Result<()> {
        info!("client session closed");

        self.transport.close()
    }
}

impl<'a, T: Transport, H: 'a + Handler> transport::Handler<'a> for Client<'a, T, H> {
    fn on_received_packet(&mut self, packet: &Packet<'a>) {
        match *packet {
            Packet::ConnectAck {
                session_present,
                return_code,
            } => match return_code {
                ConnectReturnCode::ConnectionAccepted => {
                    info!(
                        "client session `{}` {}",
                        self.client_id,
                        if session_present {
                            "resumed"
                        } else {
                            "created"
                        }
                    );

                    for packet in self.session.delivery_retry() {
                        if let Err(err) = self.transport.send_packet(&packet) {
                            warn!("fail to send packet, {}", err)
                        }
                    }
                }
                _ => {
                    info!(
                        "client session `{}` refused, {}",
                        self.client_id,
                        return_code.reason()
                    );

                    if let Err(err) = self.close() {
                        warn!("fail to close session, {}", err)
                    }
                }
            },
            Packet::Publish {
                dup,
                retain,
                qos,
                ref topic,
                packet_id,
                ref payload,
            } => {
                self.session
                    .on_publish(
                        dup,
                        retain,
                        packet_id,
                        Rc::new(Message {
                            topic: topic.clone(),
                            payload: payload.clone(),
                            qos: qos,
                        }),
                    )
                    .and_then(|packet| self.transport.send_packet(&packet).ok());
            }
            Packet::PublishAck { packet_id } => {
                self.session
                    .on_publish_ack(packet_id)
                    .and_then(|packet| self.transport.send_packet(&packet).ok());
            }
            Packet::PublishReceived { packet_id } => {
                self.session
                    .on_publish_received(packet_id)
                    .and_then(|packet| self.transport.send_packet(&packet).ok());
            }
            Packet::PublishRelease { packet_id } => {
                self.session
                    .on_publish_release(packet_id)
                    .and_then(|packet| self.transport.send_packet(&packet).ok());
            }
            Packet::PublishComplete { packet_id } => {
                self.session
                    .on_publish_complete(packet_id)
                    .and_then(|packet| self.transport.send_packet(&packet).ok());
            }
            Packet::SubscribeAck {
                packet_id,
                ref status,
            } => {
                self.session.on_subscribe_ack(packet_id, status);
            }
            Packet::UnsubscribeAck { packet_id } => {
                self.session.on_unsubscribe_ack(packet_id);
            }
            Packet::PingResponse => {
                debug!("received ping response");
            }
            _ => {
                warn!("unexpected packet {:?}", packet.packet_type());
            }
        }
    }
}

pub struct Builder {
    client_id: ClientId,
    keep_alive: Duration,
}

impl Builder {
    pub fn client_id(mut self, client_id: ClientId) -> Self {
        self.client_id = client_id;
        self
    }

    pub fn keep_alive(mut self, keep_alive: Duration) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    pub fn build<'a, T: Transport, H: 'a + Handler>(
        self,
        transport: T,
        handler: &'a mut H,
    ) -> Client<'a, T, H> {
        Client {
            transport: transport,
            session: Session::new(handler),
            client_id: self.client_id,
            keep_alive: self.keep_alive,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            client_id: ClientId::new(),
            keep_alive: Duration::new(0, 0),
        }
    }
}
