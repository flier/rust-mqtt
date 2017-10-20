#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../.clippy.toml")))]

#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate nom;
extern crate rand;
extern crate byteorder;

#[macro_use]
extern crate serde_derive;
extern crate serde;

#[macro_use]
mod proto;
mod packet;
mod encode;
mod decode;

pub use decode::{ReadPacketExt, decode_variable_length_usize, read_packet};
pub use encode::WritePacketExt;
pub use packet::{ConnectAckFlags, ConnectFlags, ConnectReturnCode, FixedHeader, LastWill, Packet,
                 PacketId, PacketType, SubscribeReturnCode};
pub use proto::{ClientId, Protocol, QoS};
