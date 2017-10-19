use std::borrow::Cow;

use proto::{Protocol, QoS};

bitflags! {
    #[doc="Connect Flags"]
    pub struct ConnectFlags: u8 {
        const USERNAME      = 0b1000_0000;
        const PASSWORD      = 0b0100_0000;
        const WILL_RETAIN   = 0b0010_0000;
        const WILL_QOS      = 0b0001_1000;
        const WILL          = 0b0000_0100;
        const CLEAN_SESSION = 0b0000_0010;
    }
}

pub const WILL_QOS_SHIFT: u8 = 3;

bitflags! {
    #[doc="Connect Acknowledge Flags"]
    pub struct ConnectAckFlags: u8 {
        const SESSION_PRESENT = 0b0000_0001;
    }
}

/// Connect Return Code
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ConnectReturnCode {
    /// Connection accepted
    ConnectionAccepted = 0,
    /// Connection Refused, unacceptable protocol version
    UnacceptableProtocolVersion = 1,
    /// Connection Refused, identifier rejected
    IdentifierRejected = 2,
    /// Connection Refused, Server unavailable
    ServiceUnavailable = 3,
    /// Connection Refused, bad user name or password
    BadUserNameOrPassword = 4,
    /// Connection Refused, not authorized
    NotAuthorized = 5,
    /// Reserved
    Reserved = 6,
}

const_enum!(ConnectReturnCode: u8);

impl ConnectReturnCode {
    pub fn reason(&self) -> &'static str {
        match *self {
            ConnectReturnCode::ConnectionAccepted => "Connection Accepted",
            ConnectReturnCode::UnacceptableProtocolVersion => {
                "Connection Refused, unacceptable protocol version"
            }
            ConnectReturnCode::IdentifierRejected => "Connection Refused, identifier rejected",
            ConnectReturnCode::ServiceUnavailable => "Connection Refused, Server unavailable",
            ConnectReturnCode::BadUserNameOrPassword => {
                "Connection Refused, bad user name or password"
            }
            ConnectReturnCode::NotAuthorized => "Connection Refused, not authorized",
            _ => "Connection Refused",
        }
    }
}

/// Fixed Header
///
/// Each MQTT Control Packet contains a fixed header.
#[derive(Debug, PartialEq, Clone)]
pub struct FixedHeader {
    /// MQTT Control Packet type
    pub packet_type: PacketType,
    /// Flags specific to each MQTT Control Packet type
    pub packet_flags: u8,
    /// the number of bytes remaining within the current packet,
    /// including data in the variable header and the payload.
    pub remaining_length: usize,
}

/// Connection Will
#[derive(Debug, PartialEq, Clone)]
pub struct LastWill<'a> {
    /// the QoS level to be used when publishing the Will Message.
    pub qos: QoS,
    /// the Will Message is to be Retained when it is published.
    pub retain: bool,
    /// the Will Topic
    pub topic: Cow<'a, str>,
    /// defines the Application Message that is to be published to the Will Topic
    pub message: Cow<'a, [u8]>,
}

impl<'a> LastWill<'a> {
    pub fn into_owned(self) ->LastWill<'static> {
        LastWill {
            qos: self.qos,
            retain: self.retain,
            topic: self.topic.into_owned().into(),
            message: self.message.into_owned().into()
        }
    }
}

/// Subscribe Return Code
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SubscribeReturnCode {
    Success(QoS),
    Failure,
}

/// Packet Identifier
///
/// The variable header component of many of the Control Packet types includes a 2 byte Packet Identifier field.
pub type PacketId = u16;

