use std::collections::HashMap;

use slab::Slab;

use futures::unsync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

use errors::{Error, ErrorKind, Result};
use topic::{Filter, Level};

type SubscriberIdx = usize;
type NodeIdx = usize;

#[derive(Debug)]
pub struct Subscriber<T> {
    filter: Filter,
    sender: UnboundedSender<T>,
}

#[derive(Debug)]
pub struct Subscribed<T> {
    node_idx: NodeIdx,
    subscriber_idx: SubscriberIdx,
    receiver: UnboundedReceiver<T>,
}

#[derive(Debug, Default)]
struct Node {
    next: HashMap<Level, NodeIdx>,
    subscribers: Vec<SubscriberIdx>,
    multi_wildcard: Vec<SubscriberIdx>,
}

#[derive(Debug)]
pub struct Subscription<T> {
    subscribers: Slab<Subscriber<T>>,
    nodes: Slab<Node>,
    root: NodeIdx,
}

impl<T> Default for Subscription<T> {
    fn default() -> Self {
        let mut nodes = Slab::with_capacity(64);
        let root = nodes.insert(Default::default());

        Subscription {
            subscribers: Slab::with_capacity(64),
            nodes: nodes,
            root: root,
        }
    }
}

impl<T> Subscription<T> {
    pub fn subscribe(&mut self, filter: &Filter) -> Subscribed<T> {
        let (sender, receiver) = unbounded();
        let subscriber_idx = self.subscribers.insert(Subscriber {
            filter: filter.clone(),
            sender,
        });
        let mut cur_node_idx = self.root;

        for level in filter.levels() {
            match *level {
                Level::Normal(_) |
                Level::Metadata(_) |
                Level::Blank |
                Level::SingleWildcard => {
                    cur_node_idx = self.nodes[cur_node_idx]
                        .next
                        .get(level)
                        .cloned()
                        .unwrap_or_else(|| self.nodes.insert(Default::default()))
                }
                Level::MultiWildcard => {
                    self.nodes[cur_node_idx].multi_wildcard.push(subscriber_idx);

                    break;
                }
            }
        }

        let node = &mut self.nodes[cur_node_idx];

        if !node.multi_wildcard.contains(&subscriber_idx) {
            node.subscribers.push(subscriber_idx);
        }

        Subscribed {
            node_idx: cur_node_idx,
            subscriber_idx,
            receiver,
        }
    }

    pub fn unsubscribe(&mut self, mut subscribed: Subscribed<T>) {
        if let Some(node) = self.nodes.get_mut(subscribed.node_idx) {
            node.subscribers.remove(subscribed.subscriber_idx);
            node.multi_wildcard.remove(subscribed.subscriber_idx);
        }

        self.subscribers.remove(subscribed.subscriber_idx);

        subscribed.receiver.close();
    }

    pub fn subscribers<S: AsRef<str>>(&self, topic: S) -> Result<Vec<&Subscriber<T>>> {
        let filter: Filter = topic.as_ref().parse()?;
        let mut subscribers = vec![];
        let mut cur_node_idx = self.root;

        for level in filter.levels() {}

        Ok(subscribers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription() {
        let mut subscription = Subscription::default();

        let filters = vec![
            topic_filter!("sport/tennis/+"),
            topic_filter!("sport/tennis/player1"),
            topic_filter!("sport/tennis/player1/#"),
            topic_filter!("sport/#"),
            topic_filter!("sport/+"),
            topic_filter!("#"),
            topic_filter!("+"),
            topic_filter!("+/+"),
            topic_filter!("/+"),
            topic_filter!("$SYS/#"),
            topic_filter!("$SYS/monitor/+"),
            topic_filter!("+/monitor/Clients"),
        ];

        let subscribed = filters
            .iter()
            .map(|filter| subscription.subscribe(filter))
            .collect::<Vec<Subscribed<()>>>();

        assert_eq!(filters.len(), subscribed.len());
    }


    // #[test]
    // fn test_topic_tree() {
    //     let subscription = Tree::from_iter(vec![
    //         topic_filter!("sport/tennis/+"),
    //         topic_filter!("sport/tennis/player1"),
    //         topic_filter!("sport/tennis/player1/#"),
    //         topic_filter!("sport/#"),
    //         topic_filter!("sport/+"),
    //         topic_filter!("#"),
    //         topic_filter!("+"),
    //         topic_filter!("+/+"),
    //         topic_filter!("/+"),
    //         topic_filter!("$SYS/#"),
    //         topic_filter!("$SYS/monitor/+"),
    //         topic_filter!("+/monitor/Clients"),
    //     ]);

    //     assert_eq!(subscription.filters.len(), 12);
    //     assert_eq!(subscription.nodes.len(), 15);

    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("sport/tennis/player1")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("sport/#"),
    //             &topic_filter!("sport/tennis/player1/#"),
    //             &topic_filter!("sport/tennis/player1"),
    //             &topic_filter!("sport/tennis/+"),
    //         ])
    //     );
    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("sport/tennis/player1/ranking")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("sport/#"),
    //             &topic_filter!("sport/tennis/player1/#"),
    //         ])
    //     );
    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("sport/tennis/player1/score/wimbledo")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("sport/#"),
    //             &topic_filter!("sport/tennis/player1/#"),
    //         ])
    //     );
    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("sport")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("sport/#"),
    //             &topic_filter!("+"),
    //         ])
    //     );
    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("sport/")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("sport/#"),
    //             &topic_filter!("sport/+"),
    //             &topic_filter!("+/+"),
    //         ])
    //     );
    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("/finance")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("/+"),
    //             &topic_filter!("+/+"),
    //         ])
    //     );

    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("$SYS/monitor/Clients")),
    //         Some(vec![
    //             &topic_filter!("$SYS/#"),
    //             &topic_filter!("$SYS/monitor/+"),
    //         ])
    //     );
    //     assert_eq!(
    //         subscription.match_topic(&topic_filter!("/monitor/Clients")),
    //         Some(vec![
    //             &topic_filter!("#"),
    //             &topic_filter!("+/monitor/Clients"),
    //         ])
    //     );
    // }
}