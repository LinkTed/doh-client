use crate::DohResult;
#[cfg(feature = "http-proxy")]
use async_http_proxy::{http_connect_tokio, http_connect_tokio_with_basic_auth, HttpError};
use bytes::Bytes;
use h2::client::{handshake, SendRequest};
use rustls::ClientConfig;
use rustls::ServerName;
use std::convert::TryFrom;
use std::io::Result as IoResult;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::spawn;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
#[cfg(feature = "socks5")]
use tokio_socks::tcp::Socks5Stream;
#[cfg(feature = "socks5")]
use tokio_socks::TargetAddr;

pub(super) async fn try_http2_connect<T>(connection: T) -> DohResult<SendRequest<Bytes>>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    debug!("HTTP2 handshake");
    let (send_request, connection) = handshake(connection).await?;
    spawn(async move {
        if let Err(e) = connection.await {
            error!("HTTP2 connection close: {}", e);
        }
    });
    Ok(send_request)
}

pub(super) async fn try_tls_connect<T>(
    connection: T,
    config: &Arc<ClientConfig>,
    domain: &str,
) -> IoResult<TlsStream<T>>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let tls_connector = TlsConnector::from(config.clone());
    tls_connector
        .connect(ServerName::try_from(domain).unwrap(), connection)
        .await
}

pub(super) async fn try_tcp_connect(host: &str, port: u16) -> DohResult<TcpStream> {
    let tcp_connection = TcpStream::connect(&(host, port)).await?;
    tcp_connection.set_nodelay(true)?;
    Ok(tcp_connection)
}

#[cfg(feature = "socks5")]
async fn socks5_connect(
    host: &str,
    port: u16,
    dest_addr: &TargetAddr<'static>,
    credentials: &Option<(String, String)>,
) -> DohResult<TcpStream> {
    let dest_addr = dest_addr.to_owned();
    let socks5_connection = if let Some((username, password)) = credentials {
        Socks5Stream::connect_with_password((host, port), dest_addr, username, password).await
    } else {
        Socks5Stream::connect((host, port), dest_addr).await
    }?;
    let tcp_connection = socks5_connection.into_inner();
    tcp_connection.set_nodelay(true)?;
    Ok(tcp_connection)
}

#[cfg(feature = "socks5")]
pub(super) async fn try_socks5_connect(
    proxy_host: &str,
    proxy_port: u16,
    remote_addrs: &[std::net::SocketAddr],
    credentials: &Option<(String, String)>,
) -> DohResult<TcpStream> {
    for remote_addr in remote_addrs {
        let target_addr = TargetAddr::Ip(*remote_addr);
        let tcp_connection =
            socks5_connect(proxy_host, proxy_port, &target_addr, credentials).await;
        match tcp_connection {
            Ok(tcp_connection) => return Ok(tcp_connection),
            Err(e) => {
                error!(
                    "Could not connect to {:?} via socks5 proxy {}:{}  {}",
                    remote_addr, proxy_host, proxy_port, e
                );
            }
        }
    }
    Err(crate::DohError::CouldNotConnect(
        proxy_host.to_owned(),
        proxy_port,
    ))
}

#[cfg(feature = "socks5")]
pub(super) async fn try_socks5h_connect(
    proxy_host: &str,
    proxy_port: u16,
    remote_host: &str,
    remote_port: u16,
    credentials: &Option<(String, String)>,
) -> DohResult<TcpStream> {
    use std::borrow::Cow;

    let target_addr = TargetAddr::Domain(Cow::Owned(remote_host.to_owned()), remote_port);
    socks5_connect(proxy_host, proxy_port, &target_addr, credentials).await
}

#[cfg(feature = "http-proxy")]
pub(super) async fn try_http_proxy_connect<T>(
    connection: &mut T,
    remote_host: &str,
    remote_port: u16,
    credentials: &Option<(String, String)>,
) -> Result<(), HttpError>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    match credentials {
        Some((username, password)) => {
            http_connect_tokio_with_basic_auth(
                connection,
                remote_host,
                remote_port,
                username,
                password,
            )
            .await
        }
        None => http_connect_tokio(connection, remote_host, remote_port).await,
    }
}
