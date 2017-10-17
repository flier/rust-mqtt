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

// http://www.iana.org/assignments/service-names-port-numbers/service-names-port-numbers.xhtml
pub const TCP_PORT: u16 = 1883;
pub const SSL_PORT: u16 = 8883;

#[macro_export]
macro_rules! topic {
    ($s:expr) => ($s.parse::<Topic>().unwrap());
}
