use std::error::Error as StdError;

use derive_more::Display;
use num_enum::{TryFromPrimitive, UnsafeFromPrimitive};

use crate::Property;

/// The revision level of the protocol used by the Client.
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy, TryFromPrimitive)]
pub enum ProtocolVersion {
    /// The value of the Protocol Level field for the version 3.1.1 of the protocol is 4 (0x04).
    V311 = 4,
    /// The value of the Protocol Version field for version 5.0 of the protocol is 5 (0x05).
    V5 = 5,
}

/// The result of an operation
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Clone, Copy, TryFromPrimitive)]
pub enum ReasonCode {
    /// Granted QoS 0 [SUBACK]
    GrantedQoS0 = 0,
    /// Granted QoS 1 [SUBACK]
    GrantedQoS1 = 1,
    /// Granted QoS 2 [SUBACK]
    GrantedQoS2 = 2,
    /// Disconnect with Will Message [DISCONNECT]
    DisconnectWithWill = 0x04,
    /// No matching subscribers [PUBACK, PUBREC]
    NoMatchingSubscribers = 0x10,
    /// No subscription existed [UNSUBACK]
    NoSubscriptionExisted = 0x11,
    /// Continue authentication [AUTH]
    ContinueAuthentication = 0x18,
    /// Re-authenticate [AUTH]
    Reauthenticate = 0x19,
    /// Unspecified error [CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT]
    UnspecifiedError = 0x80,
    /// Malformed Packet [CONNACK, DISCONNECT]
    MalformedPacket = 0x81,
    /// Protocol Error [CONNACK, DISCONNECT]
    ProtocolError = 0x82,
    /// Implementation specific error [CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT]
    ImplementationSpecificError = 0x83,
    /// Unsupported Protocol Version [CONNACK]
    UnsupportedProtocolVersion = 0x84,
    /// Client Identifier not valid [CONNACK]
    ClientIdNotValid = 0x85,
    /// Bad User Name or Password [CONNACK]
    BadUserNameOrPassword = 0x86,
    /// Not authorized [CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT]
    NotAuthorized = 0x87,
    /// Server unavailable [CONNACK]
    ServerUnavailable = 0x88,
    /// Server busy [CONNACK, DISCONNECT]
    ServerBusy = 0x89,
    /// Banned [CONNACK]
    Banned = 0x8A,
    /// Server shutting down [DISCONNECT]
    ServerShuttingDown = 0x8B,
    /// Bad authentication method [CONNACK, DISCONNECT]
    BadAuthenticationMethod = 0x8C,
    /// Keep Alive timeout [DISCONNECT]
    KeepAliveTimeout = 0x8D,
    /// Session taken over [DISCONNECT]
    SessionTakenOver = 0x8E,
    /// Topic Filter invalid [SUBACK, UNSUBACK, DISCONNECT]
    InvalidTopicFilter = 0x8F,
    /// Topic Name invalid [CONNACK, PUBACK, PUBREC, DISCONNECT]
    InvalidTopicName = 0x90,
    /// Packet Identifier in use [PUBACK, PUBREC, SUBACK, UNSUBACK]
    PacketIdInUse = 0x91,
    /// Packet Identifier not found [PUBREL, PUBCOMP]
    PacketIdNotFound = 0x92,
    /// Receive Maximum exceeded [DISCONNECT]
    ReceiveMaximumExceeded = 0x93,
    /// Topic Alias invalid [DISCONNECT]
    InvalidTopicAlias = 0x94,
    /// Packet too large [CONNACK, DISCONNECT]
    PacketTooLarge = 0x95,
    /// Message rate too high [DISCONNECT]
    MessageRateTooHigh = 0x96,
    /// Quota exceeded [CONNACK, PUBACK, PUBREC, SUBACK, DISCONNECT]
    QuotaExceeded = 0x97,
    /// Administrative action [DISCONNECT]
    AdministrativeAction = 0x98,
    /// Payload format invalid [CONNACK, PUBACK, PUBREC, DISCONNECT]
    InvalidPayloadFormat = 0x99,
    /// Retain not supported [CONNACK, DISCONNECT]
    RetainNotSupported = 0x9A,
    /// QoS not supported [CONNACK, DISCONNECT]
    QoSNotSupported = 0x9B,
    /// Use another server [CONNACK, DISCONNECT]
    UseAnotherServer = 0x9C,
    /// Server moved [CONNACK, DISCONNECT]
    ServerMoved = 0x9D,
    /// Shared Subscriptions not supported [SUBACK, DISCONNECT]
    SharedSubscriptionsNotSupported = 0x9E,
    /// Connection rate exceeded [CONNACK, DISCONNECT]
    ConnectionRateExceeded = 0x9F,
    /// Maximum connect time [DISCONNECT]
    MaximumConnectTime = 0xA0,
    /// Subscription Identifiers not supported [SUBACK, DISCONNECT]
    SubscriptionIdNotSupported = 0xA1,
    /// Wildcard Subscriptions not supported [SUBACK, DISCONNECT]
    WildcardSubscriptionsNotSupported = 0xA2,
}

