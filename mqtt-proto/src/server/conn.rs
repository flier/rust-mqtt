use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use futures::{Future, IntoFuture, future};
use tokio_service::Service;

use core::{ClientId, ConnectReturnCode, LastWill, Packet, Protocol, QoS};
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
                state: Default::default(),
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
                if self.inner.connected() {
                    let err: Error = ErrorKind::ProtocolViolation.into();

                    self.inner.shutdown();

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
            Packet::PingRequest if self.inner.connected() => {
                Box::new(
                    self.inner
                        .touch()
                        .map(|_| Packet::PingResponse)
                        .into_future(),
                )
            }
            Packet::Disconnect if self.inner.connected() => {
                self.inner.disconnect();

                let err: Error = ErrorKind::ConnectionClosed.into();

                Box::new(Err(err).into_future())
            }
            _ => {
                // After a Network Connection is established by a Client to a Server,
                // the first Packet sent from the Client to the Server MUST be a CONNECT Packet [MQTT-3.1.0-1].

                self.inner.shutdown();

                let err: Error = ErrorKind::ConnectionClosed.into();

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

        if !ClientId::is_valid(&client_id) {
            bail!(ErrorKind::ConnectFailed(
                ConnectReturnCode::IdentifierRejected,
            ))
        }

        if !clean_session {
            // If CleanSession is set to 0, the Server MUST resume communications with the Client based on state
            // from the current Session (as identified by the Client identifier).
            // If there is no Session associated with the Client identifier the Server MUST create a new Session.
            // The Client and Server MUST store the Session after the Client and Server are disconnected [MQTT-3.1.2-4]. After the disconnection of a Session that had CleanSession set to 0, the Server MUST store further QoS 1 and QoS 2 messages that match any subscriptions that the client had at the time of disconnection as part of the Session state [MQTT-3.1.2-5].

            if let Some(session) = self.sessions.borrow().get(&client_id) {
                debug!("resume session with client_id #{}", client_id);

                // If the ClientId represents a Client already connected to the Server
                // then the Server MUST disconnect the existing Client [MQTT-3.1.4-2].

                // TODO

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
}

impl<'a, A> Inner<'a, A> {
    pub fn connected(&self) -> bool {
        self.state.borrow().connected()
    }

    pub fn shutdown(&self) {
        self.state.borrow_mut().close()
    }

    pub fn disconnect(&self) {
        // MUST discard any Will Message associated with the current connection without publishing it,
        // as described in Section 3.1.2.5 [MQTT-3.14.4-3].

        if let Some(session) = self.state.borrow().session() {
            session.borrow_mut().set_last_will(None)
        }

        self.state.borrow_mut().close()
    }

    pub fn touch(&self) -> Result<()> {
        self.state.borrow_mut().touch()
    }
}

#[cfg(test)]
pub mod tests {
    use futures::Async;
    use tokio_service::Service;

    use super::*;

    impl AuthManager for () {
        type Error = Error;

        fn auth<'a>(
            &mut self,
            _client_id: Cow<'a, str>,
            _username: Option<Cow<'a, str>>,
            _password: Option<Cow<'a, [u8]>>,
        ) -> Result<()> {
            Ok(())
        }
    }

    impl AuthManager for (String, Vec<u8>) {
        type Error = Error;

        fn auth<'a>(
            &mut self,
            _client_id: Cow<'a, str>,
            username: Option<Cow<'a, str>>,
            password: Option<Cow<'a, [u8]>>,
        ) -> Result<()> {
            if username == Some(self.0.clone().into()) && password == Some(self.1.clone().into()) {
                Ok(())
            } else {
                bail!(ErrorKind::ConnectFailed(
                    ConnectReturnCode::BadUserNameOrPassword,
                ))
            }
        }
    }

    fn new_test_conn<'a>() -> Conn<'a, ()> {
        Conn::new(Rc::new(RefCell::new(HashMap::new())), None)
    }

    fn new_test_conn_with_auth<'a>(username: &str, password: &[u8]) -> Conn<'a, (String, Vec<u8>)> {
        Conn::new(
            Rc::new(RefCell::new(HashMap::new())),
            Some(Rc::new(
                RefCell::new((username.to_owned(), password.to_owned())),
            )),
        )
    }

    lazy_static! {
        static ref CONNECT_REQUEST: Packet<'static> = Packet::Connect {
            protocol: Protocol::default(),
            clean_session: false,
            keep_alive: 0,
            last_will: Some(LastWill {
                qos: QoS::ExactlyOnce,
                retain: false,
                topic: Cow::from("topic"),
                message: Cow::from(&b"messages"[..]),
            }),
            client_id: Cow::from("client"),
            username: None,
            password: None,
        };
    }

    #[test]
    fn request_before_connect() {
        // After a Network Connection is established by a Client to a Server,
        // the first Packet sent from the Client to the Server MUST be a CONNECT Packet [MQTT-3.1.0-1].
        assert_matches!(new_test_conn().call(Packet::PingRequest).poll(), Err(Error(ErrorKind::ConnectionClosed, _)));
    }

    #[test]
    fn connect_with_unacceptable_protocol_version() {
        // The Server MUST respond to the CONNECT Packet with a CONNACK return code 0x01 (unacceptable protocol level)
        // and then disconnect the Client if the Protocol Level is not supported by the Server [MQTT-3.1.2-2].
        assert_matches!(new_test_conn().call(Packet::Connect {
            protocol: Protocol::MQTT(123),
            clean_session: false,
            keep_alive: 0,
            last_will: None,
            client_id: Cow::from("client_id"),
            username: None,
            password: None,
        }).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::UnacceptableProtocolVersion
        })));
    }

    #[test]
    fn missing_client_id() {
        /// The Client Identifier (ClientId) MUST be present and MUST be the first field in the CONNECT packet payload [MQTT-3.1.3-3].
        assert_matches!(new_test_conn().call(Packet::Connect {
            protocol: Protocol::default(),
            clean_session: false,
            keep_alive: 0,
            last_will: None,
            client_id: Cow::from(""),
            username: None,
            password: None,
        }).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::IdentifierRejected
        })));
    }

    #[test]
    fn invalid_client_id() {
        // The Server MUST allow ClientIds which are between 1 and 23 UTF-8 encoded bytes in length,
        // and that contain only the characters
        // "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ" [MQTT-3.1.3-5].
        assert_matches!(new_test_conn().call(Packet::Connect {
            protocol: Protocol::default(),
            clean_session: false,
            keep_alive: 0,
            last_will: None,
            client_id: Cow::from("client_id"),
            username: None,
            password: None,
        }).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::IdentifierRejected
        })));
    }

    #[test]
    fn connect_with_last_will() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        assert!(conn.inner.connected());

        let session = conn.inner.state.borrow().session().unwrap();

        assert!(session.borrow().last_will().is_some());
    }

    #[test]
    fn disconnect_should_clear_last_will() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        let session = conn.inner.state.borrow().session().unwrap();

        // On receipt of DISCONNECT the Server:
        //      MUST discard any Will Message associated with the current connection without publishing it,
        //       as described in Section 3.1.2.5 [MQTT-3.14.4-3].
        //      SHOULD close the Network Connection if the Client has not already done so.
        assert_matches!(conn.call(Packet::Disconnect).poll(), Err(Error(ErrorKind::ConnectionClosed, _)));

        assert!(!conn.inner.connected());
        assert!(session.borrow().last_will().is_none());
    }

    #[test]
    fn resume_session() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        // On receipt of DISCONNECT the Server:
        //      MUST discard any Will Message associated with the current connection without publishing it,
        //       as described in Section 3.1.2.5 [MQTT-3.14.4-3].
        //      SHOULD close the Network Connection if the Client has not already done so.
        assert_matches!(conn.call(Packet::Disconnect).poll(), Err(Error(ErrorKind::ConnectionClosed, _)));

        // If the Server accepts a connection with CleanSession set to 0,
        // the value set in Session Present depends on whether the Server already has stored Session state
        // for the supplied client ID. If the Server has stored Session state,
        // it MUST set Session Present to 1 in the CONNACK packet [MQTT-3.2.2-2].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: true,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        assert!(conn.inner.connected());
    }

    #[test]
    fn clear_session() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        let session = conn.inner.state.borrow().session().unwrap();

        // On receipt of DISCONNECT the Server:
        //      MUST discard any Will Message associated with the current connection without publishing it,
        //       as described in Section 3.1.2.5 [MQTT-3.14.4-3].
        //      SHOULD close the Network Connection if the Client has not already done so.
        assert_matches!(conn.call(Packet::Disconnect).poll(), Err(Error(ErrorKind::ConnectionClosed, _)));

        assert_matches!(conn.call(Packet::Connect {
            protocol: Protocol::default(),
            clean_session: true,
            keep_alive: 0,
            last_will: Some(LastWill {
                qos: QoS::AtLeastOnce,
                retain: false,
                topic: Cow::from("another_topic"),
                message: Cow::from(&b"another_messages"[..]),
            }),
            client_id: Cow::from("client"),
            username: None,
            password: None,
        }).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        assert!(conn.inner.connected());

        let new_session = conn.inner.state.borrow().session().unwrap();

        assert_ne!(*session.borrow(), *new_session.borrow());
        assert_eq!(new_session.borrow().last_will().unwrap(), &LastWill {
            qos: QoS::AtLeastOnce,
            retain: false,
            topic: Cow::from("another_topic"),
            message: Cow::from(&b"another_messages"[..]),
        });
    }

    #[test]
    fn duplicate_connect_request() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        // A Client can only send the CONNECT Packet once over a Network Connection.
        // The Server MUST process a second CONNECT Packet sent from a Client as a protocol violation
        // and disconnect the Client [MQTT-3.1.0-2].
        assert_matches!(conn.call(Packet::Connect {
            protocol: Protocol::default(),
            clean_session: false,
            keep_alive: 0,
            last_will: None,
            client_id: Cow::from("client"),
            username: None,
            password: None,
        }).poll(), Err(Error(ErrorKind::ProtocolViolation, _)));

        assert!(!conn.inner.connected());
    }

    #[test]
    fn ping_request() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(conn.call(CONNECT_REQUEST.clone()).poll(), Ok(Async::Ready(Packet::ConnectAck {
            session_present: false,
            return_code: ConnectReturnCode::ConnectionAccepted
        })));

        // The Server MUST send a PINGRESP Packet in response to a PINGREQ Packet [MQTT-3.12.4-1].
        assert_matches!(conn.call(Packet::PingRequest).poll(), Ok(Async::Ready(Packet::PingResponse)));
    }
}
