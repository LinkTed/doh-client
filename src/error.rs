use bytes::Bytes;

use dns_message_parser::{DecodeError, Dns, EncodeError};

use futures::channel::mpsc::TrySendError;

use h2::Error as H2Error;

use http::{HeaderValue, StatusCode};

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IoError;
use std::net::SocketAddr;
#[cfg(feature = "socks5")]
use tokio_socks::Error as SocksError;

pub enum Error {
    Io(IoError),
    H2(H2Error),
    Decode(DecodeError),
    Encode(EncodeError),
    TrySend(TrySendError<(Bytes, SocketAddr)>),
    #[cfg(feature = "socks5")]
    Socks(SocksError),
    IsNotConnected,
    PEMParser,
    CacheSize,
    CouldNotConnect(Vec<SocketAddr>),
    CouldNotGetResponse(Dns),
    HeaderStatus(StatusCode),
    HeaderContentType(HeaderValue),
    HeaderNoContentType,
    DnsNotRequest(Dns),
    DnsNotResponse(Dns),
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

impl From<H2Error> for Error {
    fn from(e: H2Error) -> Self {
        Error::H2(e)
    }
}

impl From<DecodeError> for Error {
    fn from(e: DecodeError) -> Self {
        Error::Decode(e)
    }
}

impl From<EncodeError> for Error {
    fn from(e: EncodeError) -> Self {
        Error::Encode(e)
    }
}

impl From<TrySendError<(Bytes, SocketAddr)>> for Error {
    fn from(e: TrySendError<(Bytes, SocketAddr)>) -> Self {
        Error::TrySend(e)
    }
}
#[cfg(feature = "socks5")]
impl From<SocksError> for Error {
    fn from(e: SocksError) -> Self {
        match e {
            SocksError::Io(e) => Error::Io(e),
            e => Error::Socks(e),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Error::Io(e) => write!(f, "IO Error: {}", e),
            Error::H2(e) => write!(f, "H2 Error: {}", e),
            Error::Decode(e) => write!(f, "Decode Error: {:?}", e),
            Error::Encode(e) => write!(f, "Encode Error: {:?}", e),
            Error::TrySend(e) => write!(f, "Could not send to the response handler: {}", e),
            #[cfg(feature = "socks5")]
            Error::Socks(e) => write!(f, "Socks Error: {}", e),
            Error::IsNotConnected => write!(f, "doh-client is not connected"),
            Error::PEMParser => write!(f, "Cannot parse pem file"),
            Error::CacheSize => write!(
                f,
                "Cache size is zero and cache fallback is enabled simultaneously"
            ),
            Error::CouldNotConnect(remote_addrs) => {
                write!(f, "Could not connect to any address: {:?}", remote_addrs)
            }
            Error::CouldNotGetResponse(dns_request) => {
                write!(f, "Could not get response for: {:?}", dns_request)
            }
            Error::HeaderStatus(status) => write!(f, "Header status: got {}", status),
            Error::HeaderContentType(content_type) => write!(
                f,
                "Header content type: got {:?} expected application/dns-message",
                content_type
            ),
            Error::HeaderNoContentType => write!(f, "Header content type is missing"),
            Error::DnsNotRequest(dns) => write!(f, "DNS packet is not a request: {:?}", dns),
            Error::DnsNotResponse(dns) => write!(f, "DNS packet is not a response: {:?}", dns),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
