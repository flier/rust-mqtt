use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec;

use slab::Slab;

use futures::{Poll, Sink, StartSend, Stream};
use futures::sync::mpsc::{SendError, UnboundedReceiver, UnboundedSender, unbounded};

use errors::Result;
use topic::{Filter, Level};

type SubscriberIdx = usize;
type NodeIdx = usize;

#[derive(Debug)]
pub struct Subscriber<T> {
    filter: Filter,
    sender: UnboundedSender<T>,
}

impl<T> Subscriber<T> {
    pub fn filter(&self) -> &Filter {
        &self.filter
    }

    pub fn send(&self, msg: T) -> Result<()> {
        self.sender.unbounded_send(msg)?;

        Ok(())
    }
}

impl<T> Sink for Subscriber<T> {
    type SinkItem = T;
    type SinkError = SendError<T>;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sender.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sender.poll_complete()
    }
}

#[derive(Debug)]
pub struct Subscribed<T> {
    node_idx: NodeIdx,
    subscriber_idx: SubscriberIdx,
    receiver: UnboundedReceiver<T>,
}

impl<T> Drop for Subscribed<T> {
    fn drop(&mut self) {
        self.receiver.close()
    }
}

impl<T> Stream for Subscribed<T> {
    type Item = T;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.receiver.poll()
    }
}

#[derive(Debug, Default)]
struct Node {
    next: HashMap<Level, NodeIdx>,
    subscribers: Vec<SubscriberIdx>,
    single_wildcard: Option<NodeIdx>,
    multi_wildcard: Vec<SubscriberIdx>,
}

#[derive(Debug)]
struct Inner<T> {
    subscribers: Slab<Arc<Subscriber<T>>>,
    nodes: Slab<Node>,
    root: NodeIdx,
}

#[derive(Clone, Debug)]
pub struct Subscription<T> {
    inner: Arc<Mutex<Inner<T>>>,
}

impl<T> Default for Subscription<T> {
    fn default() -> Self {
        let mut nodes = Slab::with_capacity(64);
        let root = nodes.insert(Default::default());

        Subscription {
            inner: Arc::new(Mutex::new(Inner {
                subscribers: Slab::with_capacity(64),
                nodes: nodes,
                root: root,
            })),
        }
    }
}

impl<T> Subscription<T> {
    pub fn subscribe<F: Into<Filter>>(&mut self, filter: F) -> Result<Subscribed<T>> {
        Ok(self.inner.lock()?.subscribe(filter))
    }

    pub fn unsubscribe(&mut self, subscribed: Subscribed<T>) -> Result<Arc<Subscriber<T>>> {
        Ok(self.inner.lock()?.unsubscribe(subscribed))
    }

    pub fn topic_subscribers<S: AsRef<str>>(&self, topic: S) -> Result<TopicSubscribers<T>> {
        let levels = match topic.as_ref().parse::<Filter>() {
            Ok(filter) => filter.into(),
            _ => Vec::new(),
        };

        let root = self.inner.lock()?.root;

        Ok(TopicSubscribers {
            inner: Arc::clone(&self.inner),
            next_nodes: vec![(levels.into_iter(), root)],
            subscribers: vec![],
        })
    }

    pub fn is_empty(&self) -> Result<bool> {
        let inner = self.inner.lock()?;

        Ok(inner.is_node_empty(inner.root))
    }


    pub fn purge(&mut self) -> Result<()> {
        let mut inner = self.inner.lock()?;

        let root = inner.root;

        Ok(inner.purge_node(root))
    }
}

