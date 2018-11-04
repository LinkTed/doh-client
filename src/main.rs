extern crate log;
#[macro_use]
extern crate clap;
extern crate doh_client;


use log::{set_max_level, set_logger, LevelFilter};
use clap::{Arg, App};
use doh_client::{Config, run};
use doh_client::logger::Logger;
use std::net::SocketAddr;
use std::process::exit;


static LOGGER: Logger = Logger{};


fn main() {
    let matches = App::new("DNS over HTTPS client")
        .version("1.0")
        .author("link.ted@mailbox.org")
        .about("Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.\nThe default values connect to the Cloudflare DNS.")
        .arg(Arg::with_name("listen-addr")
            .short("l")
            .long("listen-addr")
            .value_name("Addr")
            .help("Listen address")
            .default_value("127.0.0.1:53")
            .required(false))
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
            .help("The number of reties to connect to the remote server")
            .required(false))
        .arg(Arg::with_name("cafile")
            .short("c")
            .long("cafile")
            .value_name("FILE")
            .help("The path to the pem file, which contains the trusted CA certificates")
            .required(true))
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .get_matches();

    let listen_addr: SocketAddr = matches.value_of("listen-addr").unwrap().parse().unwrap();
    let remote_addr: SocketAddr = matches.value_of("remote-addr").unwrap().parse().unwrap();
    let domain = matches.value_of("domain").unwrap();
    let cafile = matches.value_of("cafile").unwrap();
    let retries: u16 = value_t!(matches, "retries", u16).unwrap_or(3);

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

    run(Config::new(listen_addr, remote_addr, domain, cafile, retries));
}