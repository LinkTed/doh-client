#[cfg(feature = "socks5")]
use super::socks5_connect;
use super::{http2_connect, tcp_connect, try_tls_connect};
use crate::{DohError, DohResult};
use bytes::Bytes;
use h2::client::{ResponseFuture, SendRequest};
use h2::SendStream;
use http::Request;
use rustls::ClientConfig;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(feature = "socks5")]
use tokio_socks::TargetAddr;

macro_rules! send_request_option {
    ($self:ident) => {
        match $self {
            Connection::Direct(_, send_request) => send_request,
            #[cfg(feature = "socks5")]
            Connection::Socks5(_, _, _, send_request) => send_request,
        }
    };
}

pub(super) enum Connection {
    Direct(Vec<SocketAddr>, Option<SendRequest<Bytes>>),
    #[cfg(feature = "socks5")]
    Socks5(
        Vec<SocketAddr>,
        Option<(String, String)>,
        Vec<TargetAddr<'static>>,
        Option<SendRequest<Bytes>>,
    ),
}

impl Connection {
    pub(super) fn direct(remote_addrs: Vec<SocketAddr>) -> Connection {
        Connection::Direct(remote_addrs, None)
    }

    #[cfg(feature = "socks5")]
    pub(super) fn socks5(
        remote_addrs: Vec<SocketAddr>,
        credentials: Option<(String, String)>,
        dest_addr: Vec<TargetAddr<'static>>,
    ) -> Connection {
        Connection::Socks5(remote_addrs, credentials, dest_addr, None)
    }

    pub(super) fn get_remote_addrs(&self) -> Vec<SocketAddr> {
        match self {
            Connection::Direct(remote_addrs, _) => remote_addrs.clone(),
            #[cfg(feature = "socks5")]
            Connection::Socks5(remote_addrs, _, _, _) => remote_addrs.clone(),
        }
    }

    pub(super) fn is_connected(&self) -> bool {
        match self {
            Connection::Direct(_, send_request) => send_request.is_some(),
            #[cfg(feature = "socks5")]
            Connection::Socks5(_, _, _, send_request) => send_request.is_some(),
        }
    }

    pub(super) async fn connect(
        &mut self,
        client_config: &Arc<ClientConfig>,
        domain: &str,
    ) -> DohResult<()> {
        match self {
            Connection::Direct(remote_addrs, send_request) => {
                if send_request.is_some() {
                    return Ok(());
                }

                let tcp_connection = tcp_connect(remote_addrs).await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = http2_connect(tls_connection).await?;
                send_request.replace(http2_connection);
            }
            #[cfg(feature = "socks5")]
            Connection::Socks5(remote_addrs, credentials, dest_addrs, send_request) => {
                if send_request.is_some() {
                    return Ok(());
                }

                let tcp_connection = socks5_connect(remote_addrs, dest_addrs, credentials).await?;
                let tls_connection = try_tls_connect(tcp_connection, client_config, domain).await?;
                let http2_connection = http2_connect(tls_connection).await?;
                send_request.replace(http2_connection);
            }
        }
        Ok(())
    }

    pub(super) fn disconnect(&mut self) {
        let send_request = send_request_option!(self);
        send_request.take();
    }

    pub(super) async fn send_request(
        &mut self,
        request: Request<()>,
    ) -> DohResult<(ResponseFuture, SendStream<Bytes>)> {
        let send_request = send_request_option!(self);
        if let Some(send_request) = send_request {
            Ok(send_request.send_request(request, false)?)
        } else {
            Err(DohError::IsNotConnected)
        }
    }
}

impl Display for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Connection::Direct(remote_addrs, _) => write!(f, "{:?}", remote_addrs),
            #[cfg(feature = "socks5")]
            Connection::Socks5(remote_addrs, _, dest_addrs, _) => {
                write!(f, "{:?} via socks {:?}", dest_addrs, remote_addrs)
            }
        }
    }
}
