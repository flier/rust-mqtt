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
                // The Server MUST process a second CONNECT Packet sent from a Client
                // as a protocol violation and disconnect the Client [MQTT-3.1.0-2].
                if self.inner.state.borrow().connected() {
                    let err: Error = ErrorKind::ProtocolViolation.into();

                    return Box::new(Err(err).into_future());
                }

                match self.inner.connect(
                    protocol,
                    clean_session,
                    Duration::from_secs(u64::from(keep_alive)),
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
            Packet::PingRequest if self.inner.state.borrow().connected() => {
                Box::new(
                    self.inner
                        .touch()
                        .map(|_| Packet::PingResponse)
                        .into_future(),
                )
            }
            _ => {
                // After a Network Connection is established by a Client to a Server,
                // the first Packet sent from the Client to the Server MUST be a CONNECT Packet [MQTT-3.1.0-1].

                self.inner.disconnect();

                let err: Error = ErrorKind::InvalidRequest.into();

                Box::new(Err(err).into_future())
            }
        }
    }
}

#[derive(Debug)]
struct Inner<'a, A> {
    state: Rc<RefCell<State<'a>>>,
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
        // If the protocol name is incorrect the Server MAY disconnect the Client,
        // or it MAY continue processing the CONNECT packet in accordance with some other specification.
        // In the latter case, the Server MUST NOT continue to process the CONNECT packet
        // in line with this specification [MQTT-3.1.2-1].
        //
        // The Server MUST respond to the CONNECT Packet with a CONNACK return code 0x01
        // (unacceptable protocol level) and then disconnect the Client if the Protocol Level
        // is not supported by the Server [MQTT-3.1.2-2].
        if protocol != Protocol::default() {
            bail!(ErrorKind::ConnectFailed(
                ConnectReturnCode::UnacceptableProtocolVersion,
            ))
        }

        if !clean_session {
            // If CleanSession is set to 0, the Server MUST resume communications with the Client based on state
            // from the current Session (as identified by the Client identifier).
            // If there is no Session associated with the Client identifier the Server MUST create a new Session.
            // The Client and Server MUST store the Session after the Client and Server are disconnected [MQTT-3.1.2-4]. After the disconnection of a Session that had CleanSession set to 0, the Server MUST store further QoS 1 and QoS 2 messages that match any subscriptions that the client had at the time of disconnection as part of the Session state [MQTT-3.1.2-5].

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

                self.state.borrow_mut().connect(Rc::clone(&session));

                return Ok((Rc::clone(session), false));
            }
        } else {
            // If CleanSession is set to 1, the Client and Server MUST discard any previous Session and start a new one.
            // This Session lasts as long as the Network Connection.
            // State data associated with this Session MUST NOT be reused in any subsequent Session [MQTT-3.1.2-6].
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
            Rc::clone(&session),
        );

        self.state.borrow_mut().connect(Rc::clone(&session));

        Ok((session, true))
    }

    pub fn disconnect(&self) {
        self.state.borrow_mut().disconnect()
    }

    pub fn touch(&self) -> Result<()> {
        self.state.borrow_mut().touch()
    }
}
