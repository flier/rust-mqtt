use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use crate::{
    mqtt::{PacketId, PayloadFormat, Property, QoS, ReasonCode, SubscriptionId, TopicAlias},
    Protocol, MQTT_V5,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Message<'a> {
    pub metadata: Metadata<'a>,
    pub topic_name: Cow<'a, str>,
    pub payload: Cow<'a, [u8]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Metadata<'a> {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub content_type: Option<Cow<'a, str>>,
    pub correlation_data: Option<Cow<'a, [u8]>>,
    pub message_expiry: Option<Duration>,
    pub payload_format: Option<PayloadFormat>,
    pub response_topic: Option<Cow<'a, str>>,
    pub subscription_id: Option<SubscriptionId>,
    pub topic_alias: Option<TopicAlias>,
    pub user_properties: Option<Vec<(Cow<'a, str>, Cow<'a, str>)>>,
}

impl<'a> Deref for Message<'a> {
    type Target = Metadata<'a>;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl<'a> DerefMut for Message<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

impl<'a> From<mqtt::Publish<'a>> for Message<'a> {
    fn from(publish: mqtt::Publish<'a>) -> Message<'a> {
        let mut metadata = Metadata {
            dup: publish.dup,
            qos: publish.qos,
            retain: publish.retain,
            ..Metadata::default()
        };

        if let Some(ref props) = publish.properties {
            use Property::*;

            for prop in props {
                match *prop {
                    PayloadFormat(format) => metadata.payload_format = Some(format),
                    MessageExpiryInterval(expire) => metadata.message_expiry = Some(expire),
                    TopicAlias(alias) => metadata.topic_alias = Some(alias),
                    ResponseTopic(topic_name) => metadata.response_topic = Some(topic_name.into()),
                    CorrelationData(data) => metadata.correlation_data = Some(data.into()),
                    SubscriptionId(id) => metadata.subscription_id = Some(id),
                    ContentType(content_type) => metadata.content_type = Some(content_type.into()),
                    UserProperty(name, value) => metadata
                        .user_properties
                        .get_or_insert_with(Vec::new)
                        .push((name.into(), value.into())),
                    _ => warn!("unexpected property: {:?}", prop),
                }
            }
        }

        Message {
            metadata,
            topic_name: publish.topic_name.into(),
            payload: publish.payload.into(),
        }
    }
}

impl<'a> Message<'a> {
    pub fn to_owned<'b>(&'a self) -> Message<'b> {
        Message {
            metadata: self.metadata.to_owned(),
            topic_name: self.topic_name.to_string().into(),
            payload: self.payload.to_vec().into(),
        }
    }
}

impl<'a> Metadata<'a> {
    pub fn to_owned<'b>(&'a self) -> Metadata<'b> {
        Metadata {
            dup: self.dup,
            qos: self.qos,
            retain: self.retain,
            content_type: self.content_type.as_ref().map(|s| s.to_string().into()),
            correlation_data: self.correlation_data.as_ref().map(|s| s.to_vec().into()),
            message_expiry: self.message_expiry,
            payload_format: self.payload_format,
            response_topic: self.response_topic.as_ref().map(|s| s.to_string().into()),
            subscription_id: self.subscription_id,
            topic_alias: self.topic_alias,
            user_properties: self.user_properties.as_ref().map(|props| {
                props
                    .iter()
                    .map(|(name, value)| (name.to_string().into(), value.to_string().into()))
                    .collect()
            }),
        }
    }
}

/// Publish message
#[derive(Clone, Debug, PartialEq)]
pub struct Publish<'a, P>(mqtt::Publish<'a>, PhantomData<P>);

impl<'a, P> Deref for Publish<'a, P> {
    type Target = mqtt::Publish<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for Publish<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, P> Into<mqtt::Publish<'a>> for Publish<'a, P> {
    fn into(self) -> mqtt::Publish<'a> {
        self.0
    }
}

