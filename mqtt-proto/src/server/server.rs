use std::io;
use std::sync::{Arc, Mutex};

use tokio_core::reactor::{Handle, Remote};
use tokio_service::{NewService, Service};

use server::{Authenticator, Conn, MockAuthenticator, Session, SessionProvider, TopicProvider,
             shutdown};

pub struct Server<S, T, A> {
    remote: Remote,
    session_provider: Arc<Mutex<S>>,
    topic_provider: T,
    authenticator: Option<Arc<Mutex<A>>>,
}

impl<S, T> Server<S, T, MockAuthenticator> {
    pub fn new(handle: &Handle, session_provider: Arc<Mutex<S>>, topic_provider: T) -> Self {
        Self::with_authenticator(handle, session_provider, topic_provider, None)
    }
}

impl<S, T, A> Server<S, T, A>
where
    A: Authenticator,
{
    pub fn with_authenticator(
        handle: &Handle,
        session_provider: Arc<Mutex<S>>,
        topic_provider: T,
        authenticator: Option<Arc<Mutex<A>>>,
    ) -> Self {
        Server {
            remote: handle.remote().clone(),
            session_provider,
            topic_provider,
            authenticator: authenticator,
        }
    }
}

impl<'a, S, T, A> NewService for Server<S, T, A>
where
    S: SessionProvider<
        Key = String,
        Value = Arc<Mutex<Session<'a>>>,
    >,
    T: TopicProvider,
    A: Authenticator,
{
    type Request = <Self::Instance as Service>::Request;
    type Response = <Self::Instance as Service>::Response;
    type Error = <Self::Instance as Service>::Error;
    type Instance = Conn<'a, S, T, A>;

    /// Create and return a new service value.
    fn new_service(&self) -> io::Result<Self::Instance> {
        let (shutdown_signal, shutdown_future) = shutdown::signal();

        Ok(Conn::new(
            shutdown_signal,
            Arc::clone(&self.session_provider),
            self.topic_provider.clone(),
            self.authenticator.clone(),
        ))
    }
}
