use std::fmt::{self, Display, Formatter};
use std::iter;
use std::ops::Deref;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[doc(hidden)]
#[macro_export]
macro_rules! const_enum {
    ($name:ty : $repr:ty) => {
        impl ::std::convert::From<$repr> for $name {
            fn from(u: $repr) -> Self {
                unsafe { ::std::mem::transmute(u) }
            }
        }

        impl ::std::convert::From<$name> for $repr {
            fn from(u: $name) -> Self {
                unsafe { ::std::mem::transmute(u) }
            }
        }
    };
}

pub const DEFAULT_MQTT_LEVEL: u8 = 4;

/// MQTT Protocol name and level
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
        Protocol::MQTT(DEFAULT_MQTT_LEVEL)
    }
}

/// Quality of Service levels
#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Reserved – must not be used
    Reserved = 3,
}

const_enum!(QoS: u8);

/// The Server MUST allow ClientIds which are between 1 and 23 UTF-8 encoded bytes in length,
/// and that contain only the characters [MQTT-3.1.3-5].

pub const MAX_CLIENT_ID_LENGTH: usize = 23;
pub const CLIENT_ID_CHARS: &[u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// A unique Client identifier for the Client
#[derive(Debug, PartialEq, Clone, Default)]
pub struct ClientId(String);

impl ClientId {
    pub fn new() -> ClientId {
        Self::with_size(16)
    }

    /// 3.1.3.1 Client Identifier
    pub fn is_valid<S: AsRef<str>>(s: S) -> bool {
        let s = s.as_ref();

        !s.is_empty()
            && s.len() <= MAX_CLIENT_ID_LENGTH
            && s.bytes().all(|b| CLIENT_ID_CHARS.contains(&b))
    }

    pub fn with_size(size: usize) -> ClientId {
        let mut rng = thread_rng();

        ClientId(
            iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(size)
                .collect(),
        )
    }

    pub fn as_ref(&self) -> &str {
        self.0.as_ref()
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
