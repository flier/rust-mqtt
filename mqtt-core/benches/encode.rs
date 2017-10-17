#![feature(test)]

extern crate test;
extern crate mqtt_core as core;

use test::Bencher;

use core::*;

#[bench]
fn bench_encode_connect_packets(b: &mut Bencher) {
    let p = Packet::Connect {
        protocol: Default::default(),
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
    };

    let mut v = Vec::new();

    b.iter(|| {
        v.clear();
        v.write_packet(&p).unwrap();
    });
}

#[bench]
fn bench_encode_publish_packets(b: &mut Bencher) {
    let p = Packet::Publish {
        dup: true,
        retain: true,
        qos: QoS::ExactlyOnce,
        topic: "topic",
        packet_id: Some(0x4321),
        payload: b"data",
    };

    let mut v = Vec::new();

    b.iter(|| {
        v.clear();
        v.write_packet(&p).unwrap();
    });
}

#[bench]
fn bench_encode_subscribe_packets(b: &mut Bencher) {
    let p = Packet::Subscribe {
        packet_id: 0x1234,
        topic_filters: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce)],
    };

    let mut v = Vec::new();

    b.iter(|| {
        v.clear();
        v.write_packet(&p).unwrap();
    });
}

#[bench]
fn bench_encode_subscribe_ack_packets(b: &mut Bencher) {
    let p = Packet::SubscribeAck {
        packet_id: 0x1234,
        status: vec![
            SubscribeReturnCode::Success(QoS::AtLeastOnce),
            SubscribeReturnCode::Failure,
            SubscribeReturnCode::Success(QoS::ExactlyOnce),
        ],
    };

    let mut v = Vec::new();

    b.iter(|| {
        v.clear();
        v.write_packet(&p).unwrap();
    });
}

#[bench]
fn bench_encode_unsubscribe_packets(b: &mut Bencher) {
    let p = Packet::Unsubscribe {
        packet_id: 0x1234,
        topic_filters: vec!["test", "filter"],
    };

    let mut v = Vec::new();

    b.iter(|| {
        v.clear();
        v.write_packet(&p).unwrap();
    });
}
