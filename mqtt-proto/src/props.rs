use core::iter::{FromIterator, IntoIterator};
use core::time::Duration;
use std::vec::IntoIter;

use crate::mqtt::{Expiry, PayloadFormat, Property, QoS};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientProperties {
    pub will_delay_interval: Option<Duration>,
    pub payload_format: Option<PayloadFormat>,
    pub message_expiry_interval: Option<Duration>,
    pub content_type: Option<String>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<Vec<u8>>,
    pub user_properties: Vec<(String, String)>,
}

impl<'a> FromIterator<Property<'a>> for ClientProperties {
    fn from_iter<T: IntoIterator<Item = Property<'a>>>(props: T) -> Self {
        let mut client = Self::default();

        for prop in props {
            match prop {
                Property::WillDelayInterval(interval) => {
                    client.will_delay_interval = Some(interval)
                }
                Property::PayloadFormat(format) => client.payload_format = Some(format),
                Property::MessageExpiryInterval(interval) => {
                    client.message_expiry_interval = Some(interval)
                }
                Property::ContentType(content_type) => {
                    client.content_type = Some(content_type.to_owned())
                }
                Property::ResponseTopic(topic) => client.response_topic = Some(topic.to_owned()),
                Property::CorrelationData(data) => client.correlation_data = Some(data.to_owned()),
                Property::UserProperty(name, value) => {
                    client
                        .user_properties
                        .push((name.to_owned(), value.to_owned()));
                }
                _ => debug!("unexpected property: {:?}", prop),
            }
        }

        client
    }
}

impl<'a> IntoIterator for &'a ClientProperties {
    type Item = Property<'a>;
    type IntoIter = IntoIter<Property<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        let mut props = self
            .user_properties
            .iter()
            .map(|(name, value)| Property::UserProperty(name.as_str(), value.as_str()))
            .collect::<Vec<_>>();

        if let Some(interval) = self.will_delay_interval {
            props.push(Property::WillDelayInterval(interval));
        }
        if let Some(format) = self.payload_format {
            props.push(Property::PayloadFormat(format))
        }
        if let Some(interval) = self.message_expiry_interval {
            props.push(Property::MessageExpiryInterval(interval))
        }
        if let Some(ref content_type) = self.content_type {
            props.push(Property::ContentType(content_type.as_str()))
        }
        if let Some(ref topic) = self.response_topic {
            props.push(Property::ResponseTopic(topic.as_str()))
        }
        if let Some(ref data) = self.correlation_data {
            props.push(Property::CorrelationData(data.as_ref()))
        }

        props.into_iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerProperties {
    pub client_id: Option<String>,
    pub keep_alive: Option<Duration>,
    pub max_packet_size: Option<u32>,
    pub max_qos: Option<QoS>,
    pub max_receive_size: Option<u16>,
    pub max_topic_alias: Option<u16>,
    pub session_expired: Option<Expiry>,
    pub supports_retained_messages: bool,
    pub supports_shared_subscriptions: bool,
    pub supports_subscription_id: bool,
    pub supports_wildcard_subscriptions: bool,
    pub user_properties: Vec<(String, String)>,
}

impl ServerProperties {
    pub fn max_receive_size(&self) -> u16 {
        self.max_receive_size.unwrap_or(Self::MAX_RECEIVE_SIZE)
    }

    pub fn max_qos(&self) -> QoS {
        self.max_qos.unwrap_or(Self::MAX_QOS)
    }

    pub fn max_topic_alias(&self) -> u16 {
        self.max_topic_alias.unwrap_or(Self::MAX_TOPIC_ALIAS)
    }
}

impl ServerProperties {
    const MAX_RECEIVE_SIZE: u16 = 65535;
    const MAX_QOS: QoS = QoS::ExactlyOnce;
    const MAX_TOPIC_ALIAS: u16 = 0;
}

impl Default for ServerProperties {
    fn default() -> Self {
        ServerProperties {
            client_id: None,
            keep_alive: None,
            max_packet_size: None,
            max_qos: None,
            max_receive_size: None,
            max_topic_alias: None,
            session_expired: None,
            supports_retained_messages: true,
            supports_shared_subscriptions: true,
            supports_subscription_id: true,
            supports_wildcard_subscriptions: true,
            user_properties: vec![],
        }
    }
}

impl<'a> FromIterator<Property<'a>> for ServerProperties {
    fn from_iter<T: IntoIterator<Item = Property<'a>>>(props: T) -> Self {
        let mut server = Self::default();

        for prop in props {
            match prop {
                Property::AssignedClientId(id) => server.client_id = Some(id.to_owned()),
                Property::MaximumPacketSize(size) => server.max_packet_size = Some(size),
                Property::MaximumQoS(qos) => server.max_qos = Some(qos),
                Property::ReceiveMaximum(size) => server.max_receive_size = Some(size),
                Property::RetainAvailable(supports) => server.supports_retained_messages = supports,
                Property::ServerKeepAlive(keep_alive) => server.keep_alive = Some(keep_alive),
                Property::SessionExpiryInterval(expiry) => server.session_expired = Some(expiry),
                Property::SharedSubscriptionAvailable(supports) => {
                    server.supports_shared_subscriptions = supports
                }
                Property::SubscriptionIdAvailable(supports) => {
                    server.supports_subscription_id = supports
                }
                Property::TopicAliasMaximum(limits) => server.max_topic_alias = Some(limits),
                Property::WildcardSubscriptionAvailable(supports) => {
                    server.supports_wildcard_subscriptions = supports
                }
                Property::UserProperty(name, value) => {
                    server
                        .user_properties
                        .push((name.to_owned(), value.to_owned()));
                }
                _ => debug!("unexpected property: {:?}", prop),
            }
        }

        server
    }
}

impl<'a> IntoIterator for &'a ServerProperties {
    type Item = Property<'a>;
    type IntoIter = IntoIter<Property<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        let mut props = self
            .user_properties
            .iter()
            .map(|(name, value)| Property::UserProperty(name.as_str(), value.as_str()))
            .collect::<Vec<_>>();

        if let Some(expiry) = self.session_expired {
            props.push(Property::SessionExpiryInterval(expiry))
        }
        if let Some(size) = self.max_receive_size {
            props.push(Property::ReceiveMaximum(size))
        }
        if let Some(qos) = self.max_qos {
            props.push(Property::MaximumQoS(qos))
        }
        if !self.supports_retained_messages {
            props.push(Property::RetainAvailable(self.supports_retained_messages))
        }
        if let Some(size) = self.max_packet_size {
            props.push(Property::MaximumPacketSize(size))
        }
        if let Some(ref client_id) = self.client_id {
            props.push(Property::AssignedClientId(client_id.as_str()))
        }
        if let Some(limits) = self.max_topic_alias {
            props.push(Property::TopicAliasMaximum(limits))
        }
        if !self.supports_wildcard_subscriptions {
            props.push(Property::WildcardSubscriptionAvailable(
                self.supports_wildcard_subscriptions,
            ))
        }
        if !self.supports_subscription_id {
            props.push(Property::SubscriptionIdAvailable(
                self.supports_subscription_id,
            ))
        }
        if !self.supports_shared_subscriptions {
            props.push(Property::SharedSubscriptionAvailable(
                self.supports_shared_subscriptions,
            ))
        }
        if let Some(keep_alive) = self.keep_alive {
            props.push(Property::ServerKeepAlive(keep_alive))
        }

        props.into_iter()
    }
}
