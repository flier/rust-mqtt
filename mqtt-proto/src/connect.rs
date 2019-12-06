use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::time::Duration;

use crate::{
    packet::{Expiry, Packet, PayloadFormat, Property, ProtocolVersion, QoS},
    Protocol, MQTT_V5,
};

/// Connect to an MQTT broker.
pub fn connect<P>(keep_alive: Duration, client_id: &str) -> Connect<P>
where
    P: crate::Protocol,
{
    Connect(
        packet::Connect {
            protocol_version: P::VERSION,
            clean_session: true,
            keep_alive: keep_alive.as_secs() as u16,
            properties: if P::VERSION >= ProtocolVersion::V5 {
                Some(Vec::new())
            } else {
                None
            },
            client_id,
            last_will: None,
            username: None,
            password: None,
        },
        PhantomData,
    )
}

/// Client request to connect to Server
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct Connect<'a, P>(packet::Connect<'a>, PhantomData<P>);

impl<'a, P> Deref for Connect<'a, P> {
    type Target = packet::Connect<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for Connect<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, P> From<Connect<'a, P>> for Packet<'a> {
    fn from(connect: Connect<'a, P>) -> Packet<'a> {
        Packet::Connect(connect.0)
    }
}

impl<'a, P: Protocol> Connect<'a, P> {
    pub fn last_will(&'a mut self) -> Option<LastWill<'a, P>> {
        self.0
            .last_will
            .as_mut()
            .map(|last_will| LastWill(last_will, PhantomData))
    }

    /// whether the Connection starts a new Session or is a continuation of an existing Session.
    pub fn without_clean_session(&mut self) -> &mut Self {
        self.clean_session = false;
        self
    }

    /// Configure will information.
    pub fn with_last_will(
        &mut self,
        topic_name: &'a str,
        message: &'a [u8],
        qos: QoS,
        retain: bool,
    ) -> &mut Self {
        self.last_will = Some(packet::LastWill {
            qos,
            retain,
            topic_name,
            message,
            properties: if P::VERSION >= ProtocolVersion::V5 {
                Some(Vec::new())
            } else {
                None
            },
        });
        self
    }

    /// Configure username.
    pub fn with_username(&mut self, username: &'a str) -> &mut Self {
        self.username = Some(username);
        self
    }

    /// Configure password.
    pub fn with_password(&mut self, password: &'a str) -> &mut Self {
        self.password = Some(password.as_bytes());
        self
    }
}

impl<'a> Connect<'a, MQTT_V5> {
    /// Configure property.
    pub fn with_property(&mut self, property: Property<'a>) -> &mut Self {
        self.properties.get_or_insert_with(Vec::new).push(property);
        self
    }

    /// Session Expiry Interval
    pub fn with_session_expiry(&mut self, expiry: Expiry) -> &mut Self {
        self.with_property(Property::SessionExpiryInterval(expiry))
    }

    /// Receive Maximum
    ///
    /// The Client uses this value to limit the number of QoS 1 and QoS 2 publications that it is willing to process concurrently.
    /// There is no mechanism to limit the QoS 0 publications that the Server might try to send.
    ///
    /// The value of Receive Maximum applies only to the current Network Connection.
    /// If the Receive Maximum value is absent then its value defaults to 65,535.
    pub fn with_receive_maximum(&mut self, value: u16) -> &mut Self {
        self.with_property(Property::ReceiveMaximum(value))
    }

    /// Maximum Packet Size
    ///
    /// The packet size is the total number of bytes in an MQTT Control Packet, as defined in section 2.1.4.
    /// The Client uses the Maximum Packet Size to inform the Server that it will not process packets exceeding this limit.
    pub fn with_maximum_packet_size(&mut self, size: u32) -> &mut Self {
        self.with_property(Property::MaximumPacketSize(size))
    }

    /// Topic Alias Maximum
    ///
    /// This value indicates the highest value that the Client will accept as a Topic Alias sent by the Server.
    /// The Client uses this value to limit the number of Topic Aliases that it is willing to hold on this Connection.
    pub fn with_topic_alias_maximum(&mut self, value: u16) -> &mut Self {
        self.with_property(Property::TopicAliasMaximum(value))
    }

    /// Request Response Information
    ///
    /// The Client uses this value to request the Server to return Response Information in the CONNACK.
    pub fn request_response_information(&mut self, value: bool) -> &mut Self {
        self.with_property(Property::RequestResponseInformation(value))
    }

    /// Request Problem Information
    ///
    /// The Client uses this value to indicate whether the Reason String or User Properties are sent in the case of failures.
    pub fn request_problem_information(&mut self, value: bool) -> &mut Self {
        self.with_property(Property::RequestProblemInformation(value))
    }

    /// User Property
    pub fn with_user_property(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.with_property(Property::UserProperty(name, value))
    }

    /// Authentication Method
    pub fn with_authentication(&mut self, method: &'a str, data: &'a [u8]) -> &mut Self {
        self.with_property(Property::AuthMethod(method))
            .with_property(Property::AuthData(data))
    }
}

/// Connection Will
#[repr(transparent)]
#[derive(Debug)]
pub struct LastWill<'a, P>(&'a mut packet::LastWill<'a>, PhantomData<P>);

impl<'a, P> Deref for LastWill<'a, P> {
    type Target = packet::LastWill<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for LastWill<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a> LastWill<'a, MQTT_V5> {
    /// Configure property.
    pub fn with_property(&mut self, property: Property<'a>) -> &mut Self {
        self.0
            .properties
            .get_or_insert_with(Vec::new)
            .push(property);
        self
    }

    /// Will Delay Interval
    pub fn with_delay_interval(&mut self, interval: Duration) -> &mut Self {
        self.with_property(Property::WillDelayInterval(interval))
    }

    /// Payload Format Indicator
    pub fn with_payload_format(&mut self, format: PayloadFormat) -> &mut Self {
        self.with_property(Property::PayloadFormat(format))
    }

    /// Message Expiry Interval
    pub fn with_message_expiry_interval(&mut self, interval: Duration) -> &mut Self {
        self.with_property(Property::MessageExpiryInterval(interval))
    }

    /// Content Type
    pub fn with_content_type(&mut self, content_type: &'a str) -> &mut Self {
        self.with_property(Property::ContentType(content_type))
    }

    /// Response Topic
    pub fn with_response_topic(&mut self, topic_name: &'a str) -> &mut Self {
        self.with_property(Property::ResponseTopic(topic_name))
    }

    /// Correlation Data
    pub fn with_correlation_data(&mut self, correlation_data: &'a [u8]) -> &mut Self {
        self.with_property(Property::CorrelationData(correlation_data))
    }

    /// User Property
    pub fn with_user_property(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.with_property(Property::UserProperty(name, value))
    }
}
