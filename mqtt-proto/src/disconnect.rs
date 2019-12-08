use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

pub use crate::{
    packet::{Expiry, LastWill, Packet, Property, ProtocolVersion, QoS, ReasonCode},
    MQTT_V5,
};

/// Disconnect from the broker.
pub fn disconnect<'a, P>() -> Disconnect<'a, P> {
    Disconnect(
        packet::Disconnect {
            reason_code: None,
            properties: None,
        },
        PhantomData,
    )
}

/// Disconnect notification
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct Disconnect<'a, P>(packet::Disconnect<'a>, PhantomData<P>);

impl<'a, P> Deref for Disconnect<'a, P> {
    type Target = packet::Disconnect<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> DerefMut for Disconnect<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, P> From<Disconnect<'a, P>> for Packet<'a> {
    fn from(disconnect: Disconnect<'a, P>) -> packet::Packet<'a> {
        Packet::Disconnect(disconnect.0)
    }
}

impl<'a> Disconnect<'a, MQTT_V5> {
    /// Disconnect Reason Code
    pub fn with_reason_code(&mut self, reason_code: ReasonCode) -> &mut Self {
        self.reason_code = Some(reason_code);
        self
    }

    /// Configure property.
    pub fn with_property(&mut self, property: Property<'a>) -> &mut Self {
        self.properties.get_or_insert_with(Vec::new).push(property);
        self
    }

    /// Session Expiry Interval
    pub fn with_session_expiry(&mut self, expiry: Expiry) -> &mut Self {
        self.with_property(Property::SessionExpiryInterval(expiry))
    }
    /// Reason String
    ///
    /// This Reason String is human readable, designed for diagnostics and SHOULD NOT be parsed by the receiver.
    pub fn with_reason(&mut self, reason: &'a str) -> &mut Self {
        self.with_property(Property::Reason(reason))
    }

    /// User Property
    pub fn with_user_property(&mut self, name: &'a str, value: &'a str) -> &mut Self {
        self.with_property(Property::UserProperty(name, value))
    }

    /// Server Reference
    pub fn with_server_reference(&mut self, server: &'a str) -> &mut Self {
        self.with_property(Property::ServerReference(server))
    }
}
