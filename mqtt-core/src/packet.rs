use derive_more::Display;
use num_enum::{TryFromPrimitive, UnsafeFromPrimitive};

/// MQTT Control Packets
#[derive(Debug, PartialEq, Clone)]
pub enum Packet<'a> {
    /// Client request to connect to Server
    Connect(Connect<'a>),
    /// Connect acknowledgment
    ConnectAck(ConnectAck),
    /// Publish message
    Publish(Publish<'a>),
    /// Publish acknowledgment
    PublishAck(PublishAck),
    /// Publish received (assured delivery part 1)
    PublishReceived(PublishReceived),
    /// Publish release (assured delivery part 2)
    PublishRelease(PublishRelease),
    /// Publish complete (assured delivery part 3)
    PublishComplete(PublishComplete),
    /// Client subscribe request
    Subscribe(Subscribe<'a>),
    /// Subscribe acknowledgment
    SubscribeAck(SubscribeAck),
    /// Unsubscribe request
    Unsubscribe(Unsubscribe<'a>),
    /// Unsubscribe acknowledgment
    UnsubscribeAck(UnsubscribeAck),
    /// PING request
    Ping,
    /// PING response
    Pong,
    /// Client is disconnecting
    Disconnect,
}

/// Fixed Header
///
/// Each MQTT Control Packet contains a fixed header.
#[derive(Debug, PartialEq, Clone)]
pub struct FixedHeader {
    /// MQTT Control Packet type
    pub packet_type: Type,
    /// Flags specific to each MQTT Control Packet type
    pub packet_flags: u8,
    /// the number of bytes remaining within the current packet,
    /// including data in the variable header and the payload.
    pub remaining_length: usize,
}

/// MQTT Control Packet type
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, TryFromPrimitive)]
pub enum Type {
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
}

pub const PROTOCOL_NAME: &[u8] = b"\x00\x04MQTT";
pub const PROTOCOL_LEVEL: u8 = 4;

/// Quality of Service levels
#[repr(u8)]
#[derive(
    Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, TryFromPrimitive, UnsafeFromPrimitive,
)]
pub enum QoS {
    /// At most once delivery
    ///
    /// The message is delivered according to the capabilities of the underlying network.
    /// No response is sent by the receiver and no retry is performed by the sender.
    /// The message arrives at the receiver either once or not at all.
    AtMostOnce = 0,

    /// At least once delivery
    ///
    /// This quality of service ensures that the message arrives at the receiver at least once.
    /// A QoS 1 PUBLISH Packet has a Packet Identifier in its variable header
    /// and is acknowledged by a PUBACK Packet.
    AtLeastOnce = 1,

    /// Exactly once delivery
    ///
    /// This is the highest quality of service,
    /// for use when neither loss nor duplication of messages are acceptable.
    /// There is an increased overhead associated with this quality of service.
    ExactlyOnce = 2,
}

/// Client request to connect to Server
#[derive(Debug, PartialEq, Clone)]
pub struct Connect<'a> {
    /// the handling of the Session state.
    pub clean_session: bool,
    /// a time interval measured in seconds.
    pub keep_alive: u16,
    /// identifies the Client to the Server.
    pub client_id: &'a str,
    /// Will Message be stored on the Server and associated with the Network Connection.
    pub last_will: Option<LastWill<'a>>,
    /// username can be used by the Server for authentication and authorization.
    pub username: Option<&'a str>,
    /// password can be used by the Server for authentication and authorization.
    pub password: Option<&'a [u8]>,
}

bitflags! {
    /// Connect Flags
    #[derive(Default)]
    pub struct ConnectFlags: u8 {
        const USERNAME      = 0b1000_0000;
        const PASSWORD      = 0b0100_0000;
        const WILL_RETAIN   = 0b0010_0000;
        const WILL_QOS      = 0b0001_1000;
        const LAST_WILL     = 0b0000_0100;
        const CLEAN_SESSION = 0b0000_0010;
    }
}

const WILL_QOS_SHIFT: usize = 3;

impl ConnectFlags {
    /// the QoS level to be used when publishing the Will Message.
    pub fn qos(self) -> QoS {
        unsafe { QoS::from_unchecked((self & Self::WILL_QOS).bits() >> WILL_QOS_SHIFT) }
    }
}

impl From<QoS> for ConnectFlags {
    fn from(qos: QoS) -> Self {
        Self::from_bits_truncate((qos as u8) << WILL_QOS_SHIFT)
    }
}

/// Connection Will
#[derive(Debug, PartialEq, Hash, Clone)]
pub struct LastWill<'a> {
    /// the QoS level to be used when publishing the Will Message.
    pub qos: QoS,
    /// the Will Message is to be Retained when it is published.
    pub retain: bool,
    /// the Will Topic
    pub topic: &'a str,
    /// defines the Application Message that is to be published to the Will Topic
    pub message: &'a [u8],
}

/// Connect acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct ConnectAck {
    /// enables a Client to establish whether the Client and Server have a consistent view
    /// about whether there is already stored Session state.
    pub session_present: bool,
    /// If a well formed CONNECT Packet is received by the Server,
    /// but the Server is unable to process it for some reason,
    /// then the Server SHOULD attempt to send a CONNACK packet
    /// containing the appropriate non-zero Connect return code from this table.
    pub return_code: ConnectReturnCode,
}

