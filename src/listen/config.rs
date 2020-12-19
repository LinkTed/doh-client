#[cfg(target_os = "macos")]
use libc::size_t;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Result as IoResult;
use std::net::SocketAddr;
use std::net::UdpSocket as StdUdpSocket;
#[cfg(target_os = "macos")]
use std::os::raw::{c_char, c_int, c_void};
use tokio::net::UdpSocket as TokioUdpSocket;

#[cfg(target_os = "macos")]
extern "C" {
    fn launch_activate_socket(name: *const c_char, fds: *mut *mut c_int, cnt: *mut size_t)
        -> c_int;
}

#[cfg(target_os = "macos")]
fn get_activation_socket() -> IoResult<StdUdpSocket> {
    use libc::free;
    use std::ffi::CString;
    use std::io;
    use std::io::ErrorKind::Other;
    use std::os::unix::io::FromRawFd;
    use std::ptr::null_mut;
    unsafe {
        let mut fds: *mut c_int = null_mut();
        let mut cnt: size_t = 0;

        let name = CString::new("Listeners").expect("CString::new failed");
        if launch_activate_socket(name.as_ptr(), &mut fds, &mut cnt) == 0 {
            if cnt == 1 {
                let socket = StdUdpSocket::from_raw_fd(*fds.offset(0));
                free(fds as *mut c_void);
                Ok(socket)
            } else {
                Err(io::Error::new(Other, "Could not get fd: cnt != 1"))
            }
        } else {
            Err(io::Error::new(
                Other,
                "Could not get fd: launch_activate_socket != 0",
            ))
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn get_activation_socket() -> IoResult<StdUdpSocket> {
    use std::os::unix::io::FromRawFd;
    unsafe { Ok(StdUdpSocket::from_raw_fd(3)) }
}

#[cfg(target_family = "windows")]
fn get_activation_socket() -> IoResult<StdUdpSocket> {
    use std::io;
    use std::io::ErrorKind::Other;
    Err(io::Error::new(
        Other,
        "This is not supported in windows platforms",
    ))
}

#[derive(Copy, Clone)]
pub enum Config {
    Addr(SocketAddr),
    Activation,
}

impl Config {
    pub(crate) async fn into_socket(self) -> IoResult<TokioUdpSocket> {
        let socket = match self {
            Config::Addr(socket_addr) => TokioUdpSocket::bind(&socket_addr).await?,
            Config::Activation => {
                let socket = get_activation_socket()?;
                TokioUdpSocket::from_std(socket)?
            }
        };
        Ok(socket)
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
                    write!(f, "this is not supported")
                }
            }
        }
    }
}
