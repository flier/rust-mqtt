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
mod proto;
mod packet;
mod encode;
mod decode;

pub use proto::{Protocol, PacketId, ClientId, QoS};
pub use packet::{ConnectFlags, ConnectAckFlags, Packet, LastWill, ConnectReturnCode, FixedHeader,
                 PacketType, SubscribeReturnCode};
pub use encode::WritePacketExt;
pub use decode::{ReadPacketExt, read_packet, decode_variable_length_usize};
