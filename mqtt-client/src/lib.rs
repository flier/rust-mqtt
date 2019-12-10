#[macro_use]
extern crate log;

pub extern crate mqtt_core as mqtt;
pub extern crate mqtt_packet as packet;
pub extern crate mqtt_proto as proto;

#[cfg(feature = "async")]
pub mod r#async;
pub mod sync;

#[cfg(feature = "async")]
pub use r#async::connect as connect_async;
pub use sync::connect;
