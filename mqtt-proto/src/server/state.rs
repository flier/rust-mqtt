use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use errors::{ErrorKind, Result};
use server::Session;

#[derive(Debug)]
pub enum State<'a> {
    Disconnected,
    Connected {
        session: Rc<RefCell<Session<'a>>>,
        latest: Cell<Instant>,
    },
}

impl<'a> State<'a> {
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
            bail!(ErrorKind::Disconnected)
        }
    }

    pub fn connected(&mut self, session: Rc<RefCell<Session<'a>>>) {
        *self = State::Connected {
            session,
            latest: Cell::new(Instant::now()),
        }
    }

    pub fn disconnected(&mut self) {
        *self = State::Disconnected
    }
}
