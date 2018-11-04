#[macro_use]
extern crate log;
extern crate tokio;
extern crate http;
extern crate bytes;
extern crate rustls;
extern crate futures;
extern crate h2;
extern crate tokio_rustls;
extern crate webpki;


use std::net::SocketAddr;
use std::time::Duration;
use std::thread::sleep;
use std::process::exit;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};

use http::Request;
use bytes::Bytes;

use rustls::ClientConfig;

use futures::sync::mpsc::unbounded;
use futures::Sink;
use futures::Stream;
use futures::Future;

use tokio::runtime::Runtime;


mod dns;
use dns::DnsCodec;

mod http2;
use http2::{create_config, connect_http2_server, Http2ResponseFuture};

pub mod logger;


pub struct Config {
    listen_addr: SocketAddr,
    remote_addr: SocketAddr,
    domain: String,
    client_config: ClientConfig,
    retries: u16,
}

impl Config {
    pub fn new(listen_addr: SocketAddr, remote_addr: SocketAddr, domain: &str, cafile: &str, retries: u16) -> Config {
        let client_config = match create_config(&cafile) {
            Ok(client_config) =>  client_config,
            Err(e) => {
                error!("Cannot open cafile: {}: {}", cafile, e);
                exit(1);
            }
        };

        Config{listen_addr, remote_addr, domain: domain.to_string(), client_config, retries}
    }
}


pub fn run(config: Config) {
    loop {
        let mut runtime = Runtime::new().unwrap();
        // UDP
        let (dns_sink, dns_stream) = match DnsCodec::new(config.listen_addr) {
            Ok(result) => result,
            Err(e) => {
                error!("Cannot listen to UDP address {}: {}", config.listen_addr, e);
                exit(1);
            }
        };
        let (sender, receiver) = unbounded::<(Bytes, SocketAddr)>();
        let sender = Arc::new(Mutex::new(sender));
        runtime.spawn(
            dns_sink.send_all(receiver
                .map_err(|_| {
                    Error::new(ErrorKind::Other, "receiver")
                }))
                .map(|_| {})
                .map_err(|e| error!("dns_sink: {}", e))
        );

        info!("Connect to remote server {} ({})", config.remote_addr, config.domain);
        if let Ok(mut send_request) = connect_http2_server(&mut runtime, config.remote_addr, config.client_config.clone(), config.domain.to_string(), config.retries) {
            info!("Connection was successfully established to remote server {} ({})", config.remote_addr, config.domain);
            let executor = runtime.executor();
            let dns_queries = dns_stream.for_each(move |(msg, addr)| {
                let body = msg.freeze();

                debug!("Received UDP packet from {}", addr);

                let request = Request::builder()
                    .method("POST")
                    .uri("https://cloudflare-dns.com/dns-query")
                    .header("accept", "application/dns-message")
                    .header("content-type", "application/dns-message")
                    .header("content-length", body.len().to_string())
                    .body(())
                    .unwrap();

                match send_request.send_request(request, false) {
                    Ok((response, mut request)) => {
                        match request.send_data(body, true) {
                            Ok(()) => {
                                executor.spawn(Http2ResponseFuture::new(response, sender.clone(), addr));
                                return Ok(());
                            },
                            Err(e) => {
                                return if e.is_io() {
                                    Err(e.into_io().unwrap())
                                } else {
                                    Err(Error::new(ErrorKind::Other, e))
                                };
                            }
                        }
                    },
                    Err(e) => {
                        return if e.is_io() {
                            Err(e.into_io().unwrap())
                        } else {
                            Err(Error::new(ErrorKind::Other, e))
                        };
                    }
                }
            });

            match runtime.block_on(dns_queries) {
                Ok(()) => info!("That should not happen"),
                Err(e) => error!("Connection to remote server lost {} ({}): {}", config.remote_addr, config.domain, e)
            }

            runtime.shutdown_on_idle();
            sleep(Duration::from_millis(200));
        } else {
            error!("Too many connection attempts to remote server {} ({})", config.remote_addr, config.domain);
            exit(1);
        }
    }
}
