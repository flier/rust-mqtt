use std::result::Result as StdResult;
use std::sync::Arc;

use futures::{Sink, Stream};

use crate::errors::{Error, Result};
use crate::message::Message;
use crate::server::{Subscribed, Subscriber, Subscription, TopicSubscribers};
use crate::topic::Filter;

pub trait TopicProvider: Clone {
    type Message;
    type Error;
    type Subscribed: Stream<Item = Self::Message>;
    type Subscriber: Sink<SinkItem = Self::Message>;
    type TopicSubscribers: Iterator<Item = Arc<Self::Subscriber>>;

    fn subscribe(&mut self, filter: Filter) -> StdResult<Self::Subscribed, Self::Error>;

    fn unsubscribe(
        &mut self,
        subscribed: Subscribed<Self::Message>,
    ) -> StdResult<Arc<Self::Subscriber>, Self::Error>;

    fn topic_subscribers<S: AsRef<str>>(
        &self,
        topic: S,
    ) -> StdResult<Self::TopicSubscribers, Self::Error>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryTopicProvider<'a> {
    subscription: Subscription<Arc<Message<'a>>>,
}

impl<'a> TopicProvider for InMemoryTopicProvider<'a> {
    type Message = Arc<Message<'a>>;
    type Error = Error;
    type Subscribed = Subscribed<Self::Message>;
    type Subscriber = Subscriber<Self::Message>;
    type TopicSubscribers = TopicSubscribers<Self::Message>;

    fn subscribe(&mut self, filter: Filter) -> Result<Self::Subscribed> {
        self.subscription.subscribe(filter)
    }

    fn unsubscribe(
        &mut self,
        subscribed: Subscribed<Self::Message>,
    ) -> Result<Arc<Self::Subscriber>> {
        self.subscription.unsubscribe(subscribed)
    }

    fn topic_subscribers<S: AsRef<str>>(&self, topic: S) -> Result<Self::TopicSubscribers> {
        self.subscription.topic_subscribers(topic)
    }
}
