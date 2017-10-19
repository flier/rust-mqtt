use std::time::Instant;

use errors::{ErrorKind, Result};

#[derive(Debug)]
pub enum State {
    Disconnected,
    Connected { latest: Instant },
}

impl State {
    pub fn touch(&mut self) -> Result<()> {
        if let State::Connected { .. } = *self {
            *self = State::Connected { latest: Instant::now() };

            Ok(())
        } else {
            bail!(ErrorKind::Disconnected)
        }
    }

    pub fn disconnect(&mut self) {
        *self = State::Disconnected
    }
}
