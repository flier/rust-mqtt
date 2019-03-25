use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{Future, IntoFuture};
use tokio_service::Service;

use core::{ConnectReturnCode, Packet, QoS, SubscribeReturnCode};
use errors::{Error, ErrorKind, Result};
use server::{
    Authenticator, Connected, Session, SessionProvider, ShutdownSignal, State, TopicProvider,
};

/// MQTT service
pub struct Conn<'a, S, T, A> {
    state: Cell<State<'a, S, T, A>>,
}

impl<'a, S, T, A> Conn<'a, S, T, A> {
    pub fn new(
        shutdown: ShutdownSignal,
        sessions: Arc<Mutex<S>>,
        topics: T,
        authenticator: Option<Arc<Mutex<A>>>,
    ) -> Conn<'a, S, T, A> {
        Conn {
            state: Cell::new(State::new(sessions, topics, authenticator)),
        }
    }

    pub fn connected(&self) -> Option<Connected<'a>> {
        unsafe { (*self.state.as_ptr()).connected() }
    }
}

impl<'a, S, T, A> Service for Conn<'a, S, T, A>
where
    S: SessionProvider<Key = String, Value = Arc<Mutex<Session<'a>>>>,
    T: TopicProvider,
    A: Authenticator,
{
    type Request = Packet<'a>;
    type Response = Option<Packet<'a>>;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        trace!("serve request: {:?}", request);

        Box::new(self.handle(request).into_future())
    }
}

impl<'a, S, T, A> Conn<'a, S, T, A>
where
    S: SessionProvider<Key = String, Value = Arc<Mutex<Session<'a>>>>,
    T: TopicProvider,
    A: Authenticator,
{
    fn handle<'b>(&self, request: Packet<'a>) -> Result<Option<Packet<'b>>> {
        let (response, next_state) = match self.state.take() {
            State::Connecting(connecting) => {
                if let Packet::Connect {
                    protocol,
                    clean_session,
                    keep_alive,
                    last_will,
                    client_id,
                    username,
                    password,
                } = request
                {
                    match connecting.connect(
                        protocol,
                        clean_session,
                        Duration::from_secs(u64::from(keep_alive)),
                        last_will,
                        client_id.into_owned(),
                        username,
                        password,
                    ) {
                        Ok(connected) => {
                            trace!("client connected: {:?}", connected.session());

                            (
                                Some(Packet::ConnectAck {
                                    session_present: !clean_session && connected.is_resumed(),
                                    return_code: ConnectReturnCode::ConnectionAccepted,
                                }),
                                State::Connected(connected),
                            )
                        }
                        Err(Error(kind, _)) => {
                            trace!("client connect failed, {:?}", kind);

                            (
                                Some(Packet::ConnectAck {
                                    session_present: false,
                                    return_code: match kind {
                                        ErrorKind::ConnectFailed(code) => code,
                                        _ => ConnectReturnCode::ServiceUnavailable,
                                    },
                                }),
                                State::Connecting(connecting),
                            )
                        }
                    }
                } else {
                    bail!(ErrorKind::ProtocolViolation)
                }
            }
            State::Connected(connected) => {
                let mut connected = connected.clone();

                let response = match request {
                    Packet::Publish {
                        dup,
                        retain,
                        qos,
                        topic,
                        packet_id,
                        payload,
                    } => {
                        let session = connected.session();
                        let mut session = session.lock()?;

                        session
                            .message_receiver
                            .on_publish(dup, retain, qos, packet_id, topic, payload)
                            .map(|packet_id| {
                                packet_id.and_then(|packet_id| match qos {
                                    QoS::AtLeastOnce => Some(Packet::PublishAck { packet_id }),
                                    QoS::ExactlyOnce => Some(Packet::PublishReceived { packet_id }),
                                    _ => None,
                                })
                            })?
                    }
                    Packet::PublishAck { packet_id } => {
                        let session = connected.session();
                        let mut session = session.lock()?;

                        session
                            .message_sender
                            .on_publish_ack(packet_id)
                            .map(|_| None)?
                    }
                    Packet::PublishReceived { packet_id } => {
                        let session = connected.session();
                        let mut session = session.lock()?;

                        session
                            .message_sender
                            .on_publish_received(packet_id)
                            .map(|packet_id| Some(Packet::PublishRelease { packet_id }))?
                    }
                    Packet::PublishComplete { packet_id } => {
                        let session = connected.session();
                        let mut session = session.lock()?;

                        session
                            .message_sender
                            .on_publish_complete(packet_id)
                            .map(|_| None)?
                    }
                    Packet::PublishRelease { packet_id } => {
                        let session = connected.session();
                        let mut session = session.lock()?;

                        session
                            .message_receiver
                            .on_publish_release(packet_id)
                            .map(|_| None)?
                    }
                    Packet::Subscribe {
                        packet_id,
                        topic_filters,
                    } => {
                        trace!("subscribe filters: {:?}", topic_filters);

                        let session = connected.session();

                        Some(Packet::SubscribeAck {
                            packet_id,
                            status: topic_filters
                                .iter()
                                .map(|&(ref filter, qos)| {
                                    let session = Arc::clone(&session);
                                    let lock = session.try_lock().map_err(|err| err.into());

                                    match lock
                                        .and_then(move |mut session| session.subscribe(filter, qos))
                                    {
                                        Ok(_) => {
                                            trace!(
                                                "subscribed filter {} with QoS {:?}",
                                                filter,
                                                qos
                                            );

                                            SubscribeReturnCode::Success(qos)
                                        }
                                        Err(err) => {
                                            warn!("subscribe topic {} failed, {}", filter, err);

                                            SubscribeReturnCode::Failure
                                        }
                                    }
                                })
                                .collect(),
                        })
                    }
                    Packet::Unsubscribe {
                        packet_id,
                        topic_filters,
                    } => {
                        trace!("unsubscribe filters: {:?}", topic_filters);

                        let session = connected.session();
                        let mut session = session.lock()?;

                        for filter in topic_filters {
                            session.unsubscribe(&filter);
                        }

                        Some(Packet::UnsubscribeAck { packet_id })
                    }
                    Packet::PingRequest => {
                        trace!("ping");

                        connected.touch();

                        Some(Packet::PingResponse)
                    }
                    Packet::Disconnect => {
                        trace!("disconnect");

                        let session = connected.session();
                        let mut session = session.lock()?;

                        session.set_last_will(None);

                        bail!(ErrorKind::ConnectionClosed);
                    }
                    _ => bail!(ErrorKind::ProtocolViolation),
                };

                (response, State::Connected(connected))
            }
            State::Disconnected => bail!(ErrorKind::ProtocolViolation),
        };

        self.state.set(next_state);

        Ok(response)
    }
}

