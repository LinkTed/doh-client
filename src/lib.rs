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
}

impl Config {
    pub fn new(listen_socket: UdpListenSocket, remote_addr: SocketAddr, domain: &str, cafile: &str, retries: u16) -> Config {
        let client_config = match create_config(&cafile) {
            Ok(client_config) =>  client_config,
            Err(e) => {
                error!("Cannot open cafile: {}: {}", cafile, e);
                exit(1);
            }
        };

        Config {listen_socket, remote_addr, domain: domain.to_string(), client_config, retries}
    }
}

impl Clone for Config {
    fn clone(&self) -> Config {
        Config {listen_socket: self.listen_socket.clone(), remote_addr: self.remote_addr, domain: self.domain.clone(), client_config: self.client_config.clone(), retries: self.retries}
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

//        let mut guard_send_request = mutex_send_request.lock().unwrap();
//
//        if let None = *guard_send_request {
//            let mut guard_runtime_http = mutex_runtime_http.lock().unwrap();
//            *guard_send_request = match connect_http2_server(config.remote_addr, config.client_config.clone(), config.domain.clone(), config.retries) {
//                Ok(mut send_request) => {
//
//                    Some(send_request)
//                }
//                Err(_e) => {
//                    exit(1);
//                }
//            };
//        }
//
//        let result = match *guard_send_request {
//            Some(ref mut send_request) => {
//                send_request.send_request(request, false)
//            },
//            None => {
//                println!("That should not happen");
//                return Err(Error::new(ErrorKind::Other, "guard_send_request is None"));
//            }
//        };
//        drop(guard_send_request);
//
//        match result {
//            Ok((response, mut request)) => {
//                match request.send_data(msg.get_without_tid(), true) {
//                    Ok(()) => {
//                        let mutex_send_request  = mutex_send_request.clone();
//                        let mut guard_runtime_http = mutex_runtime_http.lock().unwrap();
//                        match *guard_runtime_http {
//                            Some(ref mut runtime_http) => {
//                                let mutex_runtime_http  = mutex_runtime_http.clone();
//                                runtime_http.spawn(Http2ResponseFuture::new(response, sender.clone(), addr, tid).timeout(Duration::from_secs(3)).map_err(move |_e| {
//                                    close_connection!(mutex_send_request, mutex_runtime_http);
//                                }));
//                            },
//                            None => {}
//                        }
//                        drop(guard_runtime_http);
//                    },
//                    Err(e) => {
//                        close_connection!(mutex_send_request, mutex_runtime_http);
//                    }
//                }
//            },
//            Err(e) => {
//                close_connection!(mutex_send_request, mutex_runtime_http);
//            }
//        }


        Ok(())
    });
    tokio::run(dns_queries.map_err(|e| {error!("UDP socket err: {}", e)}).join(dns_sink).map(|(_a, _b)| {
        error!("UDP error")
    }));
}
