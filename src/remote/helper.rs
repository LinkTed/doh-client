use crate::{DohError, DohResult};
use bytes::Bytes;
use h2::client::{handshake, SendRequest};
use rustls::ClientConfig;
use std::io::Result as IoResult;
use std::net::SocketAddr;
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
use webpki::DNSNameRef;

pub(super) async fn http2_connect<T>(connection: T) -> DohResult<SendRequest<Bytes>>
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
        .connect(DNSNameRef::try_from_ascii_str(domain).unwrap(), connection)
        .await
}

async fn try_tcp_connect(remote_addr: &SocketAddr) -> IoResult<TcpStream> {
    let tcp_connection = TcpStream::connect(remote_addr).await?;
    tcp_connection.set_nodelay(true)?;
    Ok(tcp_connection)
}

pub(super) async fn tcp_connect(remote_addrs: &[SocketAddr]) -> DohResult<TcpStream> {
    for remote_addr in remote_addrs {
        match try_tcp_connect(remote_addr).await {
            Ok(tcp_connection) => return Ok(tcp_connection),
            Err(e) => {
                error!("Could not connectio to {}: {}", remote_addr, e);
            }
        };
    }
    Err(DohError::CouldNotConnect(Vec::from(remote_addrs)))
}
#[cfg(feature = "socks5")]
async fn try_socks5_connect(
    remote_addr: &SocketAddr,
    dest_addr: &TargetAddr<'static>,
    credentials: &Option<(String, String)>,
) -> DohResult<TcpStream> {
    let dest_addr = dest_addr.to_owned();
    let socks5_connection = if let Some((username, password)) = credentials {
        Socks5Stream::connect_with_password(remote_addr, dest_addr, username, password).await
    } else {
        Socks5Stream::connect(remote_addr, dest_addr).await
    }?;
    let tcp_connection = socks5_connection.into_inner();
    tcp_connection.set_nodelay(true)?;
    Ok(tcp_connection)
}
#[cfg(feature = "socks5")]
pub(super) async fn socks5_connect(
    remote_addrs: &[SocketAddr],
    dest_addrs: &[TargetAddr<'static>],
    credentials: &Option<(String, String)>,
) -> DohResult<TcpStream> {
    for remote_addr in remote_addrs {
        for dest_addr in dest_addrs {
            let tcp_connection = try_socks5_connect(remote_addr, dest_addr, credentials).await;
            match tcp_connection {
                Ok(tcp_connection) => return Ok(tcp_connection),
                Err(e) => {
                    error!(
                        "Could not connect to {:?} via socks5 proxy {}: {}",
                        dest_addr, remote_addr, e
                    );
                }
            }
        }
    }
    Err(DohError::CouldNotConnect(Vec::from(remote_addrs)))
}
