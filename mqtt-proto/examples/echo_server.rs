#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate error_chain;
extern crate getopts;

extern crate tokio_proto;

extern crate mqtt_proto as mqtt;

use std::env;
use std::iter::FromIterator;
use std::net::SocketAddr;
use std::process;
use std::sync::{Arc, Mutex};

use getopts::Options;

use tokio_proto::TcpServer;

use mqtt::server::{InMemoryAuthenticator, InMemorySessionProvider, InMemoryTopicProvider, Server};

error_chain!{
    links {
        MQTT(mqtt::errors::Error, mqtt::errors::ErrorKind);
    }
    foreign_links {
        IoError(::std::io::Error);
        OptionParseFail(::getopts::Fail);
        AddrParseError(::std::net::AddrParseError);
    }
    errors {
        MissingArgument(name: String) {
            description("missing required argument")
            display("missing required argument, {}", name)
        }
    }
}

struct Config {
    addr: SocketAddr,
    users: Vec<(String, Option<Vec<u8>>)>,
}

impl Config {
    fn authenticator(&self) -> Option<InMemoryAuthenticator> {
        if self.users.is_empty() {
            None
        } else {
            Some(InMemoryAuthenticator::from_iter(self.users.clone()))
        }
    }
}

fn parse_cmdline() -> Result<Config> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optflag("h", "help", "print this help menu");
    opts.optopt("l", "listen", "listen on the address", "ADDR");
    opts.optopt("u", "user", "add username with password", "USER:PASS");

    let matches = opts.parse(&args[1..])?;

    if matches.opt_present("h") {
        let brief = format!("Usage: {} [options]", program);

        print!("{}", opts.usage(&brief));

        process::exit(0);
    }

    Ok(Config {
        addr: matches
            .opt_str("listen")
            .ok_or_else(|| ErrorKind::MissingArgument("listen".to_owned()))?
            .parse()?,
        users: matches
            .opt_strs("user")
            .into_iter()
            .map(|s| if let Some(off) = s.find(':') {
                let (username, password) = s.split_at(off);
                (
                    username.to_owned(),
                    Some(password.as_bytes()[1..].to_owned()),
                )
            } else {
                (s, None)
            })
            .collect(),
    })
}

fn main() {
    let _ = pretty_env_logger::init().expect("fail to initial logger");

    let cfg = parse_cmdline().expect("fail to parse command line");

    let session_provider = Arc::new(Mutex::new(InMemorySessionProvider::default()));
    let topic_provider = InMemoryTopicProvider::default();
    let authenticator = cfg.authenticator().map(|authenticator| {
        Arc::new(Mutex::new(authenticator))
    });

    let server = TcpServer::new(mqtt::MQTT::default(), cfg.addr);

    server.with_handle(move |handle| {
        Server::with_authenticator(
            handle,
            Arc::clone(&session_provider),
            topic_provider.clone(),
            authenticator.clone(),
        )
    });
}
