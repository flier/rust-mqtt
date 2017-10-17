use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use rand::{thread_rng, Rng};

#[macro_export]
macro_rules! const_enum {
    ($name:ty : $repr:ty) => {
        impl ::std::convert::From<$repr> for $name {
            fn from(u: $repr) -> Self {
                unsafe { ::std::mem::transmute(u) }
            }
        }

        impl ::std::convert::Into<$repr> for $name {
            fn into(self) -> $repr {
                unsafe { ::std::mem::transmute(self) }
            }
        }
    }
}

pub const DEFAULT_MQTT_LEVEL: u8 = 4;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    MQTT(u8),
}

impl Protocol {
    pub fn name(&self) -> &'static str {
        match *self {
            Protocol::MQTT(_) => "MQTT",
        }
    }

    pub fn level(&self) -> u8 {
        match *self {
            Protocol::MQTT(level) => level,
        }
    }
}

impl Default for Protocol {
    fn default() -> Self {
        return Protocol::MQTT(DEFAULT_MQTT_LEVEL);
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Quality of Service levels
pub enum QoS {
    /// At most once delivery
    ///
    /// The message is delivered according to the capabilities of the underlying network.
    /// No response is sent by the receiver and no retry is performed by the sender.
    /// The message arrives at the receiver either once or not at all.
    AtMostOnce = 0,
    /// At least once delivery
    ///
    /// This quality of service ensures that the message arrives at the receiver at least once.
    /// A QoS 1 PUBLISH Packet has a Packet Identifier in its variable header
    /// and is acknowledged by a PUBACK Packet.
    AtLeastOnce = 1,
    /// Exactly once delivery
    ///
    /// This is the highest quality of service,
    /// for use when neither loss nor duplication of messages are acceptable.
    /// There is an increased overhead associated with this quality of service.
    ExactlyOnce = 2,
}

const_enum!(QoS: u8);

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ClientId(String);

impl ClientId {
    pub fn new() -> ClientId {
        Self::with_size(16)
    }

    pub fn with_size(size: usize) -> ClientId {
        ClientId(thread_rng().gen_ascii_chars().take(size).collect())
    }
}

impl Deref for ClientId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ClientId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub type PacketId = u16;

#[derive(Debug, PartialEq, Clone)]
pub struct Message<'a> {
    pub topic: &'a str,
    pub payload: &'a [u8],
    pub qos: QoS,
}
