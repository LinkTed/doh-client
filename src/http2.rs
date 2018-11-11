use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::thread::sleep;
use std::time::Duration;

use tokio_rustls::{Connect, TlsConnector, TlsStream};

use tokio::runtime::Runtime;
use tokio::net::TcpStream;
use tokio::net::tcp::ConnectFuture;

use rustls::{ClientSession, ClientConfig};

use webpki::DNSNameRef;

use futures::{Async, Future, Stream};
use futures::sync::mpsc::UnboundedSender;

use h2::client::{SendRequest, Handshake, ResponseFuture, Connection, handshake};
use h2::RecvStream;

use bytes::Bytes;

use dns::DnsPacket;


const ALPN_H2: &str = "h2";

pub fn create_config(cafile: &str) -> Result<ClientConfig, Error> {
    let certfile = File::open(&cafile)?;

    let mut config = ClientConfig::new();
    if let Err(()) = config.root_store.add_pem_file(&mut BufReader::new(certfile)) {
        return Err(Error::new(ErrorKind::Other, "Cannot parse pem file"));
    }
    config.alpn_protocols.push(ALPN_H2.to_owned());
    Ok(config)
}

pub fn connect_http2_server(runtime: &mut Runtime, remote_addr: SocketAddr, client_config: ClientConfig, domain: String, retries: u16) -> Result<(SendRequest<Bytes>), ()> {
    if retries == 0 {
        return Err(());
    }

    for x in 1..(retries + 1) {
        let result = runtime.block_on(Http2ConnectionFuture::new(remote_addr, client_config.clone(), domain.clone()));
        match result {
            Ok((send_request, connection)) => {
                runtime.spawn(connection.map_err(|e| error!("H2 connection error: {}", e)));
                return Ok(send_request);
            },
            Err(e) => {
                error!("Connection to remote server {} ({}) failed: {}: retry: {}", remote_addr, domain, e, x);
            }
        }

        sleep(Duration::from_secs(2));
    }

    return Err(());
}

enum Http2ConnectionState {
    GetTcpConnection(ConnectFuture),
    GetTlsConnection(Connect<TcpStream>),
    GetHttp2Connection(Handshake<TlsStream<TcpStream, ClientSession>, Bytes>),
}

pub struct Http2ConnectionFuture {
    state: Http2ConnectionState,
    tls_connector: TlsConnector,
    domain: String,
}

impl Http2ConnectionFuture {
    pub fn new(remote_addr: SocketAddr, config: ClientConfig, domain: String) -> Http2ConnectionFuture {
        Http2ConnectionFuture{state: Http2ConnectionState::GetTcpConnection(TcpStream::connect(&remote_addr)), tls_connector: TlsConnector::from(Arc::new(config)), domain}
    }
}

impl Future for Http2ConnectionFuture {
    type Item = (SendRequest<Bytes>, Connection<TlsStream<TcpStream, ClientSession>>);
    type Error = Error;

    fn poll(&mut self) -> Result<Async<(SendRequest<Bytes>, Connection<TlsStream<TcpStream, ClientSession>>)>, Error> {
        use self::Http2ConnectionState::*;
        loop {
            self.state = match self.state {
                GetTcpConnection(ref mut future) => {
                    match future.poll() {
                        Ok(async) => {
                            match async {
                                Async::Ready(tcp) => {
                                    if let Err(e) = tcp.set_keepalive(Some(Duration::from_secs(1))) {
                                        error!("Could not set keepalive on TCP: {}", e);
                                    }
                                    GetTlsConnection(self.tls_connector.connect(DNSNameRef::try_from_ascii_str(&self.domain).unwrap(), tcp))
                                },
                                Async::NotReady => return Ok(Async::NotReady),
                            }
                        },
                        Err(e) => {
                            return Err(e);
                        }
                    }
                },
                GetTlsConnection(ref mut connect) => {
                    match connect.poll() {
                        Ok(async) => {
                            match async {
                                Async::Ready(tls) => {
                                    GetHttp2Connection(handshake(tls))
                                }
                                Async::NotReady => return Ok(Async::NotReady),
                            }
                        },
                        Err(e) => {
                            return Err(e);
                        }
                    }
                },
                GetHttp2Connection(ref mut handshake) => {
                    match handshake.poll() {
                        Ok(async) => {
                            match async {
                                Async::Ready(http2) => {
                                    return Ok(Async::Ready(http2))
                                }
                                Async::NotReady => return Ok(Async::NotReady),
                            }
                        },
                        Err(e) => {
                            return Err(Error::new(ErrorKind::Other, e));
                        }
                    }
                }
            }
        }
    }
}

