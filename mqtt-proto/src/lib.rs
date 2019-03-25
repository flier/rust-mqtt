#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../.clippy.toml")))]

#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate bytes;
extern crate itertools;
extern crate nom;
extern crate pwhash;
extern crate serde;
extern crate slab;
extern crate time;
extern crate void;
#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;

extern crate mqtt_core as core;

#[cfg(test)]
#[macro_use]
extern crate matches;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

mod codec;
pub mod errors;
mod proto;
#[macro_use]
mod topic;
mod message;
pub mod server;

pub use codec::Codec;
pub use message::{Message, MessageReceiver, MessageSender};
pub use proto::MQTT;
pub use topic::{Filter, Level, MatchTopic};
