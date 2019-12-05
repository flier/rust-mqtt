use core::time::Duration;

use derive_more::Display;
use num_enum::{TryFromPrimitive, UnsafeFromPrimitive};

/// The Protocol Name of the protocol.
pub const PROTOCOL_NAME: &[u8] = b"\x00\x04MQTT";

/// The revision level of the protocol used by the Client.
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, PartialOrd, Copy, Clone, TryFromPrimitive)]
pub enum ProtocolVersion {
    /// The value of the Protocol Level field for the version 3.1.1 of the protocol is 4 (0x04).
    V311 = 4,
    /// The value of the Protocol Version field for version 5.0 of the protocol is 5 (0x05).
    V5 = 5,
}

/// MQTT Control Packets
#[derive(Debug, PartialEq, Clone)]
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
    /// Disconnect notification
    DISCONNECT = 14,
    /// Authentication exchange
    AUTH = 15,
}

/// Property identifier which defines its usage and data type
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, TryFromPrimitive)]
pub enum PropertyId {
    /// Payload Format Indicator [PUBLISH, Will Properties]
    PayloadFormat = 0x01,
    /// Message Expiry Interval [PUBLISH, Will Properties]
    MessageExpiryInterval = 0x02,
    /// Content Type [PUBLISH, Will Properties]
    ContentType = 0x03,
    /// Response Topic [PUBLISH, Will Properties]
    ResponseTopic = 0x08,
    /// Correlation Data [PUBLISH, Will Properties]
    CorrelationData = 0x09,
    /// Subscription Identifier [PUBLISH, SUBSCRIBE]
    SubscriptionId = 0x0B,
    /// Session Expiry Interval [CONNECT, CONNACK, DISCONNECT]
    SessionExpiryInterval = 0x11,
    /// Assigned Client Identifier [CONNACK]
    AssignedClientId = 0x12,
    /// Server Keep Alive [CONNACK]
    ServerKeepAlive = 0x13,
    /// Authentication Method [CONNECT, CONNACK, AUTH]
    AuthMethod = 0x15,
    /// Authentication Data [CONNECT, CONNACK, AUTH]
    AuthData = 0x16,
    /// Request Problem Information [CONNECT]
    RequestProblem = 0x17,
    /// Will Delay Interval [Will Properties]
    WillDelayInterval = 0x18,
    /// Request Response Information [CONNECT]
    RequestResponse = 0x19,
    /// Response Information [CONNACK]
    Response = 0x1A,
    /// Server Reference [CONNACK, DISCONNECT]
    ServerReference = 0x1C,
    /// Reason String [CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBACK, UNSUBACK, DISCONNECT, AUTH]
    Reason = 0x1F,
    /// Receive Maximum [CONNECT, CONNACK]
    ReceiveMaximum = 0x21,
    /// Topic Alias Maximum [CONNECT, CONNACK]
    TopicAliasMaximum = 0x22,
    /// Topic Alias [PUBLISH]
    TopicAlias = 0x23,
    /// MaximumQoS [CONNACK]
    MaximumQoS = 0x24,
    /// Retain Available [CONNACK]
    RetainAvailable = 0x25,
    /// User Property [CONNECT, CONNACK, PUBLISH, Will Properties, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBSCRIBE, SUBACK, UNSUBSCRIBE, UNSUBACK, DISCONNECT, AUTH]
    UserProperty = 0x26,
    /// Maximum Packet Size [CONNECT, CONNACK]
    MaximumPacketSize = 0x27,
    /// Wildcard Subscription Available [CONNACK]
    WildcardSubscriptionAvailable = 0x28,
    /// Subscription Identifier Available [CONNACK]
    SubscriptionIdAvailable = 0x29,
    /// Shared Subscription Available [CONNACK]
    SharedSubscriptionAvailable = 0x2A,
}

