#[macro_use]
extern crate criterion;
#[macro_use]
extern crate mqtt;

use criterion::Criterion;

use mqtt::proto::MatchTopic;
use mqtt::*;

fn bench_parse_topic(c: &mut Criterion) {
    c.bench_function("parse_topic", |b| {
        b.iter(|| {
            let _ = topic!("$SYS/+/player1");
        })
    });
}

fn bench_match_topic(c: &mut Criterion) {
    c.bench_function("match_topic", move |b| {
        let t1 = topic!("sport/tennis/player1");
        let t2 = topic!("sport/+/player1");

        b.iter(|| {
            let _ = t1.match_topic(&t2);
        })
    });
}

// fn bench_match_topic_tree(c: &mut Criterion) {
//     let tree = TopicTree::build(vec![
//         topic!("sport/tennis/+"),
//         topic!("sport/tennis/player1"),
//         topic!("sport/tennis/player1/#"),
//         topic!("sport/#"),
//         topic!("sport/+"),
//         topic!("#"),
//         topic!("+"),
//         topic!("+/+"),
//         topic!("/+"),
//         topic!("$SYS/#"),
//         topic!("$SYS/monitor/+"),
//         topic!("+/monitor/Clients"),
//     ]);
//     let t = topic!("sport/tennis/player1");

//     c.bench_function("match_topic_tree", move |b| tree.match_topic(&t));
// }

criterion_group!(
    topic,
    bench_parse_topic,
    bench_match_topic,
    // bench_match_topic_tree,
);
criterion_main!(topic);
