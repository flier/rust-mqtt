#[macro_use]
extern crate afl;

use mqtt_core::Packet;

fn main() {
    fuzz!(|data: &[u8]| {
        let _ = Packet::parse::<()>(data);
    });
}
