#[macro_use]
extern crate criterion;

use criterion::Criterion;

extern crate mqtt_core;

use mqtt_core::*;

fn bench_decode_connect_packets(c: &mut Criterion) {
    let buf = b"\x10\x1D\x00\x04MQTT\x04\xC0\x00\x3C\x00\x0512345\x00\x04user\x00\x04pass";

    c.bench_function("decode_connect_packets", move |b| {
        b.iter(|| buf.read_packet().unwrap())
    });
}

fn bench_decode_connect_ack_packets(c: &mut Criterion) {
    let buf = b"\x20\x02\x01\x04";

    c.bench_function("decode_connect_ack_packets", move |b| {
        b.iter(|| buf.read_packet().unwrap())
    });
}

fn bench_decode_publish_packets(c: &mut Criterion) {
    let buf = b"\x3d\x0D\x00\x05topic\x43\x21data";

    c.bench_function("decode_publish_packets", move |b| {
        b.iter(|| buf.read_packet().unwrap())
    });
}

fn bench_decode_subscribe_packets(c: &mut Criterion) {
    let buf = b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02";

    c.bench_function("decode_subscribe_packets", move |b| {
        b.iter(|| buf.read_packet().unwrap())
    });
}

fn bench_decode_subscribe_ack_packets(c: &mut Criterion) {
    let buf = b"\x90\x05\x12\x34\x01\x80\x02";

    c.bench_function("decode_subscribe_ack_packets", move |b| {
        b.iter(|| buf.read_packet().unwrap())
    });
}

fn bench_decode_unsubscribe_packets(c: &mut Criterion) {
    let buf = b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter";

    c.bench_function("decode_unsubscribe_packets", move |b| {
        b.iter(|| buf.read_packet().unwrap())
    });
}

criterion_group!(
    decode,
    bench_decode_connect_packets,
    bench_decode_connect_ack_packets,
    bench_decode_publish_packets,
    bench_decode_subscribe_packets,
    bench_decode_subscribe_ack_packets,
    bench_decode_unsubscribe_packets
);
criterion_main!(decode);
