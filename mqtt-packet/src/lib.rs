//! The MQTT protocol works by exchanging a series of MQTT Control Packets in a defined way.
//!
//! This crate describes the format of these packets.
#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;

mod decode;
mod encode;
mod packet;

pub use crate::packet::*;
pub use crate::encode::WriteTo;
