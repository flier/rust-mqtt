use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::{
    mqtt::{Property, ReasonCode},
    Protocol, MQTT_V5,
};

/// An UNSUBSCRIBE packet is sent by the Client to the Server, to unsubscribe from topics.
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct Unsubscribe<'a, P>(mqtt::Unsubscribe<'a>, PhantomData<P>);

impl<'a, P> Deref for Unsubscribe<'a, P> {
    type Target = mqtt::Unsubscribe<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for Unsubscribe<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, P> Into<mqtt::Unsubscribe<'a>> for Unsubscribe<'a, P> {
    fn into(self) -> mqtt::Unsubscribe<'a> {
        self.0
    }
}

impl<'a, P> Unsubscribe<'a, P> {
    /// An UNSUBSCRIBE packet is sent by the Client to the Server, to unsubscribe from topics.
    pub fn new<I>(packet_id: u16, topic_filters: I) -> Unsubscribe<'a, P>
    where
        P: Protocol,
        I: IntoIterator<Item = &'a str>,
    {
        Unsubscribe(
            mqtt::Unsubscribe {
                packet_id,
                properties: P::default_properties(),
                topic_filters: topic_filters.into_iter().collect(),
            },
            PhantomData,
        )
    }

    pub fn with_topic_filter(&mut self, topic_filter: &'a str) -> &mut Self {
        self.topic_filters.push(topic_filter);
        self
    }
}

impl<'a> Unsubscribe<'a, MQTT_V5> {
    /// Configure property.
    pub fn with_property(&mut self, property: Property<'a>) -> &mut Self {
        self.properties.get_or_insert_with(Vec::new).push(property);
        self
    }

    /// User Property
    pub fn with_user_property(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.with_property(Property::UserProperty(name, value))
    }
}

pub struct Unsubscribed {
    pub status: Vec<Result<(), ReasonCode>>,
    pub reason: Option<String>,
    pub user_properties: Option<Vec<(String, String)>>,
}

impl<'a> From<mqtt::UnsubscribeAck<'a>> for Unsubscribed {
    fn from(unsubscribe_ack: mqtt::UnsubscribeAck) -> Self {
        Unsubscribed {
            status: unsubscribe_ack.status.unwrap_or_default(),
            reason: unsubscribe_ack.properties.as_ref().and_then(|props| {
                props.iter().find_map(|prop| {
                    if let Property::Reason(reason) = prop {
                        Some(reason.to_string())
                    } else {
                        None
                    }
                })
            }),
            user_properties: unsubscribe_ack.properties.as_ref().and_then(|props| {
                let user_props = props
                    .iter()
                    .flat_map(|prop| {
                        if let Property::UserProperty(name, value) = prop {
                            Some((name.to_string(), value.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                if user_props.is_empty() {
                    None
                } else {
                    Some(user_props)
                }
            }),
        }
    }
}

impl IntoIterator for Unsubscribed {
    type Item = Result<(), ReasonCode>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.status.into_iter()
    }
}
