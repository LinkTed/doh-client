use super::activation_socket::get_activation_socket;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Result as IoResult;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[derive(Copy, Clone)]
pub enum Config {
    Addr(SocketAddr),
    Activation,
}

impl Config {
    pub(crate) async fn into_socket(self) -> IoResult<UdpSocket> {
        match self {
            Config::Addr(socket_addr) => UdpSocket::bind(&socket_addr).await,
            Config::Activation => {
                let socket = get_activation_socket()?;
                UdpSocket::from_std(socket)
            }
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Config::Addr(socket_addr) => write!(f, "{}", socket_addr),
            Config::Activation => {
                if cfg!(target_os = "macos") {
                    write!(f, "file descriptor of launch_activate_socket()")
                } else if cfg!(target_family = "unix") {
                    write!(f, "file descriptor 3")
                } else {
                    write!(f, "this is not supported on windows")
                }
            }
        }
    }
}
