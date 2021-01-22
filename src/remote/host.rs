#[cfg(feature = "http-proxy")]
use super::try_http_proxy_connect;
use super::{try_http2_connect, try_tcp_connect, try_tls_connect};
#[cfg(feature = "socks5")]
use super::{try_socks5_connect, try_socks5h_connect};
use crate::DohResult;
use bytes::Bytes;
use h2::client::SendRequest;
use rustls::ClientConfig;
use std::fmt::{Display, Formatter, Result};
use std::sync::Arc;

pub enum Host {
    Direct(String, u16),
    #[cfg(feature = "socks5")]
    Socks5(
        String,
        u16,
        Option<(String, String)>,
        Vec<std::net::SocketAddr>,
    ),
    #[cfg(feature = "socks5")]
    Socks5h(String, u16, Option<(String, String)>, String, u16),
    #[cfg(feature = "http-proxy")]
    HttpProxy(String, u16, Option<(String, String)>, String, u16),
    #[cfg(feature = "http-proxy")]
    HttpsProxy(
        String,
        u16,
        Option<(String, String)>,
        String,
        u16,
        Arc<ClientConfig>,
        String,
    ),
}

impl Host {
    pub(super) async fn connect(
        &mut self,
        client_config: &Arc<ClientConfig>,
        domain: &str,
    ) -> DohResult<SendRequest<Bytes>> {
        match self {
            Host::Direct(remote_host, remote_port) => {
                let tcp_connection = try_tcp_connect(remote_host, *remote_port).await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = try_http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
            #[cfg(feature = "socks5")]
            Host::Socks5(proxy_host, proxy_port, credentials, remote_addrs) => {
                let tcp_connection =
                    try_socks5_connect(proxy_host, *proxy_port, remote_addrs, credentials).await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = try_http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
            #[cfg(feature = "socks5")]
            Host::Socks5h(proxy_host, proxy_port, credentials, remote_host, remote_port) => {
                let tcp_connection = try_socks5h_connect(
                    proxy_host,
                    *proxy_port,
                    remote_host,
                    *remote_port,
                    credentials,
                )
                .await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = try_http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
            #[cfg(feature = "http-proxy")]
            Host::HttpProxy(proxy_host, proxy_port, credentials, remote_host, remote_port) => {
                let mut tcp_connection = try_tcp_connect(proxy_host, *proxy_port).await?;
                try_http_proxy_connect(&mut tcp_connection, remote_host, *remote_port, credentials)
                    .await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = try_http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
            #[cfg(feature = "http-proxy")]
            Host::HttpsProxy(
                proxy_host,
                proxy_port,
                credentials,
                remote_host,
                remote_port,
                https_client_config,
                https_domain,
            ) => {
                let tcp_connection = try_tcp_connect(proxy_host, *proxy_port).await?;
                let mut tls_connection =
                    try_tls_connect(tcp_connection, https_client_config, https_domain).await?;
                try_http_proxy_connect(&mut tls_connection, remote_host, *remote_port, credentials)
                    .await?;
                let tls_connection = try_tls_connect(tls_connection, client_config, domain).await?;
                let http2_connection = try_http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
        }
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Host::Direct(remote_host, remote_port) => write!(f, "{}:{}", remote_host, remote_port),
            #[cfg(feature = "socks5")]
            Host::Socks5(proxy_host, proxy_port, _, remote_addrs) => {
                write!(
                    f,
                    "{}:{} via socks5 {:?}",
                    proxy_host, proxy_port, remote_addrs
                )
            }
            #[cfg(feature = "socks5")]
            Host::Socks5h(proxy_host, proxy_port, _, remote_host, remote_port) => {
                write!(
                    f,
                    "{}:{} via socks5h {}:{}",
                    proxy_host, proxy_port, remote_host, remote_port
                )
            }
            #[cfg(feature = "http-proxy")]
            Host::HttpProxy(proxy_host, proxy_port, _, remote_host, remote_port) => {
                write!(
                    f,
                    "{}:{} via http {}:{}",
                    proxy_host, proxy_port, remote_host, remote_port
                )
            }
            #[cfg(feature = "http-proxy")]
            Host::HttpsProxy(proxy_host, proxy_port, _, remote_host, remote_port, _, _) => {
                write!(
                    f,
                    "{}:{} via https {}:{}",
                    proxy_host, proxy_port, remote_host, remote_port
                )
            }
        }
    }
}