impl<T> Inner<T> {
    pub fn subscribe<F: Into<Filter>>(&mut self, filter: F) -> Subscribed<T> {
        let filter = filter.into();
        let (sender, receiver) = unbounded();
        let subscriber_idx = self.subscribers.insert(Arc::new(Subscriber {
            filter: filter.clone(),
            sender,
        }));
        let mut cur_node_idx = self.root;

        for level in filter.levels() {
            cur_node_idx = match *level {
                Level::Normal(_) |
                Level::Metadata(_) |
                Level::Blank => {
                    self.nodes[cur_node_idx]
                        .next
                        .get(level)
                        .cloned()
                        .unwrap_or_else(|| {
                            let next_node_idx = self.nodes.insert(Default::default());

                            self.nodes[cur_node_idx].next.insert(
                                level.clone(),
                                next_node_idx,
                            );

                            next_node_idx
                        })
                }
                Level::SingleWildcard => {
                    if let Some(next_node_idx) = self.nodes[cur_node_idx].single_wildcard {
                        next_node_idx
                    } else {
                        let next_node_idx = self.nodes.insert(Default::default());

                        self.nodes[cur_node_idx].single_wildcard = Some(next_node_idx);

                        next_node_idx
                    }
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

    pub fn unsubscribe(&mut self, mut subscribed: Subscribed<T>) -> Arc<Subscriber<T>> {
        if let Some(node) = self.nodes.get_mut(subscribed.node_idx) {
            if let Some(index) = node.subscribers.iter().position(|&subscriber_idx| {
                subscriber_idx == subscribed.subscriber_idx
            })
            {
                node.subscribers.remove(index);
            }

            if let Some(index) = node.multi_wildcard.iter().position(|&subscriber_idx| {
                subscriber_idx == subscribed.subscriber_idx
            })
            {
                node.multi_wildcard.remove(index);
            }
        }

        subscribed.receiver.close();

        self.subscribers.remove(subscribed.subscriber_idx)
    }

    fn is_node_empty(&self, node_idx: NodeIdx) -> bool {
        let node = &self.nodes[node_idx];

        node.subscribers.is_empty() &&
            node.single_wildcard.map_or(
                true,
                |idx| self.is_node_empty(idx),
            ) && node.multi_wildcard.is_empty() &&
            node.next.iter().all(|(_, &idx)| self.is_node_empty(idx))
    }

    fn purge_node(&mut self, node_idx: NodeIdx) {
        let nodes = self.nodes[node_idx]
            .next
            .iter()
            .map(|(level, node_idx)| (level.clone(), *node_idx))
            .collect::<Vec<(Level, NodeIdx)>>();

        for (level, next_node_idx) in nodes {
            self.purge_node(next_node_idx);

            if self.is_node_empty(next_node_idx) {
                self.nodes[node_idx].next.remove(&level);
                self.nodes.remove(next_node_idx);
            }
        }

        if let Some(next_node_idx) = self.nodes[node_idx].single_wildcard {
            self.purge_node(next_node_idx);

            if self.is_node_empty(next_node_idx) {
                self.nodes[node_idx].single_wildcard = None;
                self.nodes.remove(next_node_idx);
            }
        }
    }
}

pub struct TopicSubscribers<T> {
    inner: Arc<Mutex<Inner<T>>>,
    next_nodes: Vec<(vec::IntoIter<Level>, NodeIdx)>,
    subscribers: Vec<SubscriberIdx>,
}

impl<T> Iterator for TopicSubscribers<T> {
    type Item = Arc<Subscriber<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscribers
            .pop()
            .or_else(|| {
                while let Some((mut levels, next_node_idx)) = self.next_nodes.pop() {
                    if let Ok(inner) = self.inner.lock() {
                        if let Some(node) = inner.nodes.get(next_node_idx) {
                            if let Some(level) = levels.next() {
                                match node.single_wildcard {
                                    Some(next_node_idx) if !level.is_metadata() => {
                                        self.next_nodes.push((levels.clone(), next_node_idx))
                                    }
                                    _ => {}
                                }

                                match level {
                                    Level::Normal(_) |
                                    Level::Metadata(_) |
                                    Level::Blank => {
                                        if let Some(&next_node_idx) = node.next.get(&level) {
                                            self.next_nodes.push((levels, next_node_idx))
                                        }
                                    }
                                    _ => break,
                                }

                                if !level.is_metadata() {
                                    self.subscribers.extend(node.multi_wildcard.iter());

                                    if !self.subscribers.is_empty() {
                                        break;
                                    }
                                }
                            } else {
                                self.subscribers.extend(node.subscribers.iter());
                                self.subscribers.extend(node.multi_wildcard.iter());

                                break;
                            }
                        }
                    }
                }

                self.subscribers.pop()
            })
            .and_then(|subscriber_idx| if let Ok(inner) = self.inner.lock() {
                Some(Arc::clone(&inner.subscribers[subscriber_idx]))
            } else {
                None
            })
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn test_subscribe() {
        let mut subscription = Subscription::<()>::default();

        assert!(subscription.is_empty().unwrap());

        assert_matches!(
            subscription.subscribe(topic_filter!("sport/tennis/+")).unwrap(),
            Subscribed{
                node_idx: 3,
                subscriber_idx: 0,
                ..
            }
        );
        assert_matches!(
            subscription.subscribe(topic_filter!("sport/tennis/player1/#")).unwrap(),
            Subscribed{
                node_idx: 4,
                subscriber_idx: 1,
                ..
            }
        );
        assert_matches!(
            subscription.subscribe(topic_filter!("sport/+")).unwrap(),
            Subscribed{
                node_idx: 5,
                subscriber_idx: 2,
                ..
            }
        );
        assert_matches!(
            subscription.subscribe(topic_filter!("#")).unwrap(),
            Subscribed{
                node_idx: 0,
                subscriber_idx: 3,
                ..
            }
        );
    }

    #[test]
    fn test_unsubscribe() {
        let mut subscription = Subscription::default();

        assert!(subscription.is_empty().unwrap());

        let filters = vec![
            topic_filter!("sport/tennis/+"),
            topic_filter!("sport/tennis/player1"),
            topic_filter!("sport/tennis/player1/#"),
            topic_filter!("sport/#"),
            topic_filter!("sport/+"),
            topic_filter!("#")
        ];

        let subscribed = filters
            .into_iter()
            .map(|filter| subscription.subscribe(filter).unwrap())
            .collect::<Vec<Subscribed<()>>>();

        assert_eq!(subscription.inner.lock().unwrap().nodes.len(), 6);

        subscribed.into_iter().for_each(|subscribed| {
            subscription.unsubscribe(subscribed).unwrap();
        });

        assert!(subscription.is_empty().unwrap());

        subscription.purge().unwrap();

        assert!(subscription.is_empty().unwrap());
        assert_eq!(subscription.inner.lock().unwrap().nodes.len(), 1);
    }

    #[test]
    fn test_topic_subscribers() {
        let mut subscription = Subscription::default();

        assert!(subscription.is_empty().unwrap());

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

        let filters_count = filters.len();

        let subscribed = filters
            .into_iter()
            .map(|filter| subscription.subscribe(filter).unwrap())
            .collect::<Vec<Subscribed<()>>>();

        assert_eq!(filters_count, subscribed.len());

        assert_eq!(
            subscription
                .topic_subscribers("sport/tennis/player1")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![
                topic_filter!("sport/tennis/player1"),
                topic_filter!("sport/tennis/player1/#"),
                topic_filter!("sport/tennis/+"),
                topic_filter!("sport/#"),
                topic_filter!("#"),
            ]
        );

        assert_eq!(
            subscription
                .topic_subscribers("sport/tennis/player1/ranking")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![
                topic_filter!("sport/tennis/player1/#"),
                topic_filter!("sport/#"),
                topic_filter!("#"),
            ]
        );

        assert_eq!(
            subscription
                .topic_subscribers("sport/tennis/player1/score/wimbledo")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![
                topic_filter!("sport/tennis/player1/#"),
                topic_filter!("sport/#"),
                topic_filter!("#"),
            ]
        );

        assert_eq!(
            subscription
                .topic_subscribers("sport")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![
                topic_filter!("sport/#"),
                topic_filter!("+"),
                topic_filter!("#"),
            ]
        );

        assert_eq!(
            subscription
                .topic_subscribers("sport/")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![
                topic_filter!("sport/+"),
                topic_filter!("sport/#"),
                topic_filter!("+/+"),
                topic_filter!("#"),
            ]
        );
        assert_eq!(
            subscription
                .topic_subscribers("/finance")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![
                topic_filter!("/+"),
                topic_filter!("+/+"),
                topic_filter!("#"),
            ]
        );

        assert_eq!(
            subscription
                .topic_subscribers("$SYS/monitor/Clients")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![topic_filter!("$SYS/monitor/+"), topic_filter!("$SYS/#")]
        );

        assert_eq!(
            subscription
                .topic_subscribers("/monitor/Clients")
                .unwrap()
                .map(|sub| sub.filter.clone())
                .sorted(),
            vec![topic_filter!("+/monitor/Clients"), topic_filter!("#")]
        );
    }
}
