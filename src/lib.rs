extern crate libc;
extern crate data_encoding;
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
extern crate lru;

use tokio::runtime::Runtime;

use std::net::SocketAddr;
use std::process::exit;
use std::io::{Error, ErrorKind};

use rustls::ClientConfig;

use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::{Sink, Stream, Future};

use futures_locks::Mutex;

use h2::client::SendRequest;

use bytes::Bytes;

mod logger;

pub use logger::Logger;

mod dns;

use dns::{DnsPacket, DnsCodec};
pub use dns::UdpListenSocket;

mod http2;
use http2::{create_config, Http2RequestFuture};

mod cache;
use cache::Cache;

#[cfg(test)]
mod tests;


pub struct Config {
    listen_socket: UdpListenSocket,
    remote_addr: SocketAddr,
    domain: String,
    client_config: ClientConfig,
    uri: String,
    retries: u32,
    timeout: u64,
    post: bool,
    cache_size: usize,
    cache_fallback: bool,
}

impl Config {
    pub fn new(listen_socket: UdpListenSocket, remote_addr: SocketAddr, domain: &str, cafile: &str, path: &str, retries: u32, timeout: u64, post: bool, cache_size: usize, cache_fallback: bool) -> Config {
        let client_config = match create_config(&cafile) {
            Ok(client_config) => client_config,
            Err(e) => {
                error!("Cannot open cafile: {}: {}", cafile, e);
                exit(1);
            }
        };

        let uri = format!("https://{}/{}", domain, path);

        if cache_fallback && cache_size == 0 {
            error!("cache size is zero and cache fallback is enabled simultaneously");
            exit(1);
        }

        Config { listen_socket, remote_addr, domain: domain.to_string(), client_config, uri, retries, timeout, post, cache_size, cache_fallback }
    }
}

pub struct Context {
    config: Config,
    sender: UnboundedSender<(DnsPacket, SocketAddr)>,
}

impl Context {
    pub fn new(config: Config, sender: UnboundedSender<(DnsPacket, SocketAddr)>) -> Context {
        Context { config, sender }
    }
}

pub fn run(config: Config) {
    let mut runtime = Runtime::new().expect("failed to start new Runtime");

    // === BEGIN UDP SETUP ===
    let (dns_sink, dns_stream) = match DnsCodec::new(config.listen_socket) {
        Ok(result) => result,
        Err(e) => {
            error!("Cannot listen to UDP address {}: {}", config.listen_socket, e);
            exit(1);
        }
    };
    let (sender, receiver) = unbounded::<(DnsPacket, SocketAddr)>();
    let dns_sink = dns_sink.send_all(receiver
        .map_err(|_| {
            Error::new(ErrorKind::Other, "receiver")
        }))
        .map_err(|e| error!("dns_sink: {}", e));
    // === END UDP SETUP ===

    let context: &'static Context = Box::leak(Box::new(Context::new(config, sender)));

    let mutex_send_request: Mutex<(Option<SendRequest<Bytes>>, u32)> = Mutex::new((None, 0));
    let mutex_cache: Mutex<Cache<Bytes, Bytes>> = Mutex::new(Cache::new(context.config.cache_size));
    let executor = runtime.executor();
    let dns_queries = dns_stream.for_each(move |(msg, addr)| {
        executor.spawn(Http2RequestFuture::new(mutex_send_request.clone(), mutex_cache.clone(), msg, addr, context));

        Ok(())
    });

    runtime.spawn(dns_queries.map_err(|e| {
        error!("dns_queries: {}", e)
    }));
    runtime.spawn(dns_sink.then(|_r| {
        Ok(())
    }));

    runtime.shutdown_on_idle().wait().unwrap();
}
