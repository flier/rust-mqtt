#[macro_use]
extern crate bitflags;

mod decode;
mod encode;
pub mod packet;

#[doc(inline)]
pub use packet::Packet;
