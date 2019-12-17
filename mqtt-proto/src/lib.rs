#![allow(non_camel_case_types)]

#[macro_use]
extern crate log;
// #[macro_use]
// extern crate failure;
// #[macro_use]
// extern crate serde_derive;

pub extern crate mqtt_core as mqtt;

mod connect;
mod disconnect;
mod props;
mod subscribe;

pub use crate::connect::Connect;
pub use crate::disconnect::Disconnect;
pub use crate::props::ServerProperties;
pub use crate::subscribe::Subscribe;

use crate::mqtt::{Property, ProtocolVersion};

/// MQTT protocol
pub trait Protocol {
    const VERSION: ProtocolVersion;

    fn default_properties<'a>() -> Option<Vec<Property<'a>>>;
}

/// MQTT v3.1.1
#[derive(Debug, PartialEq)]
pub struct MQTT_V311;

impl Protocol for MQTT_V311 {
    const VERSION: ProtocolVersion = ProtocolVersion::V311;

    fn default_properties<'a>() -> Option<Vec<Property<'a>>> {
        None
    }
}

/// MQTT v5.0
#[derive(Debug, PartialEq)]
pub struct MQTT_V5;

impl Protocol for MQTT_V5 {
    const VERSION: ProtocolVersion = ProtocolVersion::V5;

    fn default_properties<'a>() -> Option<Vec<Property<'a>>> {
        Some(Vec::new())
    }
}

// #[cfg(test)]
// #[macro_use]
// extern crate matches;
// #[cfg(test)]
// #[macro_use]
// extern crate lazy_static;

// mod codec;
// pub mod errors;
// mod proto;
// #[macro_use]
// mod topic;
// mod message;
// pub mod server;

// pub use codec::Codec;
// pub use message::{Message, MessageReceiver, MessageSender};
// pub use proto::MQTT;
// pub use topic::{Filter, Level, MatchTopic};
