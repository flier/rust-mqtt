use std::io;
use std::sync::{Arc, Mutex};

use tokio_core::reactor::{Handle, Remote};
use tokio_service::NewService;

use core::Packet;
use errors::Error;
use server::{Authenticator, Conn, Session, SessionProvider};

pub struct Server<S, A> {
    remote: Remote,
    sessions: Arc<Mutex<S>>,
    authenticator: Option<Arc<Mutex<A>>>,
}

impl<S, A> Server<S, A> {
    pub fn new(
        handle: &Handle,
        sessions: Arc<Mutex<S>>,
        authenticator: Option<Arc<Mutex<A>>>,
    ) -> Self {
        Server {
            remote: handle.remote().clone(),
            sessions,
            authenticator,
        }
    }
}

impl<'a, S, A> NewService for Server<S, A>
where
    S: SessionProvider<
        Key = String,
        Value = Arc<Mutex<Session<'a>>>,
    >,
    A: Authenticator,
{
    type Request = Packet<'a>;
    type Response = Option<Packet<'a>>;
    type Error = Error;
    type Instance = Conn<'a, S, A>;

    /// Create and return a new service value.
    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(Conn::new(self.sessions.clone(), self.authenticator.clone()))
    }
}
