use std::borrow::Cow;
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use failure::AsFail;

use crate::core::{
    ClientId,
    ConnectReturnCode::{BadUserNameOrPassword, IdentifierRejected, UnacceptableProtocolVersion},
    LastWill, Protocol,
};
use crate::errors::{ErrorKind, ErrorKind::ConnectFailed, Result};
use crate::server::{Authenticator, Session, SessionProvider, TopicProvider};

#[derive(Clone, Debug)]
pub enum State<'a, S, T, A> {
    Connecting(Connecting<S, T, A>),
    Connected(Connected<'a>),
    Disconnected,
}

#[derive(Clone, Debug)]
pub struct Connecting<S, T, A> {
    session_provider: Arc<Mutex<S>>,
    topic_provider: T,
    authenticator: Option<Arc<Mutex<A>>>,
}

#[derive(Clone, Debug)]
pub struct Connected<'a> {
    session: Arc<Mutex<Session<'a>>>,
    latest: Cell<Instant>,
    resumed: bool,
}

impl<'a, S, T, A> State<'a, S, T, A> {
    pub fn new(
        session_provider: Arc<Mutex<S>>,
        topic_provider: T,
        authenticator: Option<Arc<Mutex<A>>>,
    ) -> State<'a, S, T, A> {
        State::Connecting(Connecting {
            session_provider,
            topic_provider,
            authenticator,
        })
    }

    pub fn connected(&self) -> Option<Connected<'a>> {
        if let State::Connected(ref connected) = *self {
            Some(connected.clone())
        } else {
            None
        }
    }
}

impl<'a, S, T, A> Default for State<'a, S, T, A> {
    fn default() -> State<'a, S, T, A> {
        State::Disconnected
    }
}

impl<'a, S, T, A> Connecting<S, T, A>
where
    S: SessionProvider<Key = String, Value = Arc<Mutex<Session<'a>>>>,
    T: TopicProvider,
    A: Authenticator,
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
    ) -> Result<Connected<'a>> {
        // If the protocol name is incorrect the Server MAY disconnect the Client,
        // or it MAY continue processing the CONNECT packet in accordance with some other specification.
        // In the latter case, the Server MUST NOT continue to process the CONNECT packet
        // in line with this specification [MQTT-3.1.2-1].
        //
        // The Server MUST respond to the CONNECT Packet with a CONNACK return code 0x01
        // (unacceptable protocol level) and then disconnect the Client if the Protocol Level
        // is not supported by the Server [MQTT-3.1.2-2].
        if protocol != Protocol::default() {
            return Err(ConnectFailed(UnacceptableProtocolVersion).into());
        }

        if !ClientId::is_valid(&client_id) {
            return Err(ConnectFailed(IdentifierRejected).into());
        }

        if !clean_session {
            // If CleanSession is set to 0, the Server MUST resume communications with the Client based on state
            // from the current Session (as identified by the Client identifier).
            // If there is no Session associated with the Client identifier the Server MUST create a new Session.
            // The Client and Server MUST store the Session after the Client and Server are disconnected [MQTT-3.1.2-4]. After the disconnection of a Session that had CleanSession set to 0, the Server MUST store further QoS 1 and QoS 2 messages that match any subscriptions that the client had at the time of disconnection as part of the Session state [MQTT-3.1.2-5].

            if let Some(session) = self
                .session_provider
                .lock()
                .map_err(ErrorKind::from)?
                .get(&client_id)
            {
                debug!("resume session with client_id #{}", client_id);

                // If the ClientId represents a Client already connected to the Server
                // then the Server MUST disconnect the existing Client [MQTT-3.1.4-2].

                // TODO
                {
                    let mut s = session.lock().map_err(ErrorKind::from)?;

                    s.set_keep_alive(keep_alive);
                    s.set_last_will(last_will);
                }

                // TODO resume session

                return Ok(Connected {
                    session: Arc::clone(&session),
                    latest: Cell::new(Instant::now()),
                    resumed: true,
                });
            }
        } else {
            // If CleanSession is set to 1, the Client and Server MUST discard any previous Session and start a new one.
            // This Session lasts as long as the Network Connection.
            // State data associated with this Session MUST NOT be reused in any subsequent Session [MQTT-3.1.2-6].
            self.session_provider
                .lock()
                .map_err(ErrorKind::from)?
                .remove(&client_id);
        }

        if let Some(ref authenticator) = self.authenticator {
            if let Some(username) = username {
                match authenticator.lock().map_err(ErrorKind::from)?.authenticate(
                    client_id.as_str(),
                    username.as_ref(),
                    password
                        .map(|pass| pass.into_owned())
                        .as_ref()
                        .map(|v| v.as_slice()),
                ) {
                    Ok(_) => {
                        info!("user `{}` login as client: {}", username, client_id);
                    }
                    Err(err) => {
                        debug!("user `{}` login failed, {:?}", username, err.as_fail());

                        return Err(ConnectFailed(BadUserNameOrPassword).into());
                    }
                }
            } else {
                debug!("missing username/password from client: {}", client_id);

                return Err(ConnectFailed(BadUserNameOrPassword).into());
            }
        }

        let session = Arc::new(Mutex::new(Session::new(
            client_id.clone(),
            keep_alive,
            last_will.map(|last_will| last_will.into_owned()),
        )));

        self.session_provider
            .lock()
            .map_err(ErrorKind::from)?
            .insert(client_id, Arc::clone(&session));

        Ok(Connected {
            session,
            latest: Cell::new(Instant::now()),
            resumed: false,
        })
    }
}

impl<'a> Connected<'a> {
    pub fn session(&self) -> Arc<Mutex<Session<'a>>> {
        Arc::clone(&self.session)
    }

    pub fn is_resumed(&self) -> bool {
        self.resumed
    }

    pub fn touch(&mut self) {
        self.latest.set(Instant::now())
    }

    pub fn disconnect<S, T, A>(self) -> Result<State<'a, S, T, A>> {
        // MUST discard any Will Message associated with the current connection without publishing it,
        // as described in Section 3.1.2.5 [MQTT-3.14.4-3].

        self.session
            .lock()
            .map_err(ErrorKind::from)?
            .set_last_will(None);

        Ok(State::Disconnected)
    }
}
