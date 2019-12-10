#[macro_use]
extern crate criterion;

use criterion::Criterion;

use mqtt_core::*;
use mqtt_packet::*;

fn bench_encode_connect_packets(c: &mut Criterion) {
    let p = Packet::Connect(Connect {
        protocol_version: ProtocolVersion::V311,
        clean_session: false,
        keep_alive: 60,
        client_id: "12345",
        last_will: Some(LastWill {
            qos: QoS::ExactlyOnce,
            retain: false,
            topic_name: "topic",
            message: b"message",
            properties: None,
        }),
        username: None,
        password: None,
        properties: None,
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
        topic_name: "topic",
        packet_id: Some(0x4321),
        properties: None,
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
        subscriptions: vec![
            ("test", QoS::AtLeastOnce).into(),
            ("filter", QoS::ExactlyOnce).into(),
        ],
        properties: None,
    });

    c.bench_function("encode_subscribe_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| p.write_to(&mut v))
    });
}

fn bench_encode_subscribe_ack_packets(c: &mut Criterion) {
    let p = Packet::SubscribeAck(SubscribeAck {
        packet_id: 0x1234,
        properties: None,
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
        properties: None,
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