#[cfg(test)]
pub mod tests {
    use std::borrow::Cow;

    use futures::Async;
    use tokio_service::Service;

    use super::*;
    use core::{LastWill, Protocol};
    use server::{shutdown, InMemorySessionProvider, InMemoryTopicProvider, MockAuthenticator};

    fn new_test_conn<'a>(
    ) -> Conn<'a, InMemorySessionProvider<'a>, InMemoryTopicProvider<'a>, MockAuthenticator> {
        new_test_conn_with_provider(
            Arc::new(Mutex::new(InMemorySessionProvider::default())),
            InMemoryTopicProvider::default(),
        )
    }

    fn new_test_conn_with_provider<'a, S, T>(
        session_provider: Arc<Mutex<S>>,
        topic_provider: T,
    ) -> Conn<'a, S, T, MockAuthenticator> {
        let (shutdown_signal, _) = shutdown::signal();

        Conn::new(shutdown_signal, session_provider, topic_provider, None)
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
    fn test_request_before_connect() {
        // After a Network Connection is established by a Client to a Server,
        // the first Packet sent from the Client to the Server MUST be a CONNECT Packet [MQTT-3.1.0-1].
        assert_matches!(
            new_test_conn().call(Packet::PingRequest).poll(),
            Err(Error(ErrorKind::ProtocolViolation, _))
        );
    }

    #[test]
    fn test_connect_with_unacceptable_protocol_version() {
        // The Server MUST respond to the CONNECT Packet with a CONNACK return code 0x01 (unacceptable protocol level)
        // and then disconnect the Client if the Protocol Level is not supported by the Server [MQTT-3.1.2-2].
        assert_matches!(
            new_test_conn()
                .call(Packet::Connect {
                    protocol: Protocol::MQTT(123),
                    clean_session: false,
                    keep_alive: 0,
                    last_will: None,
                    client_id: Cow::from("client_id"),
                    username: None,
                    password: None,
                })
                .poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::UnacceptableProtocolVersion
            })))
        );
    }

    #[test]
    fn test_missing_client_id() {
        /// The Client Identifier (ClientId) MUST be present and MUST be the first field in the CONNECT packet payload [MQTT-3.1.3-3].
        assert_matches!(
            new_test_conn()
                .call(Packet::Connect {
                    protocol: Protocol::default(),
                    clean_session: false,
                    keep_alive: 0,
                    last_will: None,
                    client_id: Cow::from(""),
                    username: None,
                    password: None,
                })
                .poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::IdentifierRejected
            })))
        );
    }

    #[test]
    fn test_invalid_client_id() {
        // The Server MUST allow ClientIds which are between 1 and 23 UTF-8 encoded bytes in length,
        // and that contain only the characters
        // "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ" [MQTT-3.1.3-5].
        assert_matches!(
            new_test_conn()
                .call(Packet::Connect {
                    protocol: Protocol::default(),
                    clean_session: false,
                    keep_alive: 0,
                    last_will: None,
                    client_id: Cow::from("client_id"),
                    username: None,
                    password: None,
                })
                .poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::IdentifierRejected
            })))
        );
    }

    #[test]
    fn test_connect_with_last_will() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        let connected = conn.connected();

        assert!(connected.is_some());

        let connected = connected.unwrap();
        let session = connected.session();

        assert!(session.lock().unwrap().last_will().is_some());
    }

    #[test]
    fn test_disconnect_should_clear_last_will() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        let connected = conn.connected().unwrap();
        let session = connected.session();

        // On receipt of DISCONNECT the Server:
        //      MUST discard any Will Message associated with the current connection without publishing it,
        //       as described in Section 3.1.2.5 [MQTT-3.14.4-3].
        //      SHOULD close the Network Connection if the Client has not already done so.
        assert_matches!(
            conn.call(Packet::Disconnect).poll(),
            Err(Error(ErrorKind::ConnectionClosed, _))
        );

        assert!(conn.connected().is_none());
        assert!(session.lock().unwrap().last_will().is_none());
    }

    #[test]
    fn test_resume_session() {
        let session_provider = Arc::new(Mutex::new(InMemorySessionProvider::default()));
        let topic_provider = InMemoryTopicProvider::default();
        let conn =
            new_test_conn_with_provider(Arc::clone(&session_provider), topic_provider.clone());

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        // On receipt of DISCONNECT the Server:
        //      MUST discard any Will Message associated with the current connection without publishing it,
        //       as described in Section 3.1.2.5 [MQTT-3.14.4-3].
        //      SHOULD close the Network Connection if the Client has not already done so.
        assert_matches!(
            conn.call(Packet::Disconnect).poll(),
            Err(Error(ErrorKind::ConnectionClosed, _))
        );

        let conn =
            new_test_conn_with_provider(Arc::clone(&session_provider), topic_provider.clone());

        // If the Server accepts a connection with CleanSession set to 0,
        // the value set in Session Present depends on whether the Server already has stored Session state
        // for the supplied client ID. If the Server has stored Session state,
        // it MUST set Session Present to 1 in the CONNACK packet [MQTT-3.2.2-2].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: true,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        assert!(conn.connected().is_some());
    }

    #[test]
    fn test_clear_session() {
        let session_provider = Arc::new(Mutex::new(InMemorySessionProvider::default()));
        let topic_provider = InMemoryTopicProvider::default();
        let conn =
            new_test_conn_with_provider(Arc::clone(&session_provider), topic_provider.clone());

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        let connected = conn.connected().unwrap();
        let session = connected.session();

        // On receipt of DISCONNECT the Server:
        //      MUST discard any Will Message associated with the current connection without publishing it,
        //       as described in Section 3.1.2.5 [MQTT-3.14.4-3].
        //      SHOULD close the Network Connection if the Client has not already done so.
        assert_matches!(
            conn.call(Packet::Disconnect).poll(),
            Err(Error(ErrorKind::ConnectionClosed, _))
        );

        let conn = new_test_conn_with_provider(session_provider, topic_provider.clone());

        assert_matches!(
            conn.call(Packet::Connect {
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
            })
            .poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        assert!(conn.connected().is_some());

        let connected = conn.connected().unwrap();
        let new_session = connected.session();

        assert_ne!(
            session.lock().unwrap().last_will(),
            new_session.lock().unwrap().last_will()
        );
        assert_eq!(
            new_session.lock().unwrap().last_will().unwrap(),
            &LastWill {
                qos: QoS::AtLeastOnce,
                retain: false,
                topic: Cow::from("another_topic"),
                message: Cow::from(&b"another_messages"[..]),
            }
        );
    }

    #[test]
    fn test_duplicate_connect_request() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        // A Client can only send the CONNECT Packet once over a Network Connection.
        // The Server MUST process a second CONNECT Packet sent from a Client as a protocol violation
        // and disconnect the Client [MQTT-3.1.0-2].
        assert_matches!(
            conn.call(Packet::Connect {
                protocol: Protocol::default(),
                clean_session: false,
                keep_alive: 0,
                last_will: None,
                client_id: Cow::from("client"),
                username: None,
                password: None,
            })
            .poll(),
            Err(Error(ErrorKind::ProtocolViolation, _))
        );

        assert!(conn.connected().is_none());
    }

    #[test]
    fn test_ping_request() {
        let conn = new_test_conn();

        // The Server MUST acknowledge the CONNECT Packet with a CONNACK Packet containing a zero return code [MQTT-3.1.4-4].
        assert_matches!(
            conn.call(CONNECT_REQUEST.clone()).poll(),
            Ok(Async::Ready(Some(Packet::ConnectAck {
                session_present: false,
                return_code: ConnectReturnCode::ConnectionAccepted
            })))
        );

        // The Server MUST send a PINGRESP Packet in response to a PINGREQ Packet [MQTT-3.12.4-1].
        assert_matches!(
            conn.call(Packet::PingRequest).poll(),
            Ok(Async::Ready(Some(Packet::PingResponse)))
        );
    }
}
