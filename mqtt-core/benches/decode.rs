#![feature(test)]

extern crate test;
extern crate mqtt_core as core;

use test::Bencher;

use core::*;

#[bench]
fn bench_decode_connect_packets(b: &mut Bencher) {
    let buf = b"\x10\x1D\x00\x04MQTT\x04\xC0\x00\x3C\x00\
\x0512345\x00\x04user\x00\x04pass";

    b.iter(|| buf.read_packet().unwrap());
}

#[bench]
fn bench_decode_connect_ack_packets(b: &mut Bencher) {
    let buf = b"\x20\x02\x01\x04";

    b.iter(|| buf.read_packet().unwrap());
}

#[bench]
fn bench_decode_publish_packets(b: &mut Bencher) {
    let buf = b"\x3d\x0D\x00\x05topic\x43\x21data";

    b.iter(|| buf.read_packet().unwrap());
}

#[bench]
fn bench_decode_subscribe_packets(b: &mut Bencher) {
    let buf = b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02";

    b.iter(|| buf.read_packet().unwrap());
}

#[bench]
fn bench_decode_subscribe_ack_packets(b: &mut Bencher) {
    let buf = b"\x90\x05\x12\x34\x01\x80\x02";

    b.iter(|| buf.read_packet().unwrap());
}

#[bench]
fn bench_decode_unsubscribe_packets(b: &mut Bencher) {
    let buf = b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter";

    b.iter(|| buf.read_packet().unwrap());
}
