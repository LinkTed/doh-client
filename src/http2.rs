use std::sync::Arc;
use std::net::SocketAddr;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::thread::sleep;
use std::time::Duration;

use data_encoding::BASE64URL_NOPAD;

use tokio_rustls::{Connect, TlsConnector, TlsStream};

use tokio::timer::Timeout;
use tokio::net::TcpStream;
use tokio::net::tcp::ConnectFuture;
use tokio::prelude::FutureExt;

use rustls::{ClientSession, ClientConfig};

use webpki::DNSNameRef;

use futures_locks::{Mutex, MutexFut, MutexGuard};

use futures::{Async, Future, Stream};

use h2::client::{SendRequest, Handshake, ResponseFuture, Connection, handshake};
use h2::RecvStream;

use http::Request;

use bytes::Bytes;

use ttl_cache::TtlCache;

use dns::DnsPacket;

use ::Context;

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


enum Http2RequestState {
    GetMutexTtlCache(MutexFut<TtlCache<Bytes, Bytes>>),
    GetMutexSendRequest(MutexFut<(Option<SendRequest<Bytes>>, u32)>),
    GetConnection(MutexGuard<(Option<SendRequest<Bytes>>, u32)>, Http2ConnectionFuture, u32),
    GetResponse(Timeout<Http2ResponseFuture>, u32),
    CloseConnection(MutexFut<(Option<SendRequest<Bytes>>, u32)>, u32),
    SaveInCache(MutexFut<TtlCache<Bytes, Bytes>>, Bytes, Duration),
}

pub struct Http2RequestFuture {
    mutex_send_request: Mutex<(Option<SendRequest<Bytes>>, u32)>,
    mutex_ttl_cache: Mutex<TtlCache<Bytes, Bytes>>,
    state: Http2RequestState,
    context: Arc<Context>,
    msg: DnsPacket,
    addr: SocketAddr,
}

impl Http2RequestFuture {
    pub fn new(mutex_send_request: Mutex<(Option<SendRequest<Bytes>>, u32)>, mutex_ttl_cache: Mutex<TtlCache<Bytes, Bytes>>, msg: DnsPacket, addr: SocketAddr, context: Arc<Context>) -> Http2RequestFuture {
        use self::Http2RequestState::{GetMutexTtlCache, GetMutexSendRequest};
        debug!("Received UDP packet from {} {:#?}", addr, msg.get_tid());

        let state = if context.config.cache {
            GetMutexTtlCache(mutex_ttl_cache.lock())
        } else {
            GetMutexSendRequest(mutex_send_request.lock())
        };

        Http2RequestFuture { mutex_send_request, mutex_ttl_cache, state, msg, addr, context }
    }
}

macro_rules! send_request {
    ($a:ident, $b:ident) => {
        {
            let config = &$a.context.config;
            let post = config.post;
            let msg = &$a.msg;

            let request = if post {
                Request::builder()
                    .method("POST")
                    .uri(config.uri.clone())
                    .header("accept", "application/dns-message")
                    .header("content-type", "application/dns-message")
                    .header("content-length", msg.len().to_string())
                    .body(())
                    .unwrap()
            } else {
                Request::builder()
                    .method("GET")
                    .uri(format!("{}?dns={}", config.uri, BASE64URL_NOPAD.encode(&msg.get_without_tid())))
                    .header("accept", "application/dns-message")
                    .body(())
                    .unwrap()
            };

            let id = (*$b).1;

            match (*$b).0 {
                Some(ref mut send_request) => {
                    match send_request.send_request(request, false) {
                        Ok((response, mut request)) => {
                            if post {
                                match request.send_data(msg.get_without_tid(), true) {
                                    Ok(()) => GetResponse(Http2ResponseFuture::new(response).timeout(Duration::from_secs(config.timeout)), id),
                                    Err(e) => {
                                        error!("send_data: {}", e);
                                        CloseConnection($a.mutex_send_request.lock(), id)
                                    }
                                }
                            } else {
                                GetResponse(Http2ResponseFuture::new(response).timeout(Duration::from_secs(config.timeout)), id)
                            }
                        },
                        Err(e) => {
                            error!("send_request: {}", e);
                            CloseConnection($a.mutex_send_request.lock(), id)
                        }
                    }
                },
                None => return Err(())
            }
        }
    }
}

