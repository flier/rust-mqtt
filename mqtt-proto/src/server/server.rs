use std::io;
use std::sync::{Arc, Mutex};

use tokio_core::reactor::{Handle, Remote};
use tokio_service::{NewService, Service};

use server::{Authenticator, Conn, MockAuthenticator, Session, SessionProvider};

pub struct Server<S, A> {
    remote: Remote,
    sessions: Arc<Mutex<S>>,
    authenticator: Option<Arc<Mutex<A>>>,
}

impl<S> Server<S, MockAuthenticator> {
    pub fn new(handle: &Handle, sessions: Arc<Mutex<S>>) -> Self {
        Server {
            remote: handle.remote().clone(),
            sessions,
            authenticator: None,
        }
    }
}

impl<S, A> Server<S, A>
where
    A: Authenticator,
{
    pub fn with_authenticator(
        handle: &Handle,
        sessions: Arc<Mutex<S>>,
        authenticator: Option<Arc<Mutex<A>>>,
    ) -> Self {
        Server {
            remote: handle.remote().clone(),
            sessions,
            authenticator: authenticator,
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
    type Request = <Self::Instance as Service>::Request;
    type Response = <Self::Instance as Service>::Response;
    type Error = <Self::Instance as Service>::Error;
    type Instance = Conn<'a, S, A>;

    /// Create and return a new service value.
    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(Conn::new(
            Arc::clone(&self.sessions),
            self.authenticator.clone(),
        ))
    }
}
