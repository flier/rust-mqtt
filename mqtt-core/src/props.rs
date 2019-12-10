use core::time::Duration;

use num_enum::TryFromPrimitive;

use crate::QoS;

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
    RequestProblemInformation = 0x17,
    /// Will Delay Interval [Will Properties]
    WillDelayInterval = 0x18,
    /// Request Response Information [CONNECT]
    RequestResponseInformation = 0x19,
    /// Response Information [CONNACK]
    ResponseInformation = 0x1A,
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
    MessageExpiryInterval(Duration),
    /// Content Type [PUBLISH, Will Properties]
    ContentType(&'a str),
    /// Response Topic [PUBLISH, Will Properties]
    ResponseTopic(&'a str),
    /// Correlation Data [PUBLISH, Will Properties]
    CorrelationData(&'a [u8]),
    /// Subscription Identifier [PUBLISH, SUBSCRIBE]
    SubscriptionId(SubscriptionId),
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
    RequestProblemInformation(bool),
    /// Will Delay Interval [Will Properties]
    WillDelayInterval(Duration),
    /// Request Response Information [CONNECT]
    RequestResponseInformation(bool),
    /// Response Information [CONNACK]
    ResponseInformation(&'a str),
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
            Property::RequestProblemInformation(_) => PropertyId::RequestProblemInformation,
            Property::WillDelayInterval(_) => PropertyId::WillDelayInterval,
            Property::RequestResponseInformation(_) => PropertyId::RequestResponseInformation,
            Property::ResponseInformation(_) => PropertyId::ResponseInformation,
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

/// The Subscription Identifier is associated with any subscription created or modified as the result of this SUBSCRIBE packet.
pub type SubscriptionId = u32;

/// The largest value that can be represented by the Subscription Identifier.
pub const MIN_SUBSCRIPTION_ID: u32 = 1;
/// The smallest value that can be represented by the Subscription Identifier.
pub const MAX_SUBSCRIPTION_ID: u32 = 0x0FFF_FFFF;
