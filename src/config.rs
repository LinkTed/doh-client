use crate::context::Context;
use crate::listen::Config as ListenConfig;
//use crate::listen::{handler as listen_handler, Config as ListenConfig};
use crate::remote::{Host as RemoteHost, Session as RemoteSession};
use crate::{get_listen_config, get_remote_host, Cache, DohError, DohResult};
use cfg_if::cfg_if;
use clap::{value_t, ArgMatches};
use futures::lock::Mutex;
use rustls::{ClientConfig, RootCertStore};
use std::fs::File;
use std::io::BufReader;
use std::io::Result as IoResult;
use std::sync::Arc;
use tokio::net::UdpSocket;

fn create_client_config(cafile: Option<&str>) -> DohResult<ClientConfig> {
    let root_store = load_root_store(cafile)?;
    let mut config = ClientConfig::new();
    config.root_store = root_store;
    config.alpn_protocols.push(vec![104, 50]); // h2
    Ok(config)
}

fn load_root_store(cafile: Option<&str>) -> DohResult<RootCertStore> {
    if let Some(cafile) = cafile {
        let certfile = File::open(&cafile)?;
        let mut root_store = RootCertStore::empty();
        if root_store
            .add_pem_file(&mut BufReader::new(certfile))
            .is_err()
        {
            return Err(DohError::PEMParser);
        }
        Ok(root_store)
    } else {
        cfg_if! {
            if #[cfg(feature = "native-certs")] {
                match rustls_native_certs::load_native_certs() {
                    Ok(root_store) => Ok(root_store),
                    Err((_, e)) => Err(e.into()),
                }
            } else {
                panic!("feature native-certs is not enabled")
            }
        }
    }
}

/// The configuration object for the `doh-client`.
pub struct Config {
    listen_config: ListenConfig,
    remote_host: RemoteHost,
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
    pub fn new(
        listen_config: ListenConfig,
        remote_host: RemoteHost,
        domain: &str,
        cafile: Option<&str>,
        path: &str,
        retries: u32,
        timeout: u64,
        post: bool,
        cache_size: usize,
        cache_fallback: bool,
    ) -> DohResult<Config> {
        let client_config = create_client_config(cafile)?;

        let uri = format!("https://{}/{}", domain, path);

        if cache_fallback && cache_size == 0 {
            return Err(DohError::CacheSize);
        }

        Ok(Config {
            listen_config,
            remote_host,
            domain: domain.to_string(),
            client_config: Arc::new(client_config),
            uri,
            retries,
            timeout,
            post,
            cache_size,
            cache_fallback,
        })
    }

    pub async fn try_from(matches: ArgMatches<'static>) -> DohResult<Config> {
        let listen_config = get_listen_config(&matches)?;
        let remote_host = get_remote_host(&matches).await?;
        let domain = matches.value_of("domain").unwrap();
        let cafile = matches.value_of("cafile");
        let path = matches.value_of("path").unwrap();
        let retries: u32 = value_t!(matches, "retries", u32).unwrap_or(3);
        let timeout: u64 = value_t!(matches, "timeout", u64).unwrap_or(2);
        let post: bool = !matches.is_present("get");
        let cache_size: usize = value_t!(matches, "cache-size", usize).unwrap_or(1024);
        let cache_fallback: bool = matches.is_present("cache-fallback");
        Config::new(
            listen_config,
            remote_host,
            domain,
            cafile,
            path,
            retries,
            timeout,
            post,
            cache_size,
            cache_fallback,
        )
    }

    pub(crate) async fn into(self) -> IoResult<(Arc<UdpSocket>, Context)> {
        let cache = if self.cache_size == 0 {
            None
        } else {
            Some(Mutex::new(Cache::new(self.cache_size)))
        };
        let cache_fallback = self.cache_fallback;
        let timeout = self.timeout;
        let socket = self.listen_config.into_socket().await?;
        let socket = Arc::new(socket);
        let remote_session = RemoteSession::new(
            self.remote_host,
            self.domain,
            self.client_config,
            self.uri,
            self.retries,
            self.post,
        );
        let context = Context::new(
            cache,
            cache_fallback,
            timeout,
            remote_session,
            socket.clone(),
        );
        Ok((socket.clone(), context))
    }
}
