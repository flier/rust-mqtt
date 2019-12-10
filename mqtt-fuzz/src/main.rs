#[macro_use]
extern crate afl;

fn main() {
    fuzz!(|data: &[u8]| {
        let _ = mqtt_packet::parse::<()>(data, mqtt_core::ProtocolVersion::V5);
    });
}
