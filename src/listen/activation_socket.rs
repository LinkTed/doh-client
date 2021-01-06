#[cfg(target_os = "macos")]
use libc::size_t;
use std::io::Result as IoResult;
use std::net::UdpSocket;
#[cfg(target_os = "macos")]
use std::os::raw::{c_char, c_int, c_void};

#[cfg(target_os = "macos")]
extern "C" {
    fn launch_activate_socket(name: *const c_char, fds: *mut *mut c_int, cnt: *mut size_t)
        -> c_int;
}

#[cfg(target_os = "macos")]
pub(super) fn get_activation_socket() -> IoResult<UdpSocket> {
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
                let socket = UdpSocket::from_raw_fd(*fds.offset(0));
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
pub(super) fn get_activation_socket() -> IoResult<UdpSocket> {
    use std::os::unix::io::FromRawFd;
    unsafe { Ok(UdpSocket::from_raw_fd(3)) }
}

#[cfg(target_family = "windows")]
pub(super) fn get_activation_socket() -> IoResult<UdpSocket> {
    use std::io;
    use std::io::ErrorKind::Other;
    Err(io::Error::new(
        Other,
        "This is not supported in windows platforms",
    ))
}
