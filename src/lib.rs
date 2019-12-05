#[macro_use]
extern crate log;


use std::sync::Arc;
use std::net::SocketAddr;
use std::process::exit;

use rustls::ClientConfig;

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::lock::Mutex;
use futures::SinkExt;
use futures::stream::StreamExt;

use tokio::spawn;

use h2::client::SendRequest;

use bytes::Bytes;

use clap::{App, Arg};


mod dns;
use dns::{DnsPacket, DnsCodec};
pub use dns::UdpListenSocket;

mod http2;
use http2::{create_config, http2_request};

mod cache;
pub use cache::Cache;


/// Get the `clap::App` object for the argument parsing.
pub fn get_app() -> App<'static, 'static> {
    App::new("DNS over HTTPS client")
        .version("1.4.5")
        .author("link.ted@mailbox.org")
        .about("Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.\n\
        By default, the client will connect to the Cloudflare DNS service.\n\
        This binary uses the env_logger as logger implementations. See https://github.com/sebasmagri/env_logger/")
        .arg(Arg::with_name("listen-addr")
            .short("l")
            .long("listen-addr")
            .conflicts_with("listen-activation")
            .takes_value(true)
            .value_name("Addr")
            .help("Listen address [default: 127.0.0.1:53]")
            .required(false))
        .arg(Arg::with_name("listen-activation")
            .long("listen-activation")
            .help("Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS")
            .required(false))
        .arg(Arg::with_name("remote-addr")
            .short("r")
            .long("remote-addr")
            .takes_value(true)
            .value_name("Addr")
            .help("Remote address")
            .default_value("1.1.1.1:443")
            .required(false))
        .arg(Arg::with_name("domain")
            .short("d")
            .long("domain")
            .takes_value(true)
            .value_name("Domain")
            .help("The domain name of the remote server")
            .default_value("cloudflare-dns.com")
            .required(false))
        .arg(Arg::with_name("retries")
            .takes_value(true)
            .long("retries")
            .value_name("UNSIGNED INT")
            .help("The number of retries to connect to the remote server")
            .default_value("3")
            .required(false))
        .arg(Arg::with_name("timeout")
            .takes_value(true)
            .short("t")
            .long("timeout")
            .value_name("UNSIGNED LONG")
            .help("The time in seconds after that the connection would be closed if no response is received from the server")
            .default_value("2")
            .required(false))
        .arg(Arg::with_name("cafile")
            .takes_value(true)
            .value_name("CAFILE")
            .help("The path to the pem file, which contains the trusted CA certificates")
            .required(true))
        .arg(Arg::with_name("path")
            .short("p")
            .long("path")
            .takes_value(true)
            .value_name("STRING")
            .help("The path of the URI")
            .default_value("dns-query")
            .required(false))
        .arg(Arg::with_name("get")
            .short("g")
            .long("get")
            .help("Use the GET method for the HTTP/2.0 request")
            .required(false))
        .arg(Arg::with_name("cache-size")
            .long("cache-size")
            .short("c")
            .takes_value(true)
            .value_name("UNSIGNED LONG")
            .help("The size of the private HTTP cache\nIf the size is 0 then the private HTTP cache is not used (ignores cache-control)")
            .default_value("1024")
            .required(false))
        .arg(Arg::with_name("cache-fallback")
            .long("cache-fallback")
            .help("Use expired cache entries if no response is received from the server")
            .required(false))
}

/// The configuration object for the `doh-client`.
pub struct Config {
    listen_socket: UdpListenSocket,
    remote_addr: SocketAddr,
    domain: String,
    client_config: Arc<ClientConfig>,
    uri: String,
    retries: u32,
    timeout: u64,
    post: bool,
    cache_size: usize,
    cache_fallback: bool,
}

impl Config {
    /// Create a new `doh_client::Config` object.
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

        Config { listen_socket, remote_addr, domain: domain.to_string(), client_config: Arc::new(client_config), uri, retries, timeout, post, cache_size, cache_fallback }
    }
}

/// The context object for a running instance.
pub struct Context {
    config: Config,
    sender: UnboundedSender<(DnsPacket, SocketAddr)>,
    mutex_send_request: Mutex<Option<SendRequest<Bytes>>>,
    mutex_cache: Mutex<Cache<Bytes, Bytes>>
}

impl Context {
    /// Create a new `doh_client::Context` object.
    pub fn new(config: Config, sender: UnboundedSender<(DnsPacket, SocketAddr)>) -> Context {
        let cache_size = config.cache_size;
        Context {
            config,
            sender,
            mutex_send_request: Mutex::new(None),
            mutex_cache: Mutex::new(Cache::new(cache_size))
        }
    }
}

/// Run the `doh-client` with a specific configuration.
pub async fn run(config: Config) {
    // === BEGIN UDP SETUP ===
    let (mut dns_sink, mut dns_stream) = match DnsCodec::new(config.listen_socket).await {
        Ok(result) => result,
        Err(e) => {
            error!("Cannot listen to UDP address {}: {}", config.listen_socket, e);
            exit(1);
        }
    };
    let (sender, receiver) = unbounded::<(DnsPacket, SocketAddr)>();
    // === END UDP SETUP ===

    let context: &'static Context = Box::leak(Box::new(Context::new(config, sender)));

    spawn(async move {
       while let Some(result) = dns_stream.next().await {
           match result {
               Ok((msg, addr)) => {
                   spawn(http2_request(msg, addr, context));
               }
               Err(e) => {
                   error!("next: {}", e);
                   exit(1);
               }
           }
       }
    });

    if let Err(e) = dns_sink.send_all(&mut receiver.map(Ok)).await {
        error!("send_all: {}", e);
        exit(1);
    }
}