impl Future for Http2RequestFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<()>, ()> {
        use self::Http2RequestState::*;
        use self::Async::*;
        loop {
            self.state = match self.state {
                GetMutexTtlCache(ref mut mutex_fut) => {
                    match mutex_fut.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(mut guard) => {
                                    match (*guard).get(&self.msg.get_without_tid()) {
                                        Some(buffer) => {
                                            debug!("GetMutexTtlCache: found in cache");
                                            match DnsPacket::from_tid(buffer.clone(), self.msg.get_tid()) {
                                                Ok(dns) => {
                                                    match self.context.sender.unbounded_send((dns, self.addr)) {
                                                        Ok(()) => return Ok(Ready(())),
                                                        Err(e) => {
                                                            error!("GetMutexTtlCache: unbounded_send: {}", e);
                                                            return Err(());
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    debug!("GetMutexTtlCache: parse error: {}", e);
                                                    Http2RequestState::GetMutexSendRequest(self.mutex_send_request.lock())
                                                }
                                            }
                                        }
                                        None => {
                                            debug!("GetMutexTtlCache: missing in cache");
                                            Http2RequestState::GetMutexSendRequest(self.mutex_send_request.lock())
                                        }
                                    }
                                }
                                NotReady => return Ok(NotReady)
                            }
                        }
                        Err(_e) => {
                            error!("GetMutexTtlCache: could not get mutex");
                            Http2RequestState::GetMutexSendRequest(self.mutex_send_request.lock())
                        }
                    }
                }
                GetMutexSendRequest(ref mut mutex_fut) => {
                    let config = &self.context.config;

                    match mutex_fut.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(mut guard) => {
                                    if (*guard).0.is_some() {
                                        send_request!(self, guard)
                                    } else {
                                        GetConnection(guard, Http2ConnectionFuture::new(config.remote_addr, config.client_config.clone(), config.domain.clone()), 1)
                                    }
                                }
                                NotReady => return Ok(NotReady)
                            }
                        }
                        Err(_e) => {
                            error!("GetMutexSendRequest: could not get mutex");
                            return Err(());
                        }
                    }
                }
                GetConnection(ref mut guard, ref mut http2_connection_future, ref mut tries) => {
                    let config = &self.context.config;

                    match http2_connection_future.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready((mut send_request, connection)) => {
                                    tokio::spawn(connection.map_err(|e| {
                                        error!("GetConnection: H2 connection error: {}", e)
                                    }));

                                    info!("GetConnection: connection was successfully established to remote server {} ({})", config.remote_addr, config.domain);

                                    (*guard).0.replace(send_request);
                                    (*guard).1 += 1;

                                    send_request!(self, guard)
                                }
                                NotReady => return Ok(NotReady)
                            }
                        }
                        Err(e) => {
                            error!("GetConnection: connection to remote server {} ({}) failed: {}: retry: {}", config.remote_addr, config.domain, e, *tries);
                            sleep(Duration::from_secs(1));

                            if config.retries > *tries {
                                *tries += 1;
                                *http2_connection_future = Http2ConnectionFuture::new(config.remote_addr, config.client_config.clone(), config.domain.clone());
                                continue;
                            } else {
                                error!("GetConnection: too many connection attempts to remote server {} ({})", config.remote_addr, config.domain);
                                return Err(());
                            }
                        }
                    }
                }
                GetResponse(ref mut http2_response_future, ref id) => {
                    match http2_response_future.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(result) => {
                                    let (buffer, duration) = result;
                                    match DnsPacket::from_tid(buffer, self.msg.get_tid()) {
                                        Ok(dns) => {
                                            if dns.is_response() {
                                                let context = &self.context;

                                                match context.sender.unbounded_send((dns.clone(), self.addr)) {
                                                    Ok(()) => {
                                                        if context.config.cache {
                                                            if let Some(duration) = duration {
                                                                SaveInCache(self.mutex_ttl_cache.lock(), dns.get_without_tid(), duration)
                                                            } else {
                                                                return Ok(Ready(()));
                                                            }
                                                        } else {
                                                            return Ok(Ready(()));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error!("GetResponse: unbounded_send: {}", e);
                                                        return Err(());
                                                    }
                                                }
                                            } else {
                                                error!("GetResponse: get a non DNS response");
                                                return Err(());
                                            }
                                        }
                                        Err(e) => {
                                            error!("GetResponse: DNS parser error: {}", e);
                                            return Err(());
                                        }
                                    }
                                }
                                NotReady => return Ok(NotReady)
                            }
                        }
                        Err(_e) => {
                            error!("GetResponse: timeout");
                            CloseConnection(self.mutex_send_request.lock(), *id)
                        }
                    }
                }
                CloseConnection(ref mut mutex_fut, ref id) => {
                    match mutex_fut.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(mut guard) => {
                                    if (*guard).1 == *id {
                                        (*guard).0.take();
                                    }

                                    return Err(());
                                }
                                NotReady => return Ok(NotReady)
                            }
                        }
                        Err(_e) => {
                            error!("CloseConnection: could not get mutex");
                            return Err(());
                        }
                    }
                }
                SaveInCache(ref mut mutex_fut, ref buffer, ref duration) => {
                    match mutex_fut.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(mut guard) => {
                                    (*guard).insert(self.msg.get_without_tid(), buffer.clone(), duration.clone());
                                    return Ok(Ready(()));
                                }
                                NotReady => return Ok(NotReady)
                            }
                        }
                        Err(_e) => {
                            error!("SaveInCache: could not get mutex");
                            return Err(());
                        }
                    }
                }
            }
        }
    }
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
        Http2ConnectionFuture { state: Http2ConnectionState::GetTcpConnection(TcpStream::connect(&remote_addr)), tls_connector: TlsConnector::from(Arc::new(config)), domain }
    }
}

