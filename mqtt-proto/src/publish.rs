use std::ops::{Deref, DerefMut};
use std::time::Instant;

use bytes::Bytes;

use crate::mqtt::{PayloadFormat, Property, Publish, QoS, SubscriptionId};

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub topic_name: String,
    pub metadata: Metadata,
    pub payload: Bytes,
}

impl Deref for Message {
    type Target = Metadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl DerefMut for Message {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Metadata {
    pub qos: QoS,
    pub retain: bool,
    pub content_type: Option<String>,
    pub correlation_data: Option<Bytes>,
    pub message_expiry: Option<Instant>,
    pub payload_format: Option<PayloadFormat>,
    pub response_topic: Option<String>,
    pub subscription_id: Option<SubscriptionId>,
    pub topic_alias: Option<u16>,
    pub user_properties: Option<Vec<(String, String)>>,
}

impl<'a> From<Publish<'a>> for Message {
    fn from(publish: Publish<'a>) -> Self {
        let mut metadata = Metadata {
            qos: publish.qos,
            retain: publish.retain,
            ..Metadata::default()
        };

        if let Some(ref props) = publish.properties {
            use Property::*;

            for prop in props {
                match *prop {
                    PayloadFormat(format) => metadata.payload_format = Some(format),
                    MessageExpiryInterval(expire) => {
                        metadata.message_expiry = Some(Instant::now() + expire)
                    }
                    TopicAlias(alias) => metadata.topic_alias = Some(alias),
                    ResponseTopic(topic_name) => {
                        metadata.response_topic = Some(topic_name.to_owned())
                    }
                    CorrelationData(data) => {
                        metadata.correlation_data = Some(Bytes::copy_from_slice(data))
                    }
                    SubscriptionId(id) => metadata.subscription_id = Some(id),
                    ContentType(content_type) => {
                        metadata.content_type = Some(content_type.to_owned())
                    }
                    UserProperty(name, value) => metadata
                        .user_properties
                        .get_or_insert_with(Vec::new)
                        .push((name.to_owned(), value.to_owned())),
                    _ => warn!("unexpected property: {:?}", prop),
                }
            }
        }

        Message {
            topic_name: publish.topic_name.to_owned(),
            metadata,
            payload: Bytes::copy_from_slice(publish.payload),
        }
    }
}