/// Property with data
#[derive(Debug, Clone, PartialEq)]
pub enum Property<'a> {
    /// Payload Format Indicator [PUBLISH, Will Properties]
    PayloadFormat(PayloadFormat),
    /// Message Expiry Interval [PUBLISH, Will Properties]
    MessageExpiryInterval(Expiry),
    /// Content Type [PUBLISH, Will Properties]
    ContentType(&'a str),
    /// Response Topic [PUBLISH, Will Properties]
    ResponseTopic(&'a str),
    /// Correlation Data [PUBLISH, Will Properties]
    CorrelationData(&'a [u8]),
    /// Subscription Identifier [PUBLISH, SUBSCRIBE]
    SubscriptionId(usize),
    /// Session Expiry Interval [CONNECT, CONNACK, DISCONNECT]
    SessionExpiryInterval(Expiry),
    /// Assigned Client Identifier [CONNACK]
    AssignedClientId(&'a str),
    /// Server Keep Alive [CONNACK]
    ServerKeepAlive(u16),
    /// Authentication Method [CONNECT, CONNACK, AUTH]
    AuthMethod(&'a str),
    /// Authentication Data [CONNECT, CONNACK, AUTH]
    AuthData(&'a [u8]),
    /// Request Problem Information [CONNECT]
    RequestProblem(u8),
    /// Will Delay Interval [Will Properties]
    WillDelayInterval(Expiry),
    /// Request Response Information [CONNECT]
    RequestResponse(u8),
    /// Response Information [CONNACK]
    Response(&'a str),
    /// Server Reference [CONNACK, DISCONNECT]
    ServerReference(&'a str),
    /// Reason String [CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBACK, UNSUBACK, DISCONNECT, AUTH]
    Reason(&'a str),
    /// Receive Maximum [CONNECT, CONNACK]
    ReceiveMaximum(u16),
    /// Topic Alias Maximum [CONNECT, CONNACK]
    TopicAliasMaximum(u16),
    /// Topic Alias [PUBLISH]
    TopicAlias(u16),
    /// MaximumQoS [CONNACK]
    MaximumQoS(QoS),
    /// Retain Available [CONNACK]
    RetainAvailable(bool),
    /// User Property [CONNECT, CONNACK, PUBLISH, Will Properties, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBSCRIBE, SUBACK, UNSUBSCRIBE, UNSUBACK, DISCONNECT, AUTH]
    UserProperty(&'a str, &'a str),
    /// Maximum Packet Size [CONNECT, CONNACK]
    MaximumPacketSize(u32),
    /// Wildcard Subscription Available [CONNACK]
    WildcardSubscriptionAvailable(bool),
    /// Subscription Identifier Available [CONNACK]
    SubscriptionIdAvailable(bool),
    /// Shared Subscription Available [CONNACK]
    SharedSubscriptionAvailable(bool),
}

impl Property<'_> {
    /// Returns property identifier which defines its usage and data type
    pub fn id(&self) -> PropertyId {
        match self {
            Property::PayloadFormat(_) => PropertyId::PayloadFormat,
            Property::MessageExpiryInterval(_) => PropertyId::MessageExpiryInterval,
            Property::ContentType(_) => PropertyId::ContentType,
            Property::ResponseTopic(_) => PropertyId::ResponseTopic,
            Property::CorrelationData(_) => PropertyId::CorrelationData,
            Property::SubscriptionId(_) => PropertyId::SubscriptionId,
            Property::SessionExpiryInterval(_) => PropertyId::SessionExpiryInterval,
            Property::AssignedClientId(_) => PropertyId::AssignedClientId,
            Property::ServerKeepAlive(_) => PropertyId::ServerKeepAlive,
            Property::AuthMethod(_) => PropertyId::AuthMethod,
            Property::AuthData(_) => PropertyId::AuthData,
            Property::RequestProblem(_) => PropertyId::RequestProblem,
            Property::WillDelayInterval(_) => PropertyId::WillDelayInterval,
            Property::RequestResponse(_) => PropertyId::RequestResponse,
            Property::Response(_) => PropertyId::Response,
            Property::ServerReference(_) => PropertyId::ServerReference,
            Property::Reason(_) => PropertyId::Reason,
            Property::ReceiveMaximum(_) => PropertyId::ReceiveMaximum,
            Property::TopicAliasMaximum(_) => PropertyId::TopicAliasMaximum,
            Property::TopicAlias(_) => PropertyId::TopicAlias,
            Property::MaximumQoS(_) => PropertyId::MaximumQoS,
            Property::RetainAvailable(_) => PropertyId::RetainAvailable,
            Property::UserProperty(_, _) => PropertyId::UserProperty,
            Property::MaximumPacketSize(_) => PropertyId::MaximumPacketSize,
            Property::WildcardSubscriptionAvailable(_) => PropertyId::WildcardSubscriptionAvailable,
            Property::SubscriptionIdAvailable(_) => PropertyId::SubscriptionIdAvailable,
            Property::SharedSubscriptionAvailable(_) => PropertyId::SharedSubscriptionAvailable,
        }
    }
}

/// Identifier of the Payload Format Indicator.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, TryFromPrimitive)]
pub enum PayloadFormat {
    /// the Will Message is unspecified bytes,
    Binary = 0,
    /// UTF-8 Encoded Character Data.
    Utf8 = 1,
}

/// Identifier of the Session Expiry Interval.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Expiry {
    /// Interval in seconds
    Interval(Duration),
    /// the Session does not expire.
    Never,
}

impl Expiry {
    /// Returns the number of whole seconds contained by this Expiry.
    pub fn as_secs(&self) -> u32 {
        match self {
            Expiry::Interval(ref d) => d.as_secs() as u32,
            Expiry::Never => u32::max_value(),
        }
    }
}

/// The result of an operation
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, TryFromPrimitive)]
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

/// Connection Will
#[derive(Debug, PartialEq, Hash, Clone)]
pub struct LastWill<'a> {
    /// the QoS level to be used when publishing the Will Message.
    pub qos: QoS,
    /// the Will Message is to be Retained when it is published.
    pub retain: bool,
    /// the Will Topic
    pub topic_name: &'a str,
    /// defines the Application Message that is to be published to the Will Topic
    pub message: &'a [u8],
}

/// Connect acknowledgment
#[derive(Debug, PartialEq, Clone)]
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

bitflags! {
    /// The Connect Acknowledge Flags.
    #[derive(Default)]
    pub struct ConnectAckFlags: u8 {
        /// The Session Present flag enables a Client to establish
        /// whether the Client and Server have a consistent view about whether there is already stored Session state.
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
    pub topic_name: &'a str,
    /// only present in PUBLISH Packets where the QoS level is 1 or 2.
    pub packet_id: Option<PacketId>,
    /// Publish properties
    pub properties: Option<Vec<Property<'a>>>,
    /// the Application Message that is being published.
    pub payload: &'a [u8],
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

/// Publish acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct PublishAck<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishAck properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Publish received (assured delivery part 1)
#[derive(Debug, PartialEq, Clone)]
pub struct PublishReceived<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishReceived properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Publish release (assured delivery part 2)
#[derive(Debug, PartialEq, Clone)]
pub struct PublishRelease<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishRelease properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Publish complete (assured delivery part 3)
#[derive(Debug, PartialEq, Clone)]
pub struct PublishComplete<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// PublishComplete properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Client subscribe request
#[derive(Debug, PartialEq, Clone)]
pub struct Subscribe<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Subscribe properties
    pub properties: Option<Vec<Property<'a>>>,
    /// the list of Topic Filters and QoS to which the Client wants to subscribe.
    pub subscriptions: Vec<(&'a str, QoS)>,
}

/// Subscribe acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct SubscribeAck<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// SubscribeAck properties
    pub properties: Option<Vec<Property<'a>>>,
    /// corresponds to a Topic Filter in the SUBSCRIBE Packet being acknowledged.
    pub status: Vec<SubscribeReturnCode>,
}

/// Subscribe Return Code
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SubscribeReturnCode {
    /// Success
    Success(QoS),
    /// Failure
    Failure,
}

impl SubscribeReturnCode {
    pub(crate) const FAILURE: u8 = 0x80;
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
#[derive(Debug, PartialEq, Clone)]
pub struct Unsubscribe<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// Unsubscribe properties
    pub properties: Option<Vec<Property<'a>>>,
    /// the list of Topic Filters that the Client wishes to unsubscribe from.
    pub topic_filters: Vec<&'a str>,
}

/// Unsubscribe acknowledgment
#[derive(Debug, PartialEq, Clone)]
pub struct UnsubscribeAck<'a> {
    /// Packet Identifier
    pub packet_id: PacketId,
    /// UnsubscribeAck properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Disconnect notification
#[derive(Debug, PartialEq, Clone)]
pub struct Disconnect<'a> {
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// Disconnect properties
    pub properties: Option<Vec<Property<'a>>>,
}

/// Authentication exchange
#[derive(Debug, PartialEq, Clone)]
pub struct Auth<'a> {
    /// Reason Code
    pub reason_code: Option<ReasonCode>,
    /// Authentication properties
    pub properties: Option<Vec<Property<'a>>>,
}
