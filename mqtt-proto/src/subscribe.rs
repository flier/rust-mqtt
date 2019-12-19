use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::{
    mqtt::{PacketId, Property, QoS, ReasonCode, Subscription, SubscriptionId},
    Protocol, MQTT_V5,
};

/// Subscribe create one or more Subscriptions. Each Subscription registers a Client’s interest in one or more Topics.
#[derive(Clone, Debug, PartialEq)]
pub struct Subscribe<'a, P>(mqtt::Subscribe<'a>, PhantomData<P>);

impl<'a, P> Deref for Subscribe<'a, P> {
    type Target = mqtt::Subscribe<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for Subscribe<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, P> Into<mqtt::Subscribe<'a>> for Subscribe<'a, P> {
    fn into(self) -> mqtt::Subscribe<'a> {
        self.0
    }
}

impl<'a, P> Subscribe<'a, P> {
    /// subscribe create one or more Subscriptions. Each Subscription registers a Client’s interest in one or more Topics.
    pub fn new<I, T>(packet_id: PacketId, subscriptions: I) -> Subscribe<'a, P>
    where
        P: Protocol,
        I: IntoIterator<Item = T>,
        T: Into<Subscription<'a>>,
    {
        Subscribe(
            mqtt::Subscribe {
                packet_id,
                properties: P::default_properties(),
                subscriptions: subscriptions.into_iter().map(|s| s.into()).collect(),
            },
            PhantomData,
        )
    }

    pub fn with_subscription<T: Into<Subscription<'a>>>(&mut self, subscription: T) -> &mut Self {
        self.subscriptions.push(subscription.into());
        self
    }
}

impl<'a> Subscribe<'a, MQTT_V5> {
    /// Configure property.
    pub fn with_property(&mut self, property: Property<'a>) -> &mut Self {
        self.properties.get_or_insert_with(Vec::new).push(property);
        self
    }

    /// Subscription Identifier
    pub fn with_subscription_id(&mut self, subscription_id: SubscriptionId) -> &mut Self {
        self.with_property(Property::SubscriptionId(subscription_id))
    }

    /// User Property
    pub fn with_user_property(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.with_property(Property::UserProperty(name, value))
    }
}

#[derive(Debug, Default)]
pub struct Subscribed {
    pub status: Vec<Result<QoS, ReasonCode>>,
    pub reason: Option<String>,
    pub user_properties: Vec<(String, String)>,
}

impl Subscribed {
    pub fn new<'a, S, I>(status: S, properties: I) -> Subscribed
    where
        S: IntoIterator<Item = Result<QoS, ReasonCode>>,
        I: IntoIterator<Item = Property<'a>>,
    {
        properties.into_iter().fold(
            Subscribed {
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

impl<'a> From<mqtt::SubscribeAck<'a>> for Subscribed {
    fn from(subscribe_ack: mqtt::SubscribeAck<'a>) -> Subscribed {
        Subscribed::new(
            subscribe_ack.status,
            subscribe_ack.properties.into_iter().flatten(),
        )
    }
}

impl IntoIterator for Subscribed {
    type Item = Result<QoS, ReasonCode>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.status.into_iter()
    }
}