enum Http2ResponseState {
    GetResponse(ResponseFuture),
    GetBody(RecvStream),
}

pub struct Http2ResponseFuture {
    state: Http2ResponseState,
    sender: Arc<Mutex<UnboundedSender<(DnsPacket, SocketAddr)>>>,
    addr: SocketAddr,
    buffer: Bytes,
    tid: [u8;2],
}

impl Http2ResponseFuture {
    pub fn new(response_future: ResponseFuture, sender: Arc<Mutex<UnboundedSender<(DnsPacket, SocketAddr)>>>, addr: SocketAddr, tid: [u8;2]) -> Http2ResponseFuture {
        Http2ResponseFuture {state: Http2ResponseState::GetResponse(response_future) ,sender, addr, buffer: Bytes::new(), tid}
    }
}

impl Future for Http2ResponseFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<()>, ()> {
        use self::Http2ResponseState::*;

        loop {
            self.state = match self.state {
                GetResponse(ref mut future) => {
                    match future.poll() {
                        Ok(async) => {
                            match async {
                                Async::Ready(response) => {
                                    let (header, body) = response.into_parts();

                                    if header.status != 200 {
                                        error!("Http2ResponseFuture: GetResponse: header.status != 200");
                                        return Err(())
                                    }

                                    match header.headers.get("content-type") {
                                        Some(value) => {
                                            if value != "application/dns-message" {
                                                error!("Http2ResponseFuture: GetResponse: content-type != application/dns-message");
                                                return Err(())
                                            }
                                        }
                                        None => {
                                            error!("Http2ResponseFuture: GetResponse: content-type is None");
                                            return Err(())
                                        }
                                    }

                                    GetBody(body)
                                },
                                Async::NotReady => return Ok(Async::NotReady),
                            }
                        },
                        Err(e) => {
                            error!("Http2ResponseFuture: GetResponse: {}", e);
                            return Err(());
                        }
                    }
                }
                GetBody(ref mut stream) => {
                    loop {
                        match stream.poll() {
                            Ok(async) => {
                                match async {
                                    Async::Ready(mut body) => {
                                        if let Some(b) = body {
                                            let buffer_len = self.buffer.len();
                                            let b_len = b.len();

                                            if buffer_len < 1024 {
                                                if buffer_len + b_len < 1024 {
                                                    self.buffer.extend(b);
                                                } else {
                                                    self.buffer.extend(b.slice_to(1024 - buffer_len));
                                                }
                                            }

                                            match stream.release_capacity().release_capacity(b_len) {
                                                Ok(()) => {},
                                                Err(e) => error!("Http2ResponseFuture: GetBody: release_capacity: {}", e)

                                            }
                                        } else {
                                            let sender = self.sender.lock().unwrap();
                                            let dns = DnsPacket::from_tid(self.buffer.clone(), self.tid.clone());

                                            match sender.unbounded_send((dns, self.addr)) {
                                                Ok(()) => {},
                                                Err(e) => {
                                                    error!("Http2ResponseFuture: GetBody: unbounded_send: {}", e);
                                                    return Err(());
                                                }
                                            }
                                            return Ok(Async::Ready(()));
                                        }
                                    },
                                    Async::NotReady => return Ok(Async::NotReady),
                                }
                            },
                            Err(e) => {
                                error!("Http2ResponseFuture: GetBody: {}", e);
                                return Err(());
                            }
                        }
                    }
                }
            }
        }
    }
}