use crate::{
    context::Context,
    helper::{load_certs, load_private_key, load_root_store},
    listen::Config as ListenConfig,
    remote::{Host as RemoteHost, Session as RemoteSession},
    {get_listen_config, get_remote_host, Cache, DohError, DohResult},
};
use clap::ArgMatches;
use futures::lock::Mutex;
use rustls::ClientConfig;
use std::{io::Result as IoResult, sync::Arc};
use tokio::net::UdpSocket;

fn create_client_config(
    cafile: Option<&str>,
    client_auth: Option<(&str, &str)>,
) -> DohResult<ClientConfig> {
    let root_store = load_root_store(cafile)?;
    let config_builder = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store);
    let mut config = if let Some((certs, key)) = client_auth {
        let cert_chain = load_certs(certs)?;
        let key_der = load_private_key(key)?;
        config_builder.with_single_cert(cert_chain, key_der)?
    } else {
        config_builder.with_no_client_auth()
    };
    config.alpn_protocols.push(vec![104, 50]); // h2
    Ok(config)
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
        client_auth: Option<(&str, &str)>,
        path: &str,
        retries: u32,
        timeout: u64,
        post: bool,
        cache_size: usize,
        cache_fallback: bool,
    ) -> DohResult<Config> {
        let client_config = create_client_config(cafile, client_auth)?;

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

    pub async fn try_from(matches: ArgMatches) -> DohResult<Config> {
        let listen_config = get_listen_config(&matches)?;
        let remote_host = get_remote_host(&matches).await?;
        let domain = matches.value_of("domain").unwrap();
        let cafile = matches.value_of("cafile");
        let client_auth = matches
            .value_of("client-auth-certs")
            .map(|certs| (certs, matches.value_of("client-auth-key").unwrap()));
        let path = matches.value_of("path").unwrap();
        let retries: u32 = matches.value_of_t("retries").unwrap_or(3);
        let timeout: u64 = matches.value_of_t("timeout").unwrap_or(2);
        let post: bool = !matches.is_present("get");
        let cache_size: usize = matches.value_of_t("cache-size").unwrap_or(1024);
        let cache_fallback: bool = matches.is_present("cache-fallback");
        Config::new(
            listen_config,
            remote_host,
            domain,
            cafile,
            client_auth,
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
        Ok((socket, context))
    }
}
