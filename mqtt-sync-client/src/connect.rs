use std::net::{TcpStream, ToSocketAddrs};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};

use crate::{
    framed::Framed,
    io::{Receiver, Sender},
    mqtt::{ConnectReturnCode, Property, ReasonCode},
    packet::Packet,
    proto::{Protocol, MQTT_V5},
    Client,
};

pub fn connect<'a, A: ToSocketAddrs>(addr: A) -> Result<Client<'a, TcpStream, MQTT_V5>> {
    Connector::<A>::new(addr).connect()
}

#[derive(Debug)]
pub struct Connector<'a, A, P = MQTT_V5> {
    pub connect: proto::Connect<'a, P>,
    pub addr: A,
}

impl<'a, A, P> Deref for Connector<'a, A, P> {
    type Target = proto::Connect<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.connect
    }
}

impl<'a, A, P> DerefMut for Connector<'a, A, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connect
    }
}

impl<'a, A, P> Connector<'a, A, P>
where
    P: Protocol,
{
    const DEFAULT_KEEPALIVE: Duration = Duration::from_secs(60);

    pub fn new(addr: A) -> Self {
        let connect = proto::connect(Some(Self::DEFAULT_KEEPALIVE), "");

        Connector { addr, connect }
    }
}

impl<'a, A, P> Connector<'a, A, P>
where
    A: ToSocketAddrs,
    P: Protocol,
{
    pub fn connect(self) -> Result<Client<'a, TcpStream, P>> {
        let mut stream = TcpStream::connect(self.addr)?;

        let connect = self.connect;
        stream.send(connect.clone())?;

        let mut framed = Framed::new(stream, P::VERSION);

        let packet = framed.receive()?;

        match packet {
            Packet::ConnectAck(connect_ack) => match connect_ack.return_code {
                ConnectReturnCode::ConnectionAccepted => {
                    let keep_alive = if connect.keep_alive > 0 {
                        Some(Duration::from_secs(connect.keep_alive as u64))
                    } else {
                        None
                    };
                    let session_present = connect_ack.session_present;
                    let properties = connect_ack
                        .properties
                        .map(|props| props.into_iter().collect())
                        .unwrap_or_default();

                    Ok(Client::new(framed, keep_alive, session_present, properties))
                }
                ConnectReturnCode::ServiceUnavailable => {
                    if let Some(addr) = connect_ack.properties.as_ref().and_then(|props| {
                        props.iter().find_map(|prop| {
                            if let Property::ServerReference(server) = prop {
                                Some(server)
                            } else {
                                None
                            }
                        })
                    }) {
                        info!("redirect to {}", addr);

                        Connector { addr, connect }.connect()
                    } else {
                        Err(connect_ack.return_code).context("connect")
                    }
                }
                code => Err(code).context("connect"),
            },
            Packet::Disconnect(disconnect) => {
                if let Some(code) = disconnect.reason_code {
                    match code {
                        ReasonCode::UseAnotherServer | ReasonCode::ServerMoved => {
                            if let Some(addr) = disconnect.properties.as_ref().and_then(|props| {
                                props.iter().find_map(|prop| {
                                    if let Property::ServerReference(server) = prop {
                                        Some(server)
                                    } else {
                                        None
                                    }
                                })
                            }) {
                                info!("redirect to {}", addr);

                                Connector { addr, connect }.connect()
                            } else {
                                Err(code).context("connect rejected")
                            }
                        }
                        _ => Err(code).context("connect rejected"),
                    }
                } else {
                    Err(anyhow!("connect rejected"))
                }
            }
            _ => Err(anyhow!("unexpected response: {:#?}", packet)),
        }
    }
}
