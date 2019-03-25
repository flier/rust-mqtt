#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
extern crate bytes;
extern crate nom;
extern crate rotor;
extern crate slab;

pub extern crate mqtt_core as core;
pub extern crate mqtt_proto as proto;

pub mod errors;
pub mod client;
pub mod server;
pub mod transport;

/// TCP ports 1883 was registered with IANA for MQTT non TLS communication respectively.
pub const TCP_PORT: u16 = 1883;
/// TCP ports 8883 was registered with IANA for MQTT TLS communication respectively.
pub const SSL_PORT: u16 = 8883;

#[macro_export]
macro_rules! topic {
    ($s:expr) => {
        $s.parse::<$crate::proto::Filter>().unwrap()
    };
}
