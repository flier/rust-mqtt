use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::ops::{Deref, DerefMut};
use std::process;
use std::sync::Once;
use std::time::Duration;

use crate::{
    packet::Packet,
    proto::{Protocol, MQTT_V5},
    sync::{Client, ReadExt, WriteExt},
};

const MAX_PACKET_SIZE: usize = 4096;

static mut DEFAULT_CLIENT_ID: String = String::new();
static DEFAULT_CLIENT_ID_INIT: Once = Once::new();

pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Client<TcpStream, MQTT_V5>> {
    Connector::<A, MQTT_V5>::new(addr).connect()
}

#[derive(Debug)]
pub struct Connector<'a, A, P> {
    pub addr: A,
    pub connect: proto::Connect<'a, P>,
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
        DEFAULT_CLIENT_ID_INIT.call_once(|| unsafe {
            DEFAULT_CLIENT_ID = format!(
                "rust-mqtt-client-{}@{}",
                hostname::get()
                    .ok()
                    .and_then(|s| s.to_str().map(|s| s.to_string()))
                    .unwrap_or("localhost".to_owned()),
                process::id()
            )
        });

        let connect = proto::Connect::new(Self::DEFAULT_KEEPALIVE, unsafe {
            DEFAULT_CLIENT_ID.as_str()
        });

        Connector { addr, connect }
    }
}

impl<'a, A, P> Connector<'a, A, P>
where
    A: ToSocketAddrs,
    P: Protocol,
{
    pub fn connect(self) -> io::Result<Client<TcpStream, P>> {
        let mut stream = TcpStream::connect(self.addr)?;

        stream.send(Packet::Connect(self.connect.into()))?;

        let mut buf = [0; MAX_PACKET_SIZE];
        let (remaining, packet) = stream.receive(&mut buf, P::VERSION)?;
        if let Packet::ConnectAck(connect_ack) = packet {
            connect_ack
                .return_code
                .ok()
                .map(|_| Client::new(stream, connect_ack.session_present, connect_ack.properties))
                .map_err(|code| io::Error::new(io::ErrorKind::Other, code))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("unexpected response type: {:?}", packet.packet_type()),
            ))
        }
    }
}
