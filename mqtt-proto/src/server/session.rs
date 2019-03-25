use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::core::{LastWill, QoS};
use crate::errors::Result;
use crate::message::{MessageReceiver, MessageSender};

#[derive(Debug)]
pub struct Session<'a> {
    client_id: String,
    keep_alive: Duration,
    last_will: Option<LastWill<'a>>,
    subscription: HashMap<String, QoS>,
    pub message_sender: MessageSender<'a>,
    pub message_receiver: MessageReceiver<'a>,
}

impl<'a> Session<'a> {
    pub fn new(client_id: String, keep_alive: Duration, last_will: Option<LastWill<'a>>) -> Self {
        Session {
            client_id,
            keep_alive,
            last_will,
            subscription: Default::default(),
            message_sender: Default::default(),
            message_receiver: Default::default(),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    pub fn set_keep_alive(&mut self, keep_alive: Duration) {
        self.keep_alive = keep_alive
    }

    pub fn last_will(&self) -> Option<&LastWill<'a>> {
        self.last_will.as_ref()
    }

    pub fn set_last_will(&mut self, last_will: Option<LastWill>) {
        self.last_will = last_will.map(|last_will| last_will.into_owned())
    }

    pub fn subscribe(&mut self, filter: &str, qos: QoS) -> Result<Option<QoS>> {
        Ok(self.subscription.insert(filter.to_owned(), qos))
    }

    pub fn unsubscribe(&mut self, filter: &str) -> Option<QoS> {
        self.subscription.remove(filter)
    }
}

pub trait SessionProvider: Clone {
    type Key;
    type Value;

    fn get(&self, key: &Self::Key) -> Option<Self::Value>;

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value>;

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemorySessionProvider<'a> {
    sessions: HashMap<String, Arc<Mutex<Session<'a>>>>,
}

impl<'a> SessionProvider for InMemorySessionProvider<'a> {
    type Key = String;
    type Value = Arc<Mutex<Session<'a>>>;

    fn get(&self, key: &Self::Key) -> Option<Self::Value> {
        self.sessions.get(key).cloned()
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        self.sessions.insert(key, value)
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        self.sessions.remove(key)
    }
}
