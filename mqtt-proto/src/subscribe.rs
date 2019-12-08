use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::{
    packet::{Packet, Property, ProtocolVersion, Subscription, SubscriptionId},
    Protocol, MQTT_V5,
};

/// subscribe create one or more Subscriptions. Each Subscription registers a Client’s interest in one or more Topics.
pub fn subscribe<'a, P, I, T>(packet_id: u16, subscriptions: I) -> Subscribe<'a, P>
where
    P: Protocol,
    I: IntoIterator<Item = T>,
    T: Into<Subscription<'a>>,
{
    Subscribe(
        packet::Subscribe {
            packet_id,
            properties: if P::VERSION >= ProtocolVersion::V5 {
                Some(Vec::new())
            } else {
                None
            },
            subscriptions: subscriptions.into_iter().map(|s| s.into()).collect(),
        },
        PhantomData,
    )
}

/// Subscribe create one or more Subscriptions. Each Subscription registers a Client’s interest in one or more Topics.
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct Subscribe<'a, P>(packet::Subscribe<'a>, PhantomData<P>);

impl<'a, P> Deref for Subscribe<'a, P> {
    type Target = packet::Subscribe<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for Subscribe<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, P> From<Subscribe<'a, P>> for Packet<'a> {
    fn from(subscribe: Subscribe<'a, P>) -> packet::Packet<'a> {
        Packet::Subscribe(subscribe.0)
    }
}

impl<'a, P> Subscribe<'a, P> {
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
