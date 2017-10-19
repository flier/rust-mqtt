use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use futures::{Future, IntoFuture, future};
use tokio_service::Service;

use core::{ClientId, ConnectReturnCode, LastWill, Packet, Protocol};
use errors::{Error, ErrorKind, Result};
use server::{AuthManager, Session, State};

/// MQTT service
#[derive(Debug)]
pub struct Conn<'a, A> {
    inner: Inner<'a, A>,
}

impl<'a, A> Conn<'a, A> {
    pub fn new(
        sessions: Rc<RefCell<HashMap<String, Rc<RefCell<Session<'a>>>>>>,
        auth_manager: Option<Rc<RefCell<A>>>,
    ) -> Self {
        Conn {
            inner: Inner {
                state: Rc::new(RefCell::new(State::Disconnected)),
                sessions,
                auth_manager,
            },
        }
    }
}

impl<'a, A> Service for Conn<'a, A>
where
    A: AuthManager,
{
    type Request = Packet<'a>;
    type Response = Packet<'a>;

    type Error = Error;

    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        trace!("serve request: {:?}", request);

        match request {
            Packet::Connect {
                protocol,
                clean_session,
                keep_alive,
                last_will,
                client_id,
                username,
                password,
            } => {
                match self.inner.connect(
                    protocol,
                    clean_session,
                    Duration::from_secs(keep_alive as u64),
                    last_will,
                    client_id.into_owned(),
                    username,
                    password,
                ) {
                    Ok((session, new_session)) => {
                        trace!("client connected: {:?}", session);

                        Box::new(future::ok(Packet::ConnectAck {
                            session_present: !clean_session && !new_session,
                            return_code: ConnectReturnCode::ConnectionAccepted,
                        }))
                    }
                    Err(Error(kind, _)) => {
                        trace!("client connect failed, {:?}", kind);

                        Box::new(future::ok(Packet::ConnectAck {
                            session_present: false,
                            return_code: match kind {
                                ErrorKind::ConnectFailed(code) => code,
                                _ => ConnectReturnCode::ServiceUnavailable,
                            },
                        }))
                    }
                }
            }
            Packet::PingRequest => Box::new(
                self.inner
                    .touch()
                    .map(|_| Packet::PingResponse)
                    .into_future(),
            ),
            _ => {
                self.inner.disconnect();

                let err: Error = ErrorKind::InvalidRequest.into();

                Box::new(Err(err).into_future())
            }
        }
    }
}

#[derive(Debug)]
struct Inner<'a, A> {
    state: Rc<RefCell<State>>,
    sessions: Rc<RefCell<HashMap<String, Rc<RefCell<Session<'a>>>>>>,
    auth_manager: Option<Rc<RefCell<A>>>,
}

impl<'a, A> Inner<'a, A>
where
    A: AuthManager,
{
    pub fn connect(
        &self,
        protocol: Protocol,
        clean_session: bool,
        keep_alive: Duration,
        last_will: Option<LastWill>,
        client_id: String,
        username: Option<Cow<str>>,
        password: Option<Cow<[u8]>>,
    ) -> Result<(Rc<RefCell<Session<'a>>>, bool)> {
        if protocol != Protocol::default() {
            bail!(ErrorKind::ConnectFailed(
                ConnectReturnCode::UnacceptableProtocolVersion,
            ))
        }

        if !clean_session {
            if !ClientId::is_valid(&client_id) {
                bail!(ErrorKind::ConnectFailed(
                    ConnectReturnCode::IdentifierRejected,
                ))
            }

            if let Some(session) = self.sessions.borrow().get(&client_id) {
                debug!("resume session with client_id #{}", client_id);

                let mut s = session.borrow_mut();

                s.set_keep_alive(keep_alive);
                s.set_last_will(last_will);

                // TODO resume session

                return Ok((session.clone(), false));
            }
        } else {
            self.sessions.borrow_mut().remove(&client_id);
        }

        if let Some(ref auth_manager) = self.auth_manager {
            if auth_manager
                .borrow_mut()
                .auth(client_id.as_str().into(), username, password)
                .is_err()
            {
                bail!(ErrorKind::ConnectFailed(
                    ConnectReturnCode::BadUserNameOrPassword,
                ))
            }
        }

        let session = Rc::new(RefCell::new(Session::new(
            client_id.clone(),
            keep_alive,
            last_will.map(|last_will| last_will.into_owned()),
        )));

        self.sessions.borrow_mut().insert(
            client_id,
            session.clone(),
        );

        Ok((session, true))
    }

    pub fn disconnect(&self) {
        self.state.borrow_mut().disconnect()
    }

    pub fn touch(&self) -> Result<()> {
        self.state.borrow_mut().touch()
    }
}
