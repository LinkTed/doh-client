use crate::cmd::RemoteHostError;
#[cfg(feature = "http-proxy")]
use async_http_proxy::HttpError as HttpProxyError;
use bytes::Bytes;
use dns_message_parser::{DecodeError, Dns, EncodeError};
use futures::channel::mpsc::TrySendError;
use h2::Error as H2Error;
use http::{HeaderValue, StatusCode};
use std::io::Error as IoError;
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error as ThisError;
#[cfg(feature = "socks5")]
use tokio_socks::Error as SocksError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(#[from] IoError),
    #[error("H2 Error: {0}")]
    H2Error(#[from] H2Error),
    #[error("Decode Error: {0:?}")]
    DecodeError(DecodeError),
    #[error("Encode Error: {0:?}")]
    EncodeError(EncodeError),
    #[error("Could not send to the response handler: {0}")]
    TrySendError(#[from] TrySendError<(Bytes, SocketAddr)>),
    #[cfg(feature = "http-proxy")]
    #[error("HTTP Proxy Error: {0}")]
    HttpProxyError(#[from] HttpProxyError),
    #[cfg(feature = "socks5")]
    #[error("Socks Error: {0}")]
    SocksError(#[from] SocksError),
    #[error("doh-client is not connected")]
    IsNotConnected,
    #[error("Cannot parse pem file")]
    PEMParser,
    #[error("Cache size is zero and cache fallback is enabled simultaneously")]
    CacheSize,
    #[error("Could not connect to DoH server")]
    CouldNotConnectServer,
    #[error("Could not connect to address: {0}:{1}")]
    CouldNotConnect(String, u16),
    #[error("Could not get response for: {0:?}")]
    CouldNotGetResponse(Dns),
    #[error("Header status: got {0}")]
    HeaderStatus(StatusCode),
    #[error("Header content type: got {0:?} expected application/dns-message")]
    HeaderContentType(HeaderValue),
    #[error("Header content type is missing")]
    HeaderNoContentType,
    #[error("DNS packet is not a request: {0:?}")]
    DnsNotRequest(Dns),
    #[error("DNS packet is not a response: {0:?}")]
    DnsNotResponse(Dns),
    #[error("Could not get listen config: {0}")]
    AddrParseError(#[from] AddrParseError),
    #[error("Remote Error: {0}")]
    RemoteHostError(#[from] RemoteHostError),
}

impl From<DecodeError> for Error {
    fn from(decode_error: DecodeError) -> Self {
        Error::DecodeError(decode_error)
    }
}

impl From<EncodeError> for Error {
    fn from(encode_error: EncodeError) -> Self {
        Error::EncodeError(encode_error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
