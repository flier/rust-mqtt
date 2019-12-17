use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::{
    mqtt::{Property, Subscription, SubscriptionId},
    Protocol, MQTT_V5,
};

/// Subscribe create one or more Subscriptions. Each Subscription registers a Client’s interest in one or more Topics.
#[repr(transparent)]
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
    pub fn new<I, T>(packet_id: u16, subscriptions: I) -> Subscribe<'a, P>
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
