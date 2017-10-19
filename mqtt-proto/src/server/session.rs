use std::time::Duration;

use core::LastWill;

#[derive(Debug)]
pub struct Session<'a> {
    client_id: String,
    keep_alive: Duration,
    last_will: Option<LastWill<'a>>,
}

impl<'a> Session<'a> {
    pub fn new(
        client_id: String,
        keep_alive: Duration,
        last_will: Option<LastWill<'a>>,
    ) -> Session<'a> {
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
