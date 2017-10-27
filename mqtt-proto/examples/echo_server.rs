#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate error_chain;
extern crate getopts;

extern crate tokio_proto;

extern crate mqtt_proto as mqtt;

use std::env;
use std::net::SocketAddr;
use std::process;
use std::sync::{Arc, Mutex};

use getopts::Options;

use tokio_proto::TcpServer;

use mqtt::server::{InMemorySessionProvider, Server};

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
}

fn parse_cmdline() -> Result<Config> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optflag("h", "help", "print this help menu");
    opts.optopt("l", "listen", "listen on the address", "ADDR");

    let matches = opts.parse(&args[1..])?;

    if matches.opt_present("h") {
        let brief = format!("Usage: {} [options]", program);

        print!("{}", opts.usage(&brief));

        process::exit(0);
    }

    Ok(Config {
        addr: matches
            .opt_str("l")
            .ok_or_else(|| ErrorKind::MissingArgument("listen".to_owned()))?
            .parse()?,
    })
}

fn main() {
    let _ = pretty_env_logger::init().expect("fail to initial logger");

    let cfg = parse_cmdline().expect("fail to parse command line");

    let session_manager = Arc::new(Mutex::new(InMemorySessionProvider::default()));

    let server = TcpServer::new(mqtt::Proto::default(), cfg.addr);

    server.with_handle(move |handle| {
        Arc::new(Server::new(handle, session_manager.clone()))
    });
}
