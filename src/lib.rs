#[macro_use]
extern crate log;
extern crate tokio;
extern crate http;
extern crate bytes;
extern crate rustls;
extern crate futures;
extern crate futures_locks;
extern crate h2;
extern crate tokio_rustls;
extern crate webpki;


use std::net::SocketAddr;
use std::process::exit;
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use rustls::ClientConfig;

use futures::sync::mpsc::unbounded;
use futures::{Sink, Stream, Future};

use futures_locks::Mutex;


pub mod dns;
use dns::{DnsPacket, DnsCodec, UdpListenSocket};

mod http2;
use http2::{create_config, Http2RequestFuture};
use h2::client::SendRequest;
use bytes::Bytes;

pub mod logger;


pub struct Config {
    listen_socket: UdpListenSocket,
    remote_addr: SocketAddr,
    domain: String,
    client_config: ClientConfig,
    retries: u16,
    timeout: u64
}

impl Config {
    pub fn new(listen_socket: UdpListenSocket, remote_addr: SocketAddr, domain: &str, cafile: &str, retries: u16, timeout: u64) -> Config {
        let client_config = match create_config(&cafile) {
            Ok(client_config) =>  client_config,
            Err(e) => {
                error!("Cannot open cafile: {}: {}", cafile, e);
                exit(1);
            }
        };

        Config {listen_socket, remote_addr, domain: domain.to_string(), client_config, retries, timeout}
    }
}

impl Clone for Config {
    fn clone(&self) -> Config {
        Config {listen_socket: self.listen_socket.clone(), remote_addr: self.remote_addr, domain: self.domain.clone(), client_config: self.client_config.clone(), retries: self.retries, timeout: self.timeout}
    }
}

pub fn run(config: Config) {
    // UDP
    let (dns_sink, dns_stream) = match DnsCodec::new(config.listen_socket) {
        Ok(result) => result,
        Err(e) => {
            error!("Cannot listen to UDP address {}: {}", config.listen_socket, e);
            exit(1);
        }
    };
    let (sender, receiver) = unbounded::<(DnsPacket, SocketAddr)>();
    let sender = Arc::new(sender);

    let dns_sink = dns_sink.send_all(receiver
        .map_err(|_| {
            Error::new(ErrorKind::Other, "receiver")
        }))
        .map_err(|e| error!("dns_sink: {}", e));

    let mutex_send_request: Mutex<(Option<SendRequest<Bytes>>, u16)> = Mutex::new((None, 0));
    let dns_queries = dns_stream.for_each(move |(msg, addr)| {
        tokio::spawn(Http2RequestFuture::new(mutex_send_request.clone(), msg, addr, sender.clone(), config.clone()));

        Ok(())
    });
    tokio::run(dns_queries.map_err(|e| {error!("UDP socket err: {}", e)}).join(dns_sink).map(|(_a, _b)| {
        error!("UDP error")
    }));
}
