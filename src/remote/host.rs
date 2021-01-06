#[cfg(feature = "socks5")]
use super::socks5_connect;
use super::{http2_connect, tcp_connect, try_tls_connect};
use crate::DohResult;
use bytes::Bytes;
use h2::client::SendRequest;
use rustls::ClientConfig;
use std::fmt::{Display, Formatter, Result};
use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(feature = "socks5")]
use tokio_socks::TargetAddr;

pub enum Host {
    Direct(Vec<SocketAddr>),
    #[cfg(feature = "socks5")]
    Socks5(
        Vec<SocketAddr>,
        Option<(String, String)>,
        Vec<TargetAddr<'static>>,
    ),
}

impl Host {
    pub(super) async fn connect(
        &mut self,
        client_config: &Arc<ClientConfig>,
        domain: &str,
    ) -> DohResult<SendRequest<Bytes>> {
        match self {
            Host::Direct(remote_addrs) => {
                let tcp_connection = tcp_connect(remote_addrs).await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
            #[cfg(feature = "socks5")]
            Host::Socks5(remote_addrs, credentials, dest_addrs) => {
                let tcp_connection = socks5_connect(remote_addrs, dest_addrs, credentials).await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = http2_connect(tls_connection).await?;
                Ok(http2_connection)
            }
        }
    }

    pub(super) fn get_remote_addrs(&self) -> Vec<SocketAddr> {
        match self {
            Host::Direct(remote_addrs) => remote_addrs.clone(),
            #[cfg(feature = "socks5")]
            Host::Socks5(remote_addrs, _, _) => remote_addrs.clone(),
        }
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Host::Direct(remote_addrs) => write!(f, "{:?}", remote_addrs),
            #[cfg(feature = "socks5")]
            Host::Socks5(remote_addrs, _, dest_addrs) => {
                write!(f, "{:?} via socks5 {:?}", dest_addrs, remote_addrs)
            }
        }
    }
}
