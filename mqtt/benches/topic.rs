#![feature(test)]

extern crate test;
#[macro_use]
extern crate mqtt;

use test::Bencher;

use mqtt::*;

#[bench]
fn bench_parse_topic(b: &mut Bencher) {
    b.iter(|| topic!("$SYS/+/player1"))
}

#[bench]
fn bench_match_topic(b: &mut Bencher) {
    let t1 = topic!("sport/tennis/player1");
    let t2 = topic!("sport/+/player1");

    b.iter(|| t1.match_topic(&t2))
}

#[bench]
fn bench_match_topic_tree(b: &mut Bencher) {
    let tree = TopicTree::build(vec![
        topic!("sport/tennis/+"),
        topic!("sport/tennis/player1"),
        topic!("sport/tennis/player1/#"),
        topic!("sport/#"),
        topic!("sport/+"),
        topic!("#"),
        topic!("+"),
        topic!("+/+"),
        topic!("/+"),
        topic!("$SYS/#"),
        topic!("$SYS/monitor/+"),
        topic!("+/monitor/Clients"),
    ]);
    let t = topic!("sport/tennis/player1");

    b.iter(|| tree.match_topic(&t))
}