impl Default for ReasonCode {
    fn default() -> Self {
        ReasonCode::Success
    }
}

#[allow(non_upper_case_globals)]
impl ReasonCode {
    /// Success [CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, UNSUBACK, AUTH]
    pub const Success: Self = Self::GrantedQoS0;

    /// Normal disconnection [DISCONNECT]
    pub const NormalDisconnection: Self = Self::Success;
}

/// Quality of Service levels
#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    TryFromPrimitive,
    UnsafeFromPrimitive,
)]
pub enum QoS {
    /// At most once delivery
    ///
    /// The message is delivered according to the capabilities of the underlying network.
    /// No response is sent by the receiver and no retry is performed by the sender.
    /// The message arrives at the receiver either once or not at all.
    #[display(fmt = "at-most-once")]
    AtMostOnce = 0,

    /// At least once delivery
    ///
    /// This quality of service ensures that the message arrives at the receiver at least once.
    /// A QoS 1 PUBLISH Packet has a Packet Identifier in its variable header
    /// and is acknowledged by a PUBACK Packet.
    #[display(fmt = "at-least-once")]
    AtLeastOnce = 1,

    /// Exactly once delivery
    ///
    /// This is the highest quality of service,
    /// for use when neither loss nor duplication of messages are acceptable.
    /// There is an increased overhead associated with this quality of service.
    #[display(fmt = "exactly-once")]
    ExactlyOnce = 2,
}

impl Default for QoS {
    fn default() -> Self {
        QoS::AtMostOnce
    }
}

/// Client request to connect to Server
#[derive(Clone, Debug, PartialEq)]
pub struct Connect<'a> {
    /// the revision level of the protocol used by the Client.
    pub protocol_version: ProtocolVersion,
    /// the handling of the Session state.
    pub clean_session: bool,
    /// a time interval measured in seconds.
    pub keep_alive: u16,
    /// Connect properties
    pub properties: Option<Vec<Property<'a>>>,
    /// identifies the Client to the Server.
    pub client_id: &'a str,
    /// Will Message be stored on the Server and associated with the Network Connection.
    pub last_will: Option<LastWill<'a>>,
    /// username can be used by the Server for authentication and authorization.
    pub username: Option<&'a str>,
    /// password can be used by the Server for authentication and authorization.
    pub password: Option<&'a [u8]>,
}

/// Connection Will
#[derive(Clone, Debug, PartialEq)]
pub struct LastWill<'a> {
    /// the QoS level to be used when publishing the Will Message.
    pub qos: QoS,
    /// the Will Message is to be Retained when it is published.
    pub retain: bool,
    /// the Will Topic
    pub topic_name: &'a str,
    /// defines the Application Message that is to be published to the Will Topic
    pub message: &'a [u8],
    /// Will properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Connect acknowledgment
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectAck<'a> {
    /// The Session Present flag enables a Client to establish
    /// whether the Client and Server have a consistent view about whether there is already stored Session state.
    pub session_present: bool,
    /// If a well formed CONNECT Packet is received by the Server,
    /// but the Server is unable to process it for some reason,
    /// then the Server SHOULD attempt to send a CONNACK packet
    /// containing the appropriate non-zero Connect return code from this table.
    pub return_code: ConnectReturnCode,
    /// ConnectAck properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Connect Return Code
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Clone, Copy, TryFromPrimitive, Display)]
pub enum ConnectReturnCode {
    /// Connection accepted
    #[display(fmt = "Connection Accepted")]
    ConnectionAccepted = 0,
    /// The Server does not support the level of the MQTT protocol requested by the Client
    #[display(fmt = "Connection Refused, unacceptable protocol version")]
    UnacceptableProtocolVersion = 1,
    /// The Client identifier is correct UTF-8 but not allowed by the Server
    #[display(fmt = "Connection Refused, identifier rejected")]
    ClientIdRejected = 2,
    /// The Network Connection has been made but the MQTT service is unavailable
    #[display(fmt = "Connection Refused, Server unavailable")]
    ServiceUnavailable = 3,
    /// The data in the user name or password is malformed
    #[display(fmt = "Connection Refused, bad user name or password")]
    BadUserNameOrPassword = 4,
    /// The Client is not authorized to connect
    #[display(fmt = "Connection Refused, not authorized")]
    NotAuthorized = 5,
}

impl StdError for ConnectReturnCode {}

impl ConnectReturnCode {
    /// Transforms the `ConnectReturnCode` into a `Result<(), ConnectReturnCode>`.
    pub fn ok(self) -> Result<(), Self> {
        if self == ConnectReturnCode::ConnectionAccepted {
            Ok(())
        } else {
            Err(self)
        }
    }
}

/// Packet Identifier
///
/// The variable header component of many of the Control Packet types includes a 2 byte Packet Identifier field.
pub type PacketId = u16;

