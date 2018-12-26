extern crate log;
#[macro_use]
extern crate clap;
extern crate doh_client;


use log::{set_max_level, set_logger, LevelFilter};

use clap::{Arg, App, ArgGroup};

use doh_client::{Config, run};
use doh_client::logger::Logger;

use std::net::SocketAddr;
use std::process::exit;

use doh_client::dns::UdpListenSocket::*;

static LOGGER: Logger = Logger{};


fn main() {
    let matches = App::new("DNS over HTTPS client")
        .version("1.1.2")
        .author("link.ted@mailbox.org")
        .about("Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.\nBy default, the client will connect to the Cloudflare DNS service.")
        .arg(Arg::with_name("listen-addr")
            .short("l")
            .long("listen-addr")
            .value_name("Addr")
            .help("Listen address [default: 127.0.0.1:53]")
            .required(false))
        .arg(Arg::with_name("listen-activation")
            .long("listen-activation")
            .help("Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS")
            .required(false))
        .group(ArgGroup::with_name("listen")
            .arg("listen-addr")
            .arg("listen-activation"))
        .arg(Arg::with_name("remote-addr")
            .short("r")
            .long("remote-addr")
            .value_name("Addr")
            .help("Remote address")
            .default_value("1.1.1.1:443")
            .required(false))
        .arg(Arg::with_name("domain")
            .short("d")
            .long("domain")
            .value_name("Domain")
            .help("The domain name of the remote server")
            .default_value("cloudflare-dns.com")
            .required(false))
        .arg(Arg::with_name("retries")
            .long("retries")
            .value_name("UNSIGNED INT")
            .help("The number of retries to connect to the remote server")
            .default_value("3")
            .required(false))
        .arg(Arg::with_name("timeout")
            .long("timeout")
            .value_name("UNSIGNED LONG")
            .help("The time in seconds after that the connection would be closed if no response is received from the server")
            .default_value("2")
            .required(false))
        .arg(Arg::with_name("cafile")
            .short("c")
            .long("cafile")
            .value_name("FILE")
            .help("The path to the pem file, which contains the trusted CA certificates")
            .required(true))
        .arg(Arg::with_name("path")
            .short("p")
            .long("path")
            .value_name("STRING")
            .help("The path of the URI")
            .default_value("dns-query")
            .required(false))
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .get_matches();

    let listen_socket = if matches.is_present("listen-activation") {
        Activation
    } else {
        if matches.is_present("listen-addr") {
            Addr(matches.value_of("listen-addr").unwrap().parse().unwrap())
        } else {
            Addr("127.0.0.1:53".parse().unwrap())
        }
    };
    let remote_addr: SocketAddr = matches.value_of("remote-addr").unwrap().parse().unwrap();
    let domain = matches.value_of("domain").unwrap();
    let cafile = matches.value_of("cafile").unwrap();
    let path = matches.value_of("path").unwrap();
    let retries: u32 = value_t!(matches, "retries", u32).unwrap_or(3);
    let timeout: u64 = value_t!(matches, "timeout", u64).unwrap_or(2);

    if let Err(e) = set_logger(&LOGGER) {
        eprintln!("Could not set logger: {}", e);
        exit(1);
    }

    match matches.occurrences_of("v") {
        0 => set_max_level(LevelFilter::Error),
        1 => set_max_level(LevelFilter::Warn),
        2 => set_max_level(LevelFilter::Info),
        3 => set_max_level(LevelFilter::Debug),
        4 | _ => set_max_level(LevelFilter::Trace),
    }

    run(Config::new(listen_socket, remote_addr, domain, cafile, path, retries, timeout));
}