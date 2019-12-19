#[macro_use]
extern crate log;

use std::fmt;
use std::process;

use anyhow::{anyhow, Result};
use hexplay::HexViewBuilder;
use structopt::StructOpt;
use url::Url;

use mqtt_sync_client::{
    mqtt::{ProtocolVersion, QoS},
    proto::*,
    Connector,
};

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
    #[structopt(short = "C", long)]
    count: Option<usize>,

    /// If this option is given, sub_client will exit immediately that all of its subscriptions have been acknowledged by the broker.
    #[structopt(short = "E")]
    exit_after_subscribe: bool,

    /// If this argument is given, messages that are received that have the retain bit set will not be printed.
    /// Messages with retain set are "stale", in that it is not known when they were originally published.
    /// When subscribing to a wildcard topic there may be a large number of retained messages. This argument suppresses their display.
    #[structopt(short = "R", long)]
    no_retain: bool,

    /// If this argument is given, only messages that are received that have the retain bit set will be printed.
    ///
    /// Messages with retain set are "stale", in that it is not known when they were originally published.
    /// With this argument in use, the receipt of the first non-stale message will cause the client to exit.
    /// See also the --retain-as-published option.
    #[structopt(long)]
    retain_only: bool,

    /// If this argument is given, the when `sub_client` receives a message with the retained bit set,
    /// it will send a message to the broker to clear that retained message.
    ///
    /// This applies to all received messages except those that are filtered out by the -T option.
    /// This option still takes effect even if -R is used. See also the --retain-as-published and --retained-only options.
    #[structopt(long)]
    remove_retained: bool,

    /// The MQTT topic to subscribe to.
    #[structopt(short, long)]
    topic: Vec<String>,

    /// A topic that will be unsubscribed from.
    ///
    /// This may be used on its own or in conjunction with the --topic option
    /// and only makes sense when used in conjunction with --clean-session.
    ///
    /// If used with --topic then subscriptions will be processed before unsubscriptions.
    #[structopt(short = "U", long)]
    unsubscribe: Vec<String>,

    /// Suppress printing of topics that match the filter.
    /// This allows subscribing to a wildcard topic and only printing a partial set of the wildcard hierarchy.
    #[structopt(short = "T", long)]
    filter_out: Vec<String>,

    /// Specify the quality of service desired for the incoming messages.
    #[structopt(short, long, default_value = "at-most-once", parse(try_from_str = parse_qos))]
    qos: QoS,

    /// Do not append an end of line character to the payload when printing.
    /// This allows streaming of payload data from multiple messages directly to another application unmodified.
    #[structopt(short = "N", long)]
    no_eol: bool,

    /// Print received messages verbosely.
    ///
    /// With this argument, messages will be printed as "topic payload".
    /// When this argument is not given, the messages are printed as "payload".
    #[structopt(short, long)]
    verbose: bool,
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
    fn server(&self) -> Result<(&str, u16)> {
        if let Some(ref url) = self.url {
            let host = url.host_str().ok_or_else(|| anyhow!("missing hostname"))?;
            let port = url
                .port()
                .or_else(|| match url.scheme() {
                    "mqtt" => Some(1883),
                    "mqtts" => Some(8883),
                    _ => None,
                })
                .ok_or_else(|| anyhow!("unexpected scheme"))?;

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
    let addr = opt.server()?;
    let mut connector = Connector::<_, P>::new(addr);

    connector.keep_alive = opt.keep_alive;

    let client_id = opt.client_id();
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

    if !opt.topic.is_empty() {
        let subscribed = client.subscribe(opt.topic.iter().map(|s| (s.as_str(), opt.qos)))?;

        if let Some(ref reason) = subscribed.reason {
            info!("subscribe reason: {}", reason)
        }

        for (topic_name, status) in opt.topic.iter().zip(subscribed) {
            match status {
                Ok(qos) => info!("{} subscribed as `{}`", topic_name, qos),
                Err(reason) => warn!("fail to subscribe `{}`, {}", topic_name, reason),
            }
        }
    }

    if !opt.unsubscribe.is_empty() {
        let unsubscribed = client.unsubscribe(opt.unsubscribe.iter().map(|s| s.as_str()))?;

        if let Some(ref reason) = unsubscribed.reason {
            info!("unsubscribe reason: {}", reason)
        }

        for (topic_filter, status) in opt.unsubscribe.iter().zip(unsubscribed) {
            match status {
                Ok(_) => info!("{} unsubscribed", topic_filter),
                Err(reason) => warn!("fail to subscribe `{}`, {}", topic_filter, reason),
            }
        }
    }

    if !opt.exit_after_subscribe {
        let mut messages = client.messages();

        for message in messages
            .by_ref()
            .take(opt.count.unwrap_or(usize::max_value()))
        {
            match message {
                Ok(message) => {
                    if message.payload.is_empty() {
                        continue;
                    }
                    if opt.remove_retained && message.retain {
                        // mosquitto_publish(mosq, &last_mid, message->topic, 0, NULL, 1, true);
                    }
                    if opt.no_retain && message.retain {
                        continue;
                    }
                    if opt.retain_only && !message.retain {
                        continue;
                    }

                    print!(
                        "{}",
                        MessageFmt {
                            message,
                            verbose: opt.verbose,
                            eol: !opt.no_eol,
                        }
                    )
                }
                Err(err) => warn!("fail to receive message: {}", err),
            }
        }

        client = messages.into();
    }

    client.disconnect()?;

    Ok(())
}

struct MessageFmt<'a> {
    message: Message<'a>,
    verbose: bool,
    eol: bool,
}

impl<'a> fmt::Display for MessageFmt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.verbose {
            f.write_fmt(format_args!("{} ", self.message.topic_name))?;
        }
        if !self.message.payload.is_empty() {
            if self.verbose {
                f.write_fmt(format_args!(
                    "\n{}",
                    HexViewBuilder::new(&self.message.payload).finish()
                ))?;
            } else {
                for b in self.message.payload.iter() {
                    f.write_fmt(format_args!("{:x}", b))?;
                }
            }
        } else if self.verbose {
            f.write_str("(null)")?;
        }
        if self.eol {
            f.write_str("\n")?;
        }

        Ok(())
    }
}
