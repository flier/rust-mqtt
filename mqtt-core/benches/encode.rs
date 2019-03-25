#[macro_use]
extern crate criterion;

use criterion::Criterion;

extern crate mqtt_core;

use mqtt_core::*;

fn bench_encode_connect_packets(c: &mut Criterion) {
    let p = Packet::Connect {
        protocol: Default::default(),
        clean_session: false,
        keep_alive: 60,
        client_id: "12345".into(),
        last_will: Some(LastWill {
            qos: QoS::ExactlyOnce,
            retain: false,
            topic: "topic".into(),
            message: (&b"message"[..]).into(),
        }),
        username: None,
        password: None,
    };

    c.bench_function("encode_connect_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| {
            v.clear();
            v.write_packet(&p).unwrap();
        })
    });
}

fn bench_encode_publish_packets(c: &mut Criterion) {
    let p = Packet::Publish {
        dup: true,
        retain: true,
        qos: QoS::ExactlyOnce,
        topic: "topic".into(),
        packet_id: Some(0x4321),
        payload: (&b"data"[..]).into(),
    };

    c.bench_function("encode_publish_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| {
            v.clear();
            v.write_packet(&p).unwrap();
        })
    });
}

fn bench_encode_subscribe_packets(c: &mut Criterion) {
    let p = Packet::Subscribe {
        packet_id: 0x1234,
        topic_filters: vec![
            ("test".into(), QoS::AtLeastOnce),
            ("filter".into(), QoS::ExactlyOnce),
        ],
    };

    c.bench_function("encode_subscribe_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| {
            v.clear();
            v.write_packet(&p).unwrap();
        })
    });
}

fn bench_encode_subscribe_ack_packets(c: &mut Criterion) {
    let p = Packet::SubscribeAck {
        packet_id: 0x1234,
        status: vec![
            SubscribeReturnCode::Success(QoS::AtLeastOnce),
            SubscribeReturnCode::Failure,
            SubscribeReturnCode::Success(QoS::ExactlyOnce),
        ],
    };

    c.bench_function("encode_subscribe_ack_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| {
            v.clear();
            v.write_packet(&p).unwrap();
        })
    });
}

fn bench_encode_unsubscribe_packets(c: &mut Criterion) {
    let p = Packet::Unsubscribe {
        packet_id: 0x1234,
        topic_filters: vec!["test".into(), "filter".into()],
    };

    c.bench_function("encode_unsubscribe_packets", move |b| {
        let mut v = Vec::new();

        b.iter(|| {
            v.clear();
            v.write_packet(&p).unwrap();
        })
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