bitflags! {
    /// ConnectAck Flags
    #[derive(Default)]
    pub struct ConnectAckFlags: u8 {
        const SESSION_PRESENT = 0b0000_0001;
    }
}

/// Connect Return Code
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, TryFromPrimitive, Display)]
pub enum ConnectReturnCode {
    /// Connection accepted
    #[display(fmt = "Connection Accepted")]
    ConnectionAccepted = 0,
    /// Connection Refused, unacceptable protocol version
    #[display(fmt = "Connection Refused, unacceptable protocol version")]
    UnacceptableProtocolVersion = 1,
    /// Connection Refused, identifier rejected
    #[display(fmt = "Connection Refused, identifier rejected")]
    IdentifierRejected = 2,
    /// Connection Refused, Server unavailable
    #[display(fmt = "Connection Refused, Server unavailable")]
    ServiceUnavailable = 3,
    /// Connection Refused, bad user name or password
    #[display(fmt = "Connection Refused, bad user name or password")]
    BadUserNameOrPassword = 4,
    /// Connection Refused, not authorized
    #[display(fmt = "Connection Refused, not authorized")]
    NotAuthorized = 5,
}

/// Packet Identifier
///
/// The variable header component of many of the Control Packet types includes a 2 byte Packet Identifier field.
pub type PacketId = u16;

/// Publish message
#[derive(Debug, PartialEq, Clone)]
pub struct Publish<'a> {
    /// If the DUP flag is set to 0, it indicates that this is the first occasion
    /// that the Client or Server has attempted to send this MQTT PUBLISH Packet.
    /// If the DUP flag is set to 1, it indicates that this might be re-delivery of
    /// an earlier attempt to send the Packet.
    pub dup: bool,
    /// The level of assurance for delivery of an Application Message.
    pub qos: QoS,
    /// If the RETAIN flag is set to 1, in a PUBLISH Packet sent by a Client to a Server,
    /// the Server MUST store the Application Message and its QoS,
    /// so that it can be delivered to future subscribers whose subscriptions match its topic name [MQTT-3.3.1-5].
    pub retain: bool,
    /// the information channel to which payload data is published.
    pub topic: &'a str,
    /// only present in PUBLISH Packets where the QoS level is 1 or 2.
    pub packet_id: Option<PacketId>,
    /// the Application Message that is being published.
    pub payload: &'a [u8],
}

bitflags! {
    /// Publish Flags
    #[derive(Default)]
    pub struct PublishFlags: u8 {
        const DUP = 0b0000_1000;
        const QOS = 0b0000_0110;
        const RETAIN = 0b0000_0001;
    }
}

const PUBLISH_DUP_SHIFT: usize = 1;

impl Publish<'_> {
    pub fn flags(&self) -> PublishFlags {
        let mut flags = PublishFlags::from(self.qos);
        if self.dup {
            flags |= PublishFlags::DUP;
        }
        if self.retain {
            flags |= PublishFlags::RETAIN;
        }
        flags
    }
}

impl PublishFlags {
    /// the QoS level to be used when publishing the Will Message.
    pub fn qos(self) -> QoS {
        unsafe { QoS::from_unchecked((self & Self::QOS).bits() >> PUBLISH_DUP_SHIFT) }
    }
}

impl From<QoS> for PublishFlags {
    fn from(qos: QoS) -> Self {
        Self::from_bits_truncate((qos as u8) << PUBLISH_DUP_SHIFT)
    }
}

/// Publish acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct PublishAck {
    /// Packet Identifier
    pub packet_id: PacketId,
}

/// Publish received (assured delivery part 1)
#[derive(Debug, PartialEq, Clone)]
pub struct PublishReceived {
    /// Packet Identifier
    pub packet_id: PacketId,
}

/// Publish release (assured delivery part 2)
#[derive(Debug, PartialEq, Clone)]
pub struct PublishRelease {
    /// Packet Identifier
    pub packet_id: PacketId,
}

/// Publish complete (assured delivery part 3)
#[derive(Debug, PartialEq, Clone)]
pub struct PublishComplete {
    /// Packet Identifier
    pub packet_id: PacketId,
}

/// Client subscribe request
#[derive(Debug, PartialEq, Clone)]
pub struct Subscribe<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// the list of Topic Filters and QoS to which the Client wants to subscribe.
    pub subscriptions: Vec<(&'a str, QoS)>,
}

/// Subscribe acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct SubscribeAck {
    pub packet_id: PacketId,
    /// corresponds to a Topic Filter in the SUBSCRIBE Packet being acknowledged.
    pub status: Vec<SubscribeReturnCode>,
}

/// Subscribe Return Code
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SubscribeReturnCode {
    Success(QoS),
    Failure,
}

impl From<SubscribeReturnCode> for u8 {
    fn from(code: SubscribeReturnCode) -> u8 {
        match code {
            SubscribeReturnCode::Success(qos) => qos as u8,
            SubscribeReturnCode::Failure => SubscribeAck::FAILURE,
        }
    }
}

/// Unsubscribe request
#[derive(Debug, PartialEq, Clone)]
pub struct Unsubscribe<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// the list of Topic Filters that the Client wishes to unsubscribe from.
    pub topic_filters: Vec<&'a str>,
}

/// Unsubscribe acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct UnsubscribeAck {
    /// Packet Identifier
    pub packet_id: PacketId,
}
