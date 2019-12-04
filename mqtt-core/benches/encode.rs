#[macro_use]
extern crate criterion;

use criterion::Criterion;

use mqtt_core::{packet::*, QoS};

fn bench_encode_connect_packets(c: &mut Criterion) {
    let p = Packet::Connect(Connect {
        clean_session: false,
        keep_alive: 60,
        client_id: "12345",
        last_will: Some(LastWill {
            qos: QoS::ExactlyOnce,
            retain: false,
            topic: "topic",
            message: b"message",
        }),
        username: None,
        password: None,
    });

    c.bench_function("encode_connect_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| p.write_to(&mut v))
    });
}

fn bench_encode_publish_packets(c: &mut Criterion) {
    let p = Packet::Publish(Publish {
        dup: true,
        retain: true,
        qos: QoS::ExactlyOnce,
        topic: "topic",
        packet_id: Some(0x4321),
        payload: b"data",
    });

    c.bench_function("encode_publish_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| p.write_to(&mut v))
    });
}

fn bench_encode_subscribe_packets(c: &mut Criterion) {
    let p = Packet::Subscribe(Subscribe {
        packet_id: 0x1234,
        subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce)],
    });

    c.bench_function("encode_subscribe_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| p.write_to(&mut v))
    });
}

fn bench_encode_subscribe_ack_packets(c: &mut Criterion) {
    let p = Packet::SubscribeAck(SubscribeAck {
        packet_id: 0x1234,
        status: vec![
            SubscribeReturnCode::Success(QoS::AtLeastOnce),
            SubscribeReturnCode::Failure,
            SubscribeReturnCode::Success(QoS::ExactlyOnce),
        ],
    });

    c.bench_function("encode_subscribe_ack_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| p.write_to(&mut v))
    });
}

fn bench_encode_unsubscribe_packets(c: &mut Criterion) {
    let p = Packet::Unsubscribe(Unsubscribe {
        packet_id: 0x1234,
        topic_filters: vec!["test", "filter"],
    });

    c.bench_function("encode_unsubscribe_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| p.write_to(&mut v))
    });
}

criterion_group!(
    encode,
    bench_encode_connect_packets,
    bench_encode_publish_packets,
    bench_encode_subscribe_packets,
    bench_encode_subscribe_ack_packets,
    bench_encode_unsubscribe_packets
);
criterion_main!(encode);
