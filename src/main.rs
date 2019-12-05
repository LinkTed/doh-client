#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;


use std::net::SocketAddr;
use std::process::exit;

use env_logger::Builder;

use doh_client::{Config, run, UdpListenSocket, get_app};


#[tokio::main]
async fn main() {
    let matches = get_app().get_matches();

    let mut builder = Builder::from_default_env();
    builder.format_timestamp(None).init();

    let listen_socket = if matches.is_present("listen-activation") {
        UdpListenSocket::Activation
    } else {
        if matches.is_present("listen-addr") {
            match matches.value_of("listen-addr").unwrap().parse() {
                Ok(addr) => UdpListenSocket::Addr(addr),
                Err(e) => {
                    error!("Could not parse listen address: {}", e);
                    exit(1);
                }
            }
        } else {
            UdpListenSocket::Addr("127.0.0.1:53".parse().unwrap())
        }
    };
    let remote_addr: SocketAddr = match matches.value_of("remote-addr").unwrap().parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Could not parse remote address: {}", e);
            exit(1);
        }
    };
    let domain = matches.value_of("domain").unwrap();
    let cafile = matches.value_of("cafile").unwrap();
    let path = matches.value_of("path").unwrap();
    let retries: u32 = value_t!(matches, "retries", u32).unwrap_or(3);
    let timeout: u64 = value_t!(matches, "timeout", u64).unwrap_or(2);
    let post: bool = !matches.is_present("get");
    let cache_size: usize = value_t!(matches, "cache-size", usize).unwrap_or(1024);
    let cache_fallback: bool = matches.is_present("cache-fallback");
    let config = Config::new(listen_socket, remote_addr, domain, cafile, path, retries, timeout, post, cache_size, cache_fallback);
    run(config).await
}
