use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use errors::{ErrorKind, Result};
use server::Session;

#[derive(Debug)]
pub enum State<'a> {
    Closed,
    Connected {
        session: Rc<RefCell<Session<'a>>>,
        latest: Cell<Instant>,
    },
}

impl<'a> Default for State<'a> {
    fn default() -> Self {
        State::Closed
    }
}

impl<'a> State<'a> {
    pub fn connected(&self) -> bool {
        if let State::Connected { .. } = *self {
            true
        } else {
            false
        }
    }

    pub fn session(&self) -> Option<Rc<RefCell<Session<'a>>>> {
        match *self {
            State::Connected { ref session, .. } => Some(Rc::clone(session)),
            _ => None,
        }
    }

    pub fn touch(&mut self) -> Result<()> {
        if let State::Connected { ref latest, .. } = *self {
            latest.set(Instant::now());

            Ok(())
        } else {
            bail!(ErrorKind::ConnectionClosed)
        }
    }

    pub fn connect(&mut self, session: Rc<RefCell<Session<'a>>>) {
        *self = State::Connected {
            session,
            latest: Cell::new(Instant::now()),
        }
    }

    pub fn close(&mut self) {
        *self = State::Closed
    }
}
