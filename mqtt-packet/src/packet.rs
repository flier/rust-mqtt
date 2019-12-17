use num_enum::TryFromPrimitive;

use crate::mqtt::*;

/// The Protocol Name of the protocol.
pub const PROTOCOL_NAME: &[u8] = b"\x00\x04MQTT";

pub const RETURN_CODE_SUCCESS: u8 = 0x00;
pub const RETURN_CODE_FAILURE: u8 = 0x80;

/// MQTT Control Packets
#[derive(Clone, Debug, PartialEq)]
pub enum Packet<'a> {
    /// Client request to connect to Server
    Connect(Connect<'a>),
    /// Connect acknowledgment
    ConnectAck(ConnectAck<'a>),
    /// Publish message
    Publish(Publish<'a>),
    /// Publish acknowledgment
    PublishAck(PublishAck<'a>),
    /// Publish received (assured delivery part 1)
    PublishReceived(PublishReceived<'a>),
    /// Publish release (assured delivery part 2)
    PublishRelease(PublishRelease<'a>),
    /// Publish complete (assured delivery part 3)
    PublishComplete(PublishComplete<'a>),
    /// Client subscribe request
    Subscribe(Subscribe<'a>),
    /// Subscribe acknowledgment
    SubscribeAck(SubscribeAck<'a>),
    /// Unsubscribe request
    Unsubscribe(Unsubscribe<'a>),
    /// Unsubscribe acknowledgment
    UnsubscribeAck(UnsubscribeAck<'a>),
    /// PING request
    Ping,
    /// PING response
    Pong,
    /// Client is disconnecting
    Disconnect(Disconnect<'a>),
    /// Authentication exchange
    Auth(Auth<'a>),
}

/// Fixed Header
///
/// Each MQTT Control Packet contains a fixed header.
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Debug, Eq, PartialEq, Clone, Copy, TryFromPrimitive)]
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
    /// Disconnect notification
    DISCONNECT = 14,
    /// Authentication exchange
    AUTH = 15,
}

bitflags! {
    /// The Connect Flags byte contains a number of parameters specifying the behavior of the MQTT connection.
    /// It also indicates the presence or absence of fields in the payload.
    #[derive(Default)]
    pub struct ConnectFlags: u8 {
        /// This bit specifies a user name be present in the payload.
        const USERNAME      = 0b1000_0000;
        /// This bit specifies a password MUST be present in the payload.
        const PASSWORD      = 0b0100_0000;
        /// This bit specifies if the Will Message is to be Retained when it is published.
        const WILL_RETAIN   = 0b0010_0000;
        /// These two bits specify the QoS level to be used when publishing the Will Message.
        const WILL_QOS      = 0b0001_1000;
        /// If the Will Flag is set to 1 this indicates that, if the Connect request is accepted,
        /// a Will Message MUST be stored on the Server and associated with the Network Connection.
        /// The Will Message MUST be published when the Network Connection is subsequently closed
        /// unless the Will Message has been deleted by the Server on receipt of a DISCONNECT Packet [MQTT-3.1.2-8].
        const LAST_WILL     = 0b0000_0100;
        /// This bit specifies the handling of the Session state.
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

bitflags! {
    /// The Connect Acknowledge Flags.
    #[derive(Default)]
    pub struct ConnectAckFlags: u8 {
        /// The Session Present flag enables a Client to establish
        /// whether the Client and Server have a consistent view about whether there is already stored Session state.
        const SESSION_PRESENT = 0b0000_0001;
    }
}

bitflags! {
    /// Publish Flags
    #[derive(Default)]
    pub struct PublishFlags: u8 {
        /// This might be re-delivery of an earlier attempt to send the Packet.
        const DUP = 0b0000_1000;
        /// The level of assurance for delivery of an Application Message.
        const QOS = 0b0000_0110;
        /// It can be delivered to future subscribers whose subscriptions match its topic name
        const RETAIN = 0b0000_0001;
    }
}

const PUBLISH_DUP_SHIFT: usize = 1;

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

bitflags! {
    /// Subscription Options
    #[derive(Default)]
    pub struct SubscriptionOptions: u8 {
        /// Maximum QoS field. This gives the maximum QoS level at which the Server can send Application Messages to the Client.
        const QOS = 0b0000_0011;
        /// No Local option.
        const NL = 0b0000_0100;
        /// Retain As Published option.
        const RAP = 0b0000_1000;
        /// Retain Handling option.
        const RETAIN_HANDLING = 0b0011_0000;
    }
}

impl SubscriptionOptions {
    pub(crate) const RETAIN_HANDLING_SHIFT: usize = 4;
}

impl<'a> From<&'a Subscription<'a>> for SubscriptionOptions {
    fn from(subscription: &'a Subscription) -> Self {
        let mut opts = SubscriptionOptions::from_bits_truncate(
            subscription.qos as u8
                + ((subscription.retain_handling as u8)
                    << SubscriptionOptions::RETAIN_HANDLING_SHIFT),
        );

        if subscription.no_local {
            opts |= SubscriptionOptions::NL;
        }
        if subscription.retain_as_published {
            opts |= SubscriptionOptions::RAP;
        }

        opts
    }
}
