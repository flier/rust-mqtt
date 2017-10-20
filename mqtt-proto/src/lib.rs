#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../.clippy.toml")))]

#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate bytes;
extern crate nom;

#[macro_use]
extern crate serde_derive;
extern crate serde;

extern crate futures;
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

pub mod errors;
mod codec;
mod proto;
pub mod server;

pub use codec::Codec;
pub use proto::Proto;