impl<'a, P> Publish<'a, P> {
    pub fn new(message: &'a Message) -> Publish<'a, P>
    where
        P: Protocol,
    {
        let mut properties = P::default_properties();

        if let Some(props) = properties.as_mut() {
            if let Some(ref content_type) = message.content_type {
                props.push(Property::ContentType(content_type));
            }
            if let Some(ref data) = message.correlation_data {
                props.push(Property::CorrelationData(data));
            }
            if let Some(expire) = message.message_expiry {
                props.push(Property::MessageExpiryInterval(expire));
            }
            if let Some(format) = message.payload_format {
                props.push(Property::PayloadFormat(format));
            }
            if let Some(ref topic) = message.response_topic {
                props.push(Property::ResponseTopic(topic));
            }
            if let Some(id) = message.subscription_id {
                props.push(Property::SubscriptionId(id));
            }
            if let Some(alias) = message.topic_alias {
                props.push(Property::TopicAlias(alias));
            }
            if let Some(ref user_properties) = message.user_properties {
                for (name, value) in user_properties {
                    props.push(Property::UserProperty(&name, &value));
                }
            }
        }

        Publish(
            mqtt::Publish {
                dup: message.dup,
                qos: message.qos,
                retain: message.retain,
                topic_name: &message.topic_name,
                packet_id: None,
                properties,
                payload: &message.payload,
            },
            PhantomData,
        )
    }

    pub fn with_dup(&mut self, dup: bool) -> &mut Self {
        self.dup = dup;
        self
    }

    pub fn with_qos(&mut self, qos: QoS) -> &mut Self {
        self.qos = qos;
        self
    }

    pub fn with_retain(&mut self, retain: bool) -> &mut Self {
        self.retain = retain;
        self
    }

    pub fn with_packet_id(&mut self, packet_id: PacketId) -> &mut Self {
        self.packet_id = Some(packet_id);
        self
    }
}

impl<'a> Publish<'a, MQTT_V5> {
    /// Configure property.
    pub fn with_property(&mut self, property: Property<'a>) -> &mut Self {
        self.properties.get_or_insert_with(Vec::new).push(property);
        self
    }

    /// User Property.
    pub fn with_user_property(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.with_property(Property::UserProperty(name, value))
    }

    /// Payload Format Indicator.
    pub fn with_payload_format(&mut self, payload_format: PayloadFormat) -> &mut Self {
        self.with_property(Property::PayloadFormat(payload_format))
    }

    /// Message Expiry Interval is the lifetime of the Application Message.
    pub fn with_message_expiry_interval(&mut self, expire: Duration) -> &mut Self {
        self.with_property(Property::MessageExpiryInterval(expire))
    }

    /// Topic Alias used to identify the Topic instead of using the Topic Name.
    pub fn with_topic_alias(&mut self, topic_alias: TopicAlias) -> &mut Self {
        self.with_property(Property::TopicAlias(topic_alias))
    }

    /// Response Topic used as the Topic Name for a response message.
    pub fn with_response_topic(&mut self, response_topic: &'a str) -> &mut Self {
        self.with_property(Property::ResponseTopic(response_topic))
    }

    /// The Correlation Data is used by the sender of the Request Message to identify which request the Response Message is for when it is received.
    pub fn with_correlation_data(&mut self, correlation_data: &'a [u8]) -> &mut Self {
        self.with_property(Property::CorrelationData(correlation_data))
    }

    /// Subscription Identifier representing the identifier of the subscription.
    pub fn with_subscription_id(&mut self, subscription_id: SubscriptionId) -> &mut Self {
        self.with_property(Property::SubscriptionId(subscription_id))
    }

    /// Content Type describing the content of the Application Message.
    pub fn with_content_type(&mut self, content_type: &'a str) -> &mut Self {
        self.with_property(Property::ContentType(content_type))
    }
}

#[derive(Debug, Default)]
pub struct Published {
    pub reason_code: ReasonCode,
    pub reason: Option<String>,
    pub user_properties: Vec<(String, String)>,
}

impl Published {
    pub fn new<'a, I>(reason_code: ReasonCode, properties: I) -> Published
    where
        I: IntoIterator<Item = Property<'a>>,
    {
        properties.into_iter().fold(
            Published {
                reason_code,
                ..Default::default()
            },
            |mut published, prop| {
                match prop {
                    Property::Reason(reason) => published.reason = Some(reason.to_string()),
                    Property::UserProperty(name, value) => published
                        .user_properties
                        .push((name.to_string(), value.to_string())),
                    _ => {}
                }

                published
            },
        )
    }
}

impl<'a> From<mqtt::PublishAck<'a>> for Published {
    fn from(publish_ack: mqtt::PublishAck<'a>) -> Published {
        Published::new(
            publish_ack.reason_code.unwrap_or_default(),
            publish_ack.properties.into_iter().flatten(),
        )
    }
}

impl<'a> From<mqtt::PublishReceived<'a>> for Published {
    fn from(publish_received: mqtt::PublishReceived<'a>) -> Published {
        Published::new(
            publish_received.reason_code.unwrap_or_default(),
            publish_received.properties.into_iter().flatten(),
        )
    }
}