/// MQTT Control Packets
#[derive(Debug, PartialEq, Clone)]
pub enum Packet<'a> {
    /// Client request to connect to Server
    Connect {
        protocol: Protocol,
        /// the handling of the Session state.
        clean_session: bool,
        /// a time interval measured in seconds.
        keep_alive: u16,
        /// Will Message be stored on the Server and associated with the Network Connection.
        last_will: Option<LastWill<'a>>,
        /// identifies the Client to the Server.
        client_id: Cow<'a, str>,
        /// username can be used by the Server for authentication and authorization.
        username: Option<Cow<'a, str>>,
        /// password can be used by the Server for authentication and authorization.
        password: Option<Cow<'a, [u8]>>,
    },
    /// Connect acknowledgment
    ConnectAck {
        /// enables a Client to establish whether the Client and Server have a consistent view
        /// about whether there is already stored Session state.
        session_present: bool,
        /// If a well formed CONNECT Packet is received by the Server,
        /// but the Server is unable to process it for some reason,
        /// then the Server SHOULD attempt to send a CONNACK packet
        /// containing the appropriate non-zero Connect return code from this table.
        return_code: ConnectReturnCode,
    },
    /// Publish message
    Publish {
        /// this might be re-delivery of an earlier attempt to send the Packet.
        dup: bool,
        retain: bool,
        /// the level of assurance for delivery of an Application Message.
        qos: QoS,
        /// the information channel to which payload data is published.
        topic: Cow<'a, str>,
        /// only present in PUBLISH Packets where the QoS level is 1 or 2.
        packet_id: Option<u16>,
        /// the Application Message that is being published.
        payload: Cow<'a, [u8]>,
    },
    /// Publish acknowledgment
    PublishAck {
        /// Packet Identifier
        packet_id: PacketId,
    },
    /// Publish received (assured delivery part 1)
    PublishReceived {
        /// Packet Identifier
        packet_id: PacketId,
    },
    /// Publish release (assured delivery part 2)
    PublishRelease {
        /// Packet Identifier
        packet_id: PacketId,
    },
    /// Publish complete (assured delivery part 3)
    PublishComplete {
        /// Packet Identifier
        packet_id: PacketId,
    },
    /// Client subscribe request
    Subscribe {
        /// Packet Identifier
        packet_id: PacketId,
        /// the list of Topic Filters and QoS to which the Client wants to subscribe.
        topic_filters: Vec<(Cow<'a, str>, QoS)>,
    },
    /// Subscribe acknowledgment
    SubscribeAck {
        packet_id: PacketId,
        /// corresponds to a Topic Filter in the SUBSCRIBE Packet being acknowledged.
        status: Vec<SubscribeReturnCode>,
    },
    /// Unsubscribe request
    Unsubscribe {
        /// Packet Identifier
        packet_id: PacketId,
        /// the list of Topic Filters that the Client wishes to unsubscribe from.
        topic_filters: Vec<Cow<'a, str>>,
    },
    /// Unsubscribe acknowledgment
    UnsubscribeAck {
        /// Packet Identifier
        packet_id: PacketId,
    },
    /// PING request
    PingRequest,
    /// PING response
    PingResponse,
    /// Client is disconnecting
    Disconnect,
}

impl<'a> Packet<'a> {
    #[inline]
    /// MQTT Control Packet type
    pub fn packet_type(&self) -> PacketType {
        match *self {
            Packet::Connect { .. } => PacketType::CONNECT,
            Packet::ConnectAck { .. } => PacketType::CONNACK,
            Packet::Publish { .. } => PacketType::PUBLISH,
            Packet::PublishAck { .. } => PacketType::PUBACK,
            Packet::PublishReceived { .. } => PacketType::PUBREC,
            Packet::PublishRelease { .. } => PacketType::PUBREL,
            Packet::PublishComplete { .. } => PacketType::PUBCOMP,
            Packet::Subscribe { .. } => PacketType::SUBSCRIBE,
            Packet::SubscribeAck { .. } => PacketType::SUBACK,
            Packet::Unsubscribe { .. } => PacketType::UNSUBSCRIBE,
            Packet::UnsubscribeAck { .. } => PacketType::UNSUBACK,
            Packet::PingRequest => PacketType::PINGREQ,
            Packet::PingResponse => PacketType::PINGRESP,
            Packet::Disconnect => PacketType::DISCONNECT,
        }
    }

    /// Flags specific to each MQTT Control Packet type
    pub fn packet_flags(&self) -> u8 {
        match *self {
            Packet::Publish { dup, qos, retain, .. } => {
                let mut b = qos.into();

                b <<= 1;

                if dup {
                    b |= 0b1000;
                }

                if retain {
                    b |= 0b0001;
                }

                b
            }
            Packet::PublishRelease { .. } |
            Packet::Subscribe { .. } |
            Packet::Unsubscribe { .. } => 0b0010,
            _ => 0,
        }
    }

