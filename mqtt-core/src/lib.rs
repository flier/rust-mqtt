#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate nom;
extern crate byteorder;
extern crate rand;
extern crate serde;

#[cfg(test)]
#[macro_use]
extern crate matches;

#[macro_use]
mod proto;
mod decode;
mod encode;
mod packet;

pub use decode::{decode_variable_length_usize, read_packet, ReadPacketExt};
pub use encode::WritePacketExt;
pub use packet::{
    ConnectAckFlags, ConnectFlags, ConnectReturnCode, FixedHeader, LastWill, Packet, PacketId,
    PacketType, SubscribeReturnCode,
};
pub use proto::{ClientId, Protocol, QoS};
