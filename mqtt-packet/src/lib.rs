//! The MQTT protocol works by exchanging a series of MQTT Control Packets in a defined way.
//!
//! This crate describes the format of these packets.
#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;

extern crate mqtt_core as mqtt;

mod decode;
mod encode;
mod packet;

pub use crate::decode::parse;
pub use crate::encode::WriteTo;
pub use crate::packet::*;