/// Publish message
#[derive(Clone, Debug, PartialEq)]
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
    pub topic_name: &'a str,
    /// only present in PUBLISH Packets where the QoS level is 1 or 2.
    pub packet_id: Option<PacketId>,
    /// Publish properties
    pub properties: Option<Vec<Property<'a>>>,
    /// the Application Message that is being published.
    pub payload: &'a [u8],
}

/// Publish acknowledgment
#[derive(Clone, Debug, PartialEq)]
pub struct PublishAck<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishAck properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Publish received (assured delivery part 1)
#[derive(Clone, Debug, PartialEq)]
pub struct PublishReceived<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishReceived properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Publish release (assured delivery part 2)
#[derive(Clone, Debug, PartialEq)]
pub struct PublishRelease<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishRelease properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Publish complete (assured delivery part 3)
#[derive(Clone, Debug, PartialEq)]
pub struct PublishComplete<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishComplete properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Client subscribe request
#[derive(Clone, Debug, PartialEq)]
pub struct Subscribe<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Subscribe properties
    pub properties: Option<Vec<Property<'a>>>,
    /// the list of Topic Filters and QoS to which the Client wants to subscribe.
    pub subscriptions: Vec<Subscription<'a>>,
}

/// MQTT Subscription
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Subscription<'a> {
    /// Topic Filter indicating the Topics to which the Client wants to subscribe.
    pub topic_filter: &'a str,
    /// Maximum QoS
    pub qos: QoS,
    /// No Local option.
    ///
    /// If the value is true, Application Messages MUST NOT be forwarded to a connection
    /// with a ClientID equal to the ClientID of the publishing connection [MQTT-3.8.3-3].
    /// It is a Protocol Error to set the No Local bit to 1 on a Shared Subscription [MQTT-3.8.3-4].
    pub no_local: bool,
    /// If true, Application Messages forwarded using this subscription keep the RETAIN flag they were published with.
    /// If false, Application Messages forwarded using this subscription have the RETAIN flag set to 0.
    /// Retained messages sent when the subscription is established have the RETAIN flag set to 1.
    pub retain_as_published: bool,
    /// This option specifies whether retained messages are sent when the subscription is established.
    ///
    /// This does not affect the sending of retained messages at any point after the subscribe.
    /// If there are no retained messages matching the Topic Filter, all of these values act the same.
    pub retain_handling: RetainHandling,
}

impl<'a> From<&'a str> for Subscription<'a> {
    fn from(topic_filter: &'a str) -> Subscription<'a> {
        Subscription {
            topic_filter,
            ..Default::default()
        }
    }
}

impl<'a> From<(&'a str, QoS)> for Subscription<'a> {
    fn from((topic_filter, qos): (&'a str, QoS)) -> Subscription<'a> {
        Subscription {
            topic_filter,
            qos,
            ..Default::default()
        }
    }
}

/// Subscribe Return Code
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
pub enum RetainHandling {
    /// Send retained messages at the time of the subscribe
    AfterSubscribe = 0,
    /// Send retained messages at subscribe only if the subscription does not currently exist
    NewSubscription = 1,
    /// Do not send retained messages at the time of the subscribe
    SkipSubscribe = 2,
}

impl Default for RetainHandling {
    fn default() -> Self {
        RetainHandling::AfterSubscribe
    }
}

/// Subscribe acknowledgment
#[derive(Clone, Debug, PartialEq)]
pub struct SubscribeAck<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// SubscribeAck properties
    pub properties: Option<Vec<Property<'a>>>,
    /// corresponds to a Topic Filter in the SUBSCRIBE Packet being acknowledged.
    pub status: Vec<SubscribeReturnCode>,
}

/// Subscribe Return Code
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SubscribeReturnCode {
    /// Success
    Success(QoS),
    /// Failure
    Failure,
}

impl SubscribeReturnCode {
    pub const FAILURE: u8 = 0x80;
}

impl From<SubscribeReturnCode> for u8 {
    fn from(code: SubscribeReturnCode) -> u8 {
        match code {
            SubscribeReturnCode::Success(qos) => qos as u8,
            SubscribeReturnCode::Failure => SubscribeReturnCode::FAILURE,
        }
    }
}

/// Unsubscribe request
#[derive(Clone, Debug, PartialEq)]
pub struct Unsubscribe<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Unsubscribe properties
    pub properties: Option<Vec<Property<'a>>>,
    /// the list of Topic Filters that the Client wishes to unsubscribe from.
    pub topic_filters: Vec<&'a str>,
}

/// Unsubscribe acknowledgment
#[derive(Clone, Debug, PartialEq)]
pub struct UnsubscribeAck<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// UnsubscribeAck properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Disconnect notification
#[derive(Clone, Debug, PartialEq)]
pub struct Disconnect<'a> {
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// Disconnect properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Authentication exchange
#[derive(Clone, Debug, PartialEq)]
pub struct Auth<'a> {
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// Authentication properties
    pub properties: Option<Vec<Property<'a>>>,
}