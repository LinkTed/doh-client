use crate::listen::{handler as listen_handler, Config as ListenConfig};
use crate::remote::{Host as RemoteHost, Session as RemoteSession};
use crate::{Cache, Context, DohError, DohResult};
use cfg_if::cfg_if;
use futures::lock::Mutex;
use rustls::ClientConfig;
use rustls::RootCertStore;
use std::fs::File;
use std::io::BufReader;
use std::io::Result as IoResult;
use std::sync::Arc;
use tokio::net::udp::RecvHalf;

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

    pub(crate) async fn into(self) -> IoResult<(RecvHalf, Context)> {
        let cache = if self.cache_size == 0 {
            None
        } else {
            Some(Mutex::new(Cache::new(self.cache_size)))
        };
        let cache_fallback = self.cache_fallback;
        let timeout = self.timeout;
        let socket = self.listen_config.into_socket().await?;
        let (recv, sender) = listen_handler(socket);
        let remote_session = RemoteSession::new(
            self.remote_host,
            self.domain,
            self.client_config,
            self.uri,
            self.retries,
            self.post,
        );
        let context = Context::new(cache, cache_fallback, timeout, remote_session, sender);
        Ok((recv, context))
    }
}
