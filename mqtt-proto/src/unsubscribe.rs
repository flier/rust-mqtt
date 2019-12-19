use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::{
    mqtt::{Property, ReasonCode},
    Protocol, MQTT_V5,
};

/// An UNSUBSCRIBE packet is sent by the Client to the Server, to unsubscribe from topics.
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

#[derive(Debug, Default)]
pub struct Unsubscribed {
    pub status: Vec<Result<(), ReasonCode>>,
    pub reason: Option<String>,
    pub user_properties: Vec<(String, String)>,
}

impl Unsubscribed {
    pub fn new<'a, S, I>(status: S, properties: I) -> Self
    where
        S: IntoIterator<Item = Result<(), ReasonCode>>,
        I: IntoIterator<Item = Property<'a>>,
    {
        properties.into_iter().fold(
            Unsubscribed {
                status: status.into_iter().collect(),
                ..Default::default()
            },
            |mut subscribed, prop| {
                match prop {
                    Property::Reason(reason) => subscribed.reason = Some(reason.to_string()),
                    Property::UserProperty(name, value) => subscribed
                        .user_properties
                        .push((name.to_string(), value.to_string())),
                    _ => {}
                }

                subscribed
            },
        )
    }
}

impl<'a> From<mqtt::UnsubscribeAck<'a>> for Unsubscribed {
    fn from(unsubscribe_ack: mqtt::UnsubscribeAck) -> Self {
        Unsubscribed::new(
            unsubscribe_ack.status.into_iter().flatten(),
            unsubscribe_ack.properties.into_iter().flatten(),
        )
    }
}

impl IntoIterator for Unsubscribed {
    type Item = Result<(), ReasonCode>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.status.into_iter()
    }
}
