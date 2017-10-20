use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use core::LastWill;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Session<'a> {
    client_id: String,
    keep_alive: Duration,
    last_will: Option<LastWill<'a>>,
}

impl<'a> Session<'a> {
    pub fn new(client_id: String, keep_alive: Duration, last_will: Option<LastWill<'a>>) -> Self {
        Session {
            client_id,
            keep_alive,
            last_will,
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
}

pub trait SessionManager {
    type Key;
    type Value;

    fn get(&self, key: &Self::Key) -> Option<Self::Value>;

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value>;

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value>;
}

#[derive(Debug, Default)]
pub struct InMemorySessionManager<'a> {
    sessions: HashMap<String, Rc<RefCell<Session<'a>>>>,
}

impl<'a> SessionManager for InMemorySessionManager<'a> {
    type Key = String;
    type Value = Rc<RefCell<Session<'a>>>;

    fn get(&self, key: &Self::Key) -> Option<Self::Value> {
        self.sessions.get(key).map(|v| Rc::clone(v))
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        self.sessions.insert(key, value)
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        self.sessions.remove(key)
    }
}
