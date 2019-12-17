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
#[derive(Debug, Eq, PartialEq, Clone, Copy, TryFromPrimitive, UnsafeFromPrimitive, Display)]
pub enum ReasonCode {
    /// Granted QoS 0 [SUBACK]
    #[display(fmt = "Granted QoS 0")]
    GrantedQoS0 = 0,
    /// Granted QoS 1 [SUBACK]
    #[display(fmt = "Granted QoS 1")]
    GrantedQoS1 = 1,
    /// Granted QoS 2 [SUBACK]
    #[display(fmt = "Granted QoS 2")]
    GrantedQoS2 = 2,
    /// Disconnect with Will Message [DISCONNECT]
    #[display(fmt = "Disconnect with Will Message")]
    DisconnectWithWill = 0x04,
    /// No matching subscribers [PUBACK, PUBREC]
    #[display(fmt = "No matching subscribers")]
    NoMatchingSubscribers = 0x10,
    /// No subscription existed [UNSUBACK]
    #[display(fmt = "No subscription existed")]
    NoSubscriptionExisted = 0x11,
    /// Continue authentication [AUTH]
    #[display(fmt = "Continue authentication")]
    ContinueAuthentication = 0x18,
    /// Re-authenticate [AUTH]
    #[display(fmt = "Re-authenticate")]
    Reauthenticate = 0x19,
    /// Unspecified error [CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT]
    #[display(fmt = "Unspecified error")]
    UnspecifiedError = 0x80,
    /// Malformed Packet [CONNACK, DISCONNECT]
    #[display(fmt = "Malformed Packet")]
    MalformedPacket = 0x81,
    /// Protocol Error [CONNACK, DISCONNECT]
    #[display(fmt = "Protocol Error")]
    ProtocolError = 0x82,
    /// Implementation specific error [CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT]
    #[display(fmt = "Implementation specific error")]
    ImplementationSpecificError = 0x83,
    /// Unsupported Protocol Version [CONNACK]
    #[display(fmt = "Unsupported Protocol Version")]
    UnsupportedProtocolVersion = 0x84,
    /// Client Identifier not valid [CONNACK]
    #[display(fmt = "Client Identifier not valid")]
    ClientIdNotValid = 0x85,
    /// Bad User Name or Password [CONNACK]
    #[display(fmt = "Bad User Name or Password")]
    BadUserNameOrPassword = 0x86,
    /// Not authorized [CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT]
    #[display(fmt = "Not authorized")]
    NotAuthorized = 0x87,
    /// Server unavailable [CONNACK]
    #[display(fmt = "Server unavailable")]
    ServerUnavailable = 0x88,
    /// Server busy [CONNACK, DISCONNECT]
    #[display(fmt = "Server busy")]
    ServerBusy = 0x89,
    /// Banned [CONNACK]
    #[display(fmt = "Banned")]
    Banned = 0x8A,
    /// Server shutting down [DISCONNECT]
    #[display(fmt = "Server shutting down")]
    ServerShuttingDown = 0x8B,
    /// Bad authentication method [CONNACK, DISCONNECT]
    #[display(fmt = "Bad authentication method")]
    BadAuthenticationMethod = 0x8C,
    /// Keep Alive timeout [DISCONNECT]
    #[display(fmt = "Keep Alive timeout")]
    KeepAliveTimeout = 0x8D,
    /// Session taken over [DISCONNECT]
    #[display(fmt = "Session taken over")]
    SessionTakenOver = 0x8E,
    /// Topic Filter invalid [SUBACK, UNSUBACK, DISCONNECT]
    #[display(fmt = "Topic Filter invalid")]
    InvalidTopicFilter = 0x8F,
    /// Topic Name invalid [CONNACK, PUBACK, PUBREC, DISCONNECT]
    #[display(fmt = "Topic Name invalid")]
    InvalidTopicName = 0x90,
    /// Packet Identifier in use [PUBACK, PUBREC, SUBACK, UNSUBACK]
    #[display(fmt = "Packet Identifier in use")]
    PacketIdInUse = 0x91,
    /// Packet Identifier not found [PUBREL, PUBCOMP]
    #[display(fmt = "Packet Identifier not found")]
    PacketIdNotFound = 0x92,
    /// Receive Maximum exceeded [DISCONNECT]
    #[display(fmt = "Receive Maximum exceeded")]
    ReceiveMaximumExceeded = 0x93,
    /// Topic Alias invalid [DISCONNECT]
    #[display(fmt = "Topic Alias invalid")]
    InvalidTopicAlias = 0x94,
    /// Packet too large [CONNACK, DISCONNECT]
    #[display(fmt = "Packet too large")]
    PacketTooLarge = 0x95,
    /// Message rate too high [DISCONNECT]
    #[display(fmt = "Message rate too high")]
    MessageRateTooHigh = 0x96,
    /// Quota exceeded [CONNACK, PUBACK, PUBREC, SUBACK, DISCONNECT]
    #[display(fmt = "Quota exceeded")]
    QuotaExceeded = 0x97,
    /// Administrative action [DISCONNECT]
    #[display(fmt = "Administrative action")]
    AdministrativeAction = 0x98,
    /// Payload format invalid [CONNACK, PUBACK, PUBREC, DISCONNECT]
    #[display(fmt = "Payload format invalid")]
    InvalidPayloadFormat = 0x99,
    /// Retain not supported [CONNACK, DISCONNECT]
    #[display(fmt = "Retain not supported")]
    RetainNotSupported = 0x9A,
    /// QoS not supported [CONNACK, DISCONNECT]
    #[display(fmt = "QoS not supported")]
    QoSNotSupported = 0x9B,
    /// Use another server [CONNACK, DISCONNECT]
    #[display(fmt = "Use another server")]
    UseAnotherServer = 0x9C,
    /// Server moved [CONNACK, DISCONNECT]
    #[display(fmt = "Server moved")]
    ServerMoved = 0x9D,
    /// Shared Subscriptions not supported [SUBACK, DISCONNECT]
    #[display(fmt = "Shared Subscriptions not supported")]
    SharedSubscriptionsNotSupported = 0x9E,
    /// Connection rate exceeded [CONNACK, DISCONNECT]
    #[display(fmt = "Connection rate exceeded")]
    ConnectionRateExceeded = 0x9F,
    /// Maximum connect time [DISCONNECT]
    #[display(fmt = "Maximum connect time")]
    MaximumConnectTime = 0xA0,
    /// Subscription Identifiers not supported [SUBACK, DISCONNECT]
    #[display(fmt = "Subscription Identifiers not supported")]
    SubscriptionIdNotSupported = 0xA1,
    /// Wildcard Subscriptions not supported [SUBACK, DISCONNECT]
    #[display(fmt = "Wildcard Subscriptions not supported")]
    WildcardSubscriptionsNotSupported = 0xA2,
}

impl Default for ReasonCode {
    fn default() -> Self {
        ReasonCode::Success
    }
}

impl StdError for ReasonCode {}

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

impl Default for ConnectReturnCode {
    fn default() -> Self {
        Self::ConnectionAccepted
    }
}

impl StdError for ConnectReturnCode {}

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
    pub status: Vec<Result<QoS, ReasonCode>>,
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
    /// The list of Reason Codes.
    pub status: Option<Vec<Result<(), ReasonCode>>>,
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