    pub fn packet_id(&self) -> Option<PacketId> {
        match *self {
            Packet::PublishAck { packet_id } |
            Packet::PublishReceived { packet_id } |
            Packet::PublishRelease { packet_id } |
            Packet::PublishComplete { packet_id } |
            Packet::Subscribe { packet_id, .. } |
            Packet::SubscribeAck { packet_id, .. } |
            Packet::Unsubscribe { packet_id, .. } |
            Packet::UnsubscribeAck { packet_id } => Some(packet_id),
            _ => None,
        }
    }

    pub fn into_owned(self) -> Packet<'static> {
        match self {
            Packet::Connect {
                protocol,
                clean_session,
                keep_alive,
                last_will,
                client_id,
                username,
                password,
            } => Packet::Connect {
                protocol,
                clean_session,
                keep_alive,
                last_will: last_will.map(|last_will| {
                    LastWill {
                        qos: last_will.qos,
                        retain: last_will.retain,
                        topic: last_will.topic.into_owned().into(),
                        message: last_will.message.into_owned().into(),
                    }
                }),
                client_id: client_id.into_owned().into(),
                username: username.map(|o| o.into_owned().into()),
                password: password.map(|o| o.into_owned().into()),
            },
            Packet::ConnectAck {
                session_present,
                return_code,
            } => Packet::ConnectAck {
                session_present,
                return_code,
            },
            Packet::Publish {
                dup,
                retain,
                qos,
                topic,
                packet_id,
                payload,
            } => Packet::Publish {
                dup,
                retain,
                qos,
                topic: topic.into_owned().into(),
                packet_id: packet_id,
                payload: payload.into_owned().into(),
            },
            Packet::PublishAck { packet_id } => Packet::PublishAck { packet_id },
            Packet::PublishReceived { packet_id } => Packet::PublishReceived { packet_id },
            Packet::PublishRelease { packet_id } => Packet::PublishRelease { packet_id },
            Packet::PublishComplete { packet_id } => Packet::PublishComplete { packet_id },
            Packet::Subscribe {
                packet_id,
                topic_filters,
            } => Packet::Subscribe {
                packet_id,
                topic_filters: topic_filters
                    .into_iter()
                    .map(|(filter, qos)| (filter.into_owned().into(), qos))
                    .collect(),
            },
            Packet::SubscribeAck { packet_id, status } => Packet::SubscribeAck {
                packet_id,
                status,
            },
            Packet::Unsubscribe {
                packet_id,
                topic_filters,
            } => Packet::Unsubscribe {
                packet_id,
                topic_filters: topic_filters
                    .into_iter()
                    .map(|filter| filter.into_owned().into())
                    .collect(),
            },
            Packet::UnsubscribeAck { packet_id } => Packet::UnsubscribeAck { packet_id },
            Packet::PingRequest => Packet::PingRequest,
            Packet::PingResponse => Packet::PingResponse,
            Packet::Disconnect => Packet::Disconnect,
        }
    }
}

/// MQTT Control Packet type
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum PacketType {
    RESERVED = 0,
    /// Client request to connect to Server
    CONNECT = 1,
    /// Connect acknowledgment
    CONNACK = 2,
    /// Publish message
    PUBLISH = 3,
    /// Publish acknowledgment
    PUBACK = 4,
    /// Publish received (assured delivery part 1)
    PUBREC = 5,
    /// Publish release (assured delivery part 2)
    PUBREL = 6,
    /// Publish complete (assured delivery part 3)
    PUBCOMP = 7,
    /// Client subscribe request
    SUBSCRIBE = 8,
    /// Subscribe acknowledgment
    SUBACK = 9,
    /// Unsubscribe request
    UNSUBSCRIBE = 10,
    /// Unsubscribe acknowledgment
    UNSUBACK = 11,
    /// PING request
    PINGREQ = 12,
    /// PING response
    PINGRESP = 13,
    /// Client is disconnecting
    DISCONNECT = 14,
    /// Reserved
    RESERVED15 = 15,
}

const_enum!(PacketType: u8);
