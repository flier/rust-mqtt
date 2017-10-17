#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate bytes;
extern crate nom;
extern crate slab;
extern crate rotor;

extern crate mqtt_core as core;

mod error;
#[macro_use]
mod topic;

pub use core::*;
pub use topic::{Level, Topic, TopicTree, MatchTopic};

pub mod transport;
pub mod server;
pub mod client;

/// TCP ports 1883 was registered with IANA for MQTT non TLS communication respectively.
pub const TCP_PORT: u16 = 1883;
/// TCP ports 8883 was registered with IANA for MQTT TLS communication respectively.
pub const SSL_PORT: u16 = 8883;

#[macro_export]
macro_rules! topic {
    ($s:expr) => ($s.parse::<Topic>().unwrap());
}
