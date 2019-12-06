#[macro_use]
extern crate log;

use core::marker::PhantomData;

use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::process;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use hexplay::HexViewBuilder;
use structopt::StructOpt;
use url::Url;

use mqtt_packet::*;
use mqtt_proto::*;

const MAX_PACKET_SIZE: usize = 4096;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "sub_client",
    about = "an MQTT version 5/3.1.1 client for subscribing to topics"
)]
struct Opt {
    /// Specify the host to connect to.
    #[structopt(short, long, default_value = "localhost")]
    host: String,

    /// Connect to the port specified.
    #[structopt(short, long, default_value = "1883")]
    port: u16,

    /// Specify specify user, password, hostname, port and topic at once as a URL.
    /// The URL must be in the form: mqtt(s)://[username[:password]@]host[:port]/topic
    ///
    /// If the scheme is mqtt:// then the port defaults to 1883. If the scheme is mqtts:// then the port defaults to 8883.
    #[structopt(short = "L", long)]
    url: Option<Url>,

    /// Specify which version of the MQTT protocol should be used when connecting to the remote broker.
    #[structopt(short = "V", long, default_value = "311", parse(try_from_str = parse_protocol_version))]
    protocol_version: ProtocolVersion,

    /// The id to use for this client.
    #[structopt(short, long)]
    id: Option<String>,

    /// Provide a prefix that the client id will be built from by appending the process id of the client.
    #[structopt(short = "I", long, default_value = "sub_client")]
    id_prefix: String,

    /// The number of seconds between sending PING commands to the broker
    /// for the purposes of informing it we are still connected and functioning.
    #[structopt(short, long, default_value = "60")]
    keep_alive: u16,

    /// The topic on which to send a Will, in the event that the client disconnects unexpectedly.
    #[structopt(long)]
    will_topic: Option<String>,

    /// Specify a message that will be stored by the broker and sent out if this client disconnects unexpectedly.
    #[structopt(long)]
    will_payload: Option<String>,

    /// The QoS to use for the Will.
    #[structopt(long, default_value = "at-most-once", parse(try_from_str = parse_qos))]
    will_qos: QoS,

    /// If given, if the client disconnects unexpectedly the message sent out will be treated as a retained message.
    #[structopt(long)]
    will_retain: bool,

    /// Provide a username to be used for authenticating with the broker.
    #[structopt(short, long)]
    username: Option<String>,

    /// Provide a password to be used for authenticating with the broker.
    #[structopt(short = "P", long)]
    password: Option<String>,

    /// Use an MQTT v5 property
    #[structopt(short = "D", long)]
    property: Vec<String>,

    /// Disconnect and exit the program immediately after the given count of messages have been received.
    /// This may be useful in shell scripts where on a single status value is required, for example.
    #[structopt(short = "C")]
    count: Option<usize>,

    /// If this option is given, sub_client will exit immediately that all of its subscriptions have been acknowledged by the broker.
    #[structopt(short = "E")]
    exit_after_subscribe: bool,

    /// If this argument is given, messages that are received that have the retain bit set will not be printed.
    /// Messages with retain set are "stale", in that it is not known when they were originally published.
    /// When subscribing to a wildcard topic there may be a large number of retained messages. This argument suppresses their display.
    #[structopt(short = "R")]
    no_retain: bool,

    /// The MQTT topic to subscribe to.
    #[structopt(short, long)]
    topic: Vec<String>,

    /// Suppress printing of topics that match the filter.
    /// This allows subscribing to a wildcard topic and only printing a partial set of the wildcard hierarchy.
    #[structopt(short = "T", long)]
    filter_out: Vec<String>,

    /// Specify the quality of service desired for the incoming messages.
    #[structopt(short, long, default_value = "at-most-once", parse(try_from_str = parse_qos))]
    qos: QoS,

    /// Do not append an end of line character to the payload when printing.
    /// This allows streaming of payload data from multiple messages directly to another application unmodified.
    #[structopt(short = "N")]
    no_eol: bool,
}

fn parse_protocol_version(s: &str) -> Result<ProtocolVersion> {
    match s {
        "v3" | "311" | "3.11" => Ok(ProtocolVersion::V311),
        "v5" | "5" | "5.0" => Ok(ProtocolVersion::V5),
        _ => Err(anyhow!("invalid protocol version: {}", s)),
    }
}