impl Future for Http2ConnectionFuture {
    type Item = (SendRequest<Bytes>, Connection<TlsStream<TcpStream, ClientSession>>);
    type Error = Error;

    fn poll(&mut self) -> Result<Async<(SendRequest<Bytes>, Connection<TlsStream<TcpStream, ClientSession>>)>, Error> {
        use self::Http2ConnectionState::*;
        use self::Async::*;
        loop {
            self.state = match self.state {
                GetTcpConnection(ref mut future) => {
                    match future.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(tcp) => {
                                    if let Err(e) = tcp.set_keepalive(Some(Duration::from_secs(1))) {
                                        error!("GetTcpConnection: could not set keepalive on TCP: {}", e);
                                    }

                                    if let Err(e) = tcp.set_nodelay(true) {
                                        error!("GetTcpConnection: could not set nodelay on TCP: {}", e);
                                    }

                                    GetTlsConnection(self.tls_connector.connect(DNSNameRef::try_from_ascii_str(&self.domain).unwrap(), tcp))
                                }
                                NotReady => return Ok(NotReady),
                            }
                        }
                        Err(e) => return Err(e)
                    }
                }
                GetTlsConnection(ref mut connect) => {
                    match connect.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(tls) => GetHttp2Connection(handshake(tls)),
                                NotReady => return Ok(NotReady),
                            }
                        }
                        Err(e) => return Err(e)
                    }
                }
                GetHttp2Connection(ref mut handshake) => {
                    match handshake.poll() {
                        Ok(async_) => return Ok(async_),
                        Err(e) => return Err(Error::new(ErrorKind::Other, e))
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
    buffer: Bytes,
    duration: Option<Duration>,
}

impl Http2ResponseFuture {
    pub fn new(response_future: ResponseFuture) -> Http2ResponseFuture {
        Http2ResponseFuture { state: Http2ResponseState::GetResponse(response_future), buffer: Bytes::new(), duration: None }
    }
}

impl Future for Http2ResponseFuture {
    type Item = (Bytes, Option<Duration>);
    type Error = ();

    fn poll(&mut self) -> Result<Async<(Bytes, Option<Duration>)>, ()> {
        use self::Http2ResponseState::*;
        use self::Async::*;
        loop {
            self.state = match self.state {
                GetResponse(ref mut future) => {
                    match future.poll() {
                        Ok(async_) => {
                            match async_ {
                                Ready(response) => {
                                    let (header, body) = response.into_parts();

                                    if header.status != 200 {
                                        error!("GetResponse: header.status != 200");
                                        return Err(());
                                    }

                                    let headers = &header.headers;

                                    match headers.get("content-type") {
                                        Some(value) => {
                                            if value != "application/dns-message" {
                                                error!("GetResponse: content-type != application/dns-message");
                                                return Err(());
                                            }
                                        }
                                        None => {
                                            error!("GetResponse: content-type is None");
                                            return Err(());
                                        }
                                    }

                                    if let Some(value) = headers.get("cache-control") {
                                        for i in value.to_str().unwrap().split(",") {
                                            let key_value: Vec<&str> = i.splitn(2, "=").map(|s| s.trim()).collect();
                                            if key_value.len() == 2 && key_value[0] == "max-age" {
                                                if let Ok(value) = key_value[1].parse::<u64>() {
                                                    self.duration = Some(Duration::from_secs(value));
                                                }
                                            }
                                        }
                                    }

                                    GetBody(body)
                                }
                                NotReady => return Ok(NotReady),
                            }
                        }
                        Err(e) => {
                            error!("GetResponse: {}", e);
                            return Err(());
                        }
                    }
                }
                GetBody(ref mut stream) => {
                    loop {
                        match stream.poll() {
                            Ok(async_) => {
                                match async_ {
                                    Ready(mut body) => {
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
                                                Ok(()) => {}
                                                Err(e) => error!("GetBody: release_capacity: {}", e)
                                            }
                                        } else {
                                            return Ok(Ready((self.buffer.clone(), self.duration)));
                                        }
                                    }
                                    NotReady => return Ok(NotReady),
                                }
                            }
                            Err(e) => {
                                error!("GetBody: {}", e);
                                return Err(());
                            }
                        }
                    }
                }
            }
        }
    }
}
