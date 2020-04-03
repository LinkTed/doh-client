use std::fmt::{Display, Formatter};
use std::io;
use std::net;
use std::net::SocketAddr;

use futures::stream::{SplitSink, SplitStream, StreamExt};

use tokio::net::UdpSocket;

use tokio_util::codec::{Decoder, Encoder};
use tokio_util::udp::UdpFramed;

use bytes::{Buf, Bytes, BytesMut};

pub static MAXIMUM_DNS_PACKET_SIZE: usize = 4096;

#[derive(Debug)]
pub struct DnsCodec;

pub enum DnsParserError {
    TooLittleData,
    TooMuchData,
}

impl Display for DnsParserError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use self::DnsParserError::*;
        match self {
            TooLittleData => write!(f, "TooLittleData"),
            TooMuchData => write!(f, "TooMuchData"),
        }
    }
}

#[derive(Copy, Clone)]
pub enum UdpListenSocket {
    Addr(SocketAddr),
    Activation,
}

impl Display for UdpListenSocket {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use self::UdpListenSocket::*;
        match self {
            Addr(socket_addr) => write!(f, "{}", socket_addr),
            Activation => {
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

#[derive(Clone)]
pub struct DnsPacket {
    data: Bytes,
    tid: [u8; 2],
    response: bool,
    questions: u16,
    answer: u16,
    authority: u16,
    additional_records: u16,
}

impl DnsPacket {
    pub fn from(buffer: Bytes) -> Result<DnsPacket, DnsParserError> {
        DnsPacket::parser(buffer)
    }

    pub fn from_tid(buffer: Bytes, tid: [u8; 2]) -> Result<DnsPacket, DnsParserError> {
        let mut buffer = BytesMut::from(buffer.bytes());
        buffer[0] = tid[0];
        buffer[1] = tid[1];

        DnsPacket::parser(buffer.freeze())
    }

    fn parser(buffer: Bytes) -> Result<DnsPacket, DnsParserError> {
        use self::DnsParserError::{TooLittleData, TooMuchData};
        let len = buffer.len();

        if len < 12 {
            return Err(TooLittleData);
        } else if MAXIMUM_DNS_PACKET_SIZE < len {
            return Err(TooMuchData);
        }

        let response = (buffer[2] & 0x80) == 0x80;

        let mut tid: [u8; 2] = [0; 2];
        tid.copy_from_slice(&buffer[0..2]);

        let questions: u16 = ((buffer[4] as u16) << 8) | (buffer[5] as u16);
        let answer: u16 = ((buffer[6] as u16) << 8) | (buffer[7] as u16);
        let authority: u16 = ((buffer[8] as u16) << 8) | (buffer[9] as u16);
        let additional_records: u16 = ((buffer[10] as u16) << 8) | (buffer[11] as u16);

        Ok(DnsPacket {
            data: buffer,
            tid,
            response,
            questions,
            answer,
            authority,
            additional_records,
        })
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_without_tid(&self) -> Bytes {
        let mut data: BytesMut = BytesMut::with_capacity(self.data.len());
        data.extend(self.data.iter());
        data[0] = b'\0';
        data[1] = b'\0';

        data.freeze()
    }

    pub fn get(&self) -> Bytes {
        self.data.clone()
    }

    pub fn get_tid(&self) -> [u8; 2] {
        self.tid.clone()
    }

    pub fn is_response(&self) -> bool {
        self.response
    }

    pub fn get_questions(&self) -> u16 {
        self.questions
    }
}

#[cfg(target_os = "macos")]
use libc::size_t;
#[cfg(target_os = "macos")]
use std::os::raw::{c_char, c_int, c_void};

#[cfg(target_os = "macos")]
extern "C" {
    fn launch_activate_socket(name: *const c_char, fds: *mut *mut c_int, cnt: *mut size_t)
        -> c_int;
}

#[cfg(target_os = "macos")]
fn get_activation_socket() -> io::Result<net::UdpSocket> {
    use libc::free;
    use std::ffi::CString;
    use std::io::ErrorKind::Other;
    use std::os::unix::io::FromRawFd;
    use std::ptr::null_mut;
    unsafe {
        let mut fds: *mut c_int = null_mut();
        let mut cnt: size_t = 0;

        let name = CString::new("Listeners").expect("CString::new failed");
        if launch_activate_socket(name.as_ptr(), &mut fds, &mut cnt) == 0 {
            if cnt == 1 {
                let socket = net::UdpSocket::from_raw_fd(*fds.offset(0));
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

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn get_activation_socket() -> io::Result<net::UdpSocket> {
    use std::os::unix::io::FromRawFd;
    unsafe { Ok(net::UdpSocket::from_raw_fd(3)) }
}

#[cfg(target_family = "windows")]
fn get_activation_socket() -> io::Result<net::UdpSocket> {
    use std::io::ErrorKind::Other;
    Err(io::Error::new(
        Other,
        "This is not supported in windows platforms",
    ))
}

impl DnsCodec {
    pub async fn new(
        listen: UdpListenSocket,
    ) -> io::Result<(
        SplitSink<UdpFramed<DnsCodec>, (DnsPacket, SocketAddr)>,
        SplitStream<UdpFramed<DnsCodec>>,
    )> {
        let socket = match listen {
            UdpListenSocket::Addr(socket_addr) => UdpSocket::bind(&socket_addr).await?,
            UdpListenSocket::Activation => {
                let socket = get_activation_socket()?;
                UdpSocket::from_std(socket)?
            }
        };
        Ok(UdpFramed::new(socket, DnsCodec).split())
    }
}

impl Decoder for DnsCodec {
    type Item = DnsPacket;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<DnsPacket>, io::Error> {
        if let Ok(dns) = DnsPacket::from(buf.clone().freeze()) {
            if dns.is_response() == false && dns.get_questions() > 0 {
                return Ok(Some(dns));
            }
        }

        buf.clear();
        Ok(None)
    }
}

impl Encoder<DnsPacket> for DnsCodec {
    type Error = io::Error;

    fn encode(&mut self, data: DnsPacket, buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.clear();
        buf.reserve(data.len());
        let b: Bytes = data.get();
        buf.extend(b);
        Ok(())
    }
}
