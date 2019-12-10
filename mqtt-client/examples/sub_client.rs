#[macro_use]
extern crate log;

use core::marker::PhantomData;

use std::io;
use std::process;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use structopt::StructOpt;
use url::Url;

use mqtt_client::sync::Connector;
use mqtt_core::{ProtocolVersion, QoS};
use mqtt_packet::{self, Packet};
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

fn main() -> Result<()> {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    debug!("{:#?}", opt);

    match opt.protocol_version {
        ProtocolVersion::V311 => run::<MQTT_V311>(opt),
        ProtocolVersion::V5 => run::<MQTT_V5>(opt),
    }
}

fn run<P>(opt: Opt) -> Result<()>
where
    P: Protocol,
{
    let client_id = opt.client_id();
    let mut connector = Connector::<_, P>::new(opt.server()?);

    connector.keep_alive = opt.keep_alive;
    connector.client_id = client_id.as_str();

    if let Some(ref username) = opt.username {
        connector.with_username(&username);

        if let Some(ref password) = opt.password {
            connector.with_password(&password);
        }
    }

    if let (Some(topic_name), Some(payload)) = (opt.will_topic.as_ref(), opt.will_payload.as_ref())
    {
        connector.with_last_will(
            topic_name,
            payload.as_bytes(),
            opt.will_qos,
            opt.will_retain,
        );
    }

    let mut client = connector.connect()?;

    client.disconnect()?;

    // let client = connect()
    // let session = c.handshake::<P>(opt)?;

    // session.disconnect()?;

    Ok(())
}
