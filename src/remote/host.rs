use super::Connection;
use std::fmt::{Display, Formatter, Result};
use std::net::SocketAddr;
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
    pub(super) fn into_connection(self) -> Connection {
        match self {
            Host::Direct(remote_addrs) => Connection::direct(remote_addrs),
            #[cfg(feature = "socks5")]
            Host::Socks5(remote_addrs, credentials, dest_addrs) => {
                Connection::socks5(remote_addrs, credentials, dest_addrs)
            }
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