fn parse_qos(s: &str) -> Result<QoS> {
    match s {
        "0" | "at-most-once" => Ok(QoS::AtMostOnce),
        "1" | "at-least-once" => Ok(QoS::AtLeastOnce),
        "2" | "exactly-once" => Ok(QoS::ExactlyOnce),
        _ => Err(anyhow!("invalid QoS: {}", s)),
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    debug!("{:#?}", opt);

    let c = connect(opt.server()?)?;

    match opt.protocol_version {
        ProtocolVersion::V311 => run::<MQTT_V311, _>(c, &opt)?,
        ProtocolVersion::V5 => run::<MQTT_V5, _>(c, &opt)?,
    }

    Ok(())
}

fn run<P, T>(c: Connected<T>, opt: &Opt) -> Result<()>
where
    T: io::Read + io::Write,
    P: Protocol,
{
    let session = c.handshake::<P>(opt)?;

    session.disconnect()?;

    Ok(())
}

impl Opt {
    fn server(&self) -> io::Result<(&str, u16)> {
        if let Some(ref url) = self.url {
            let host = url
                .host_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing hostname"))?;
            let port = url
                .port()
                .or_else(|| match url.scheme() {
                    "mqtt" => Some(1883),
                    "mqtts" => Some(8883),
                    _ => None,
                })
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "unexpected scheme"))?;

            Ok((host, port))
        } else {
            Ok((self.host.as_str(), self.port))
        }
    }

    fn client_id(&self) -> String {
        if let Some(ref id) = self.id {
            id.clone()
        } else {
            format!("{}{}", self.id_prefix, process::id())
        }
    }
}

fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Connected<TcpStream>> {
    TcpStream::connect(addr).map(Connected)
}

struct Connected<T>(T);

impl<T> Connected<T> {
    fn handshake<P: Protocol>(self, opt: &Opt) -> Result<Session<T, P>>
    where
        T: io::Read + io::Write,
    {
        let Connected(mut stream) = self;
        let client_id = opt.client_id();
        let mut connect =
            mqtt_proto::connect::<P>(Duration::from_secs(opt.keep_alive as u64), &client_id);

        if let Some(ref username) = opt.username {
            connect.with_username(&username);

            if let Some(ref password) = opt.password {
                connect.with_password(&password);
            }
        }

        if let (Some(topic_name), Some(payload)) =
            (opt.will_topic.as_ref(), opt.will_payload.as_ref())
        {
            connect.with_last_will(
                topic_name,
                payload.as_bytes(),
                opt.will_qos,
                opt.will_retain,
            );
        }

        stream.send(connect)?;

        let mut buf = [0; MAX_PACKET_SIZE];
        let (remaining, packet) = stream.receive(&mut buf, opt.protocol_version)?;

        if let Packet::ConnectAck(connect_ack) = packet {
            connect_ack
                .return_code
                .ok()
                .context("handshake")
                .map(|_| Session::new(stream))
        } else {
            Err(anyhow!("unexpected packet: {:?}", packet))
        }
    }
}

struct Session<T, P> {
    stream: T,
    phantom: PhantomData<P>,
    packet_id: u16,
}

impl<T, P> Session<T, P> {
    pub fn new(stream: T) -> Self {
        Session {
            stream,
            phantom: PhantomData,
            packet_id: 0,
        }
    }

    fn next_packet_id(&mut self) -> u16 {
        let next = self.packet_id;
        self.packet_id = self.packet_id.wrapping_add(1);
        next
    }

    pub fn disconnect(mut self) -> io::Result<()>
    where
        T: io::Write,
    {
        self.stream.send(disconnect::<P>())
    }

    pub fn subscribe(&mut self, opt: &Opt) -> Result<()>
    where
        T: io::Read + io::Write,
    {
        let packet_id = self.next_packet_id();

        Ok(())
    }
}

trait WriteExt {
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()>;
}

impl<T: io::Write> WriteExt for T {
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()> {
        let packet = packet.into();
        let mut buf = vec![];
        packet.write_to(&mut buf);
        self.write_all(&buf)?;
        debug!(
            "write {:#?} packet to {} bytes:\n{}",
            packet,
            buf.len(),
            HexViewBuilder::new(&buf).finish()
        );
        Ok(())
    }
}

trait ReadExt {
    fn receive<'a>(
        &mut self,
        buf: &'a mut [u8],
        protocol_version: ProtocolVersion,
    ) -> Result<(&'a [u8], Packet<'a>)>;
}

impl<T: io::Read> ReadExt for T {
    fn receive<'a>(
        &mut self,
        buf: &'a mut [u8],
        protocol_version: ProtocolVersion,
    ) -> Result<(&'a [u8], Packet<'a>)> {
        let read = self.read(buf)?;
        let input = &buf[..read];

        let (remaining, packet) = Packet::parse::<()>(input, protocol_version)
            .map_err(|err| anyhow!("parse packet failed, {:?}", err))?;
        debug!(
            "read {:#?} packet from {} bytes:\n{}",
            packet,
            read,
            HexViewBuilder::new(input).finish()
        );

        Ok((remaining, packet))
    }
}
