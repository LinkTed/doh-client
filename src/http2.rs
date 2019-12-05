use std::sync::Arc;
use std::net::{SocketAddr};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::ops::DerefMut;
use std::time::Duration;

use data_encoding::BASE64URL_NOPAD;

use tokio::spawn;
use tokio::time::{delay_for, timeout};
use tokio::net::TcpStream;

use rustls::ClientConfig;

use webpki::DNSNameRef;

use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use h2::client::{SendRequest, ResponseFuture, Connection, handshake};

use http::Request;

use bytes::{Bytes, BytesMut};

use crate::dns::{MAXIMUM_DNS_PACKET_SIZE, DnsPacket};
use crate::Context;


pub fn create_config(cafile: &str) -> io::Result<ClientConfig> {
    let certfile = File::open(&cafile)?;

    let mut config = ClientConfig::new();
    if let Err(()) = config.root_store.add_pem_file(&mut BufReader::new(certfile)) {
        return Err(io::Error::new(io::ErrorKind::Other, "Cannot parse pem file"));
    }
    config.alpn_protocols.push(vec![104, 50]); // h2
    Ok(config)
}

pub async fn http2_request(msg: DnsPacket, addr: SocketAddr, context: &'static Context) {
    let config = &context.config;
    let mutex_send_request = &context.mutex_send_request;
    let mutex_cache = &context.mutex_cache;
    let mut response = None;

    if config.cache_size != 0 {
        let mut mutex_guard_cache = mutex_cache.lock().await;
        let entry = if config.cache_fallback {
            mutex_guard_cache.get_expired(&msg.get_without_tid())
        } else {
            mutex_guard_cache.get(&msg.get_without_tid())
        };

        match entry {
            Some(buffer) => {
                debug!("found in cache");
                match DnsPacket::from_tid((*buffer).clone(), msg.get_tid()) {
                    Ok(dns) => {
                        response.replace(dns);
                    }
                    Err(e) => {
                        error!("parse error: {}", e);
                    }
                }
            }
            None => {
                debug!("missing in cache");
            }
        }
    }

    if response.is_none() {
        let mut mutex_guard_send_request = mutex_send_request.lock().await;
        if mutex_guard_send_request.is_none() {
            for tries in 0..config.retries {
                info!("try to connect: {}", tries + 1);
                match http2_connection(config.remote_addr, config.client_config.clone(), &config.domain).await {
                    Ok((send_request, connection)) => {
                        info!("connection was successfully established to remote server {} ({})", config.remote_addr, config.domain);

                        mutex_guard_send_request.replace(send_request);
                        spawn(async move {
                            if let Err(e) = connection.await {
                                error!("connection close: {}", e);
                            }
                        });
                        break;
                    }
                    Err(e) => {
                        error!("connection to remote server {} ({}) failed: {}", config.remote_addr, config.domain, e);
                        delay_for(Duration::from_secs(1)).await;
                    }
                }
            }
        }

        if let Some(send_request) = mutex_guard_send_request.deref_mut() {
            let post = config.post;

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

            let http2_response_future = match send_request.send_request(request, false) {
                Ok((response, mut request)) => {
                    if post {
                        match request.send_data(msg.get_without_tid(), true) {
                            Ok(()) => {
                                Some(http2_response(response))
                            }
                            Err(e) => {
                                error!("send_data: {}", e);
                                mutex_guard_send_request.take();
                                None
                            }
                        }
                    } else {
                        Some(http2_response(response))
                    }
                },
                Err(e) => {
                    error!("send_request: {}", e);
                    mutex_guard_send_request.take();
                    None
                }
            };

            if let Some(http2_response_future) = http2_response_future {
                match timeout(Duration::from_secs(config.timeout), http2_response_future).await {
                    Ok(Ok((buffer, duration))) => {
                        match DnsPacket::from_tid(buffer.clone(), msg.get_tid()) {
                            Ok(dns) => {
                                if dns.is_response() {
                                    response.replace(dns.clone());
                                    if config.cache_size != 0 {
                                        if let Some(duration) = duration {
                                            let mut mutex_guard_cache = mutex_cache.lock().await;
                                            mutex_guard_cache.put(msg.get_without_tid(), buffer.clone(), duration.clone());
                                        }
                                    }
                                } else {
                                    error!("get a non DNS response");
                                }
                            }
                            Err(e) => {
                                error!("DNS parser error: {}", e);
                            }
                        }
                    }
                    Ok(Err(_)) => {
                        error!("http2_response");
                        mutex_guard_send_request.take();
                    }
                    Err(e) => {
                        error!("timeout: {}", e);
                        mutex_guard_send_request.take();
                    }
                }
            }
        }

        if response.is_none() && config.cache_fallback {
            let mut mutex_guard_cache = mutex_cache.lock().await;
            match mutex_guard_cache.get_expired_fallback(&msg.get_without_tid()) {
                Some(buffer) => {
                    debug!("found in fallback cache");
                    match DnsPacket::from_tid(buffer.clone(), msg.get_tid()) {
                        Ok(dns) => {
                            response.replace(dns);
                        }
                        Err(e) => {
                            error!("parse error: {}", e);
                        }
                    }
                }
                None => {
                    debug!("missing in cache fallback");
                }
            }
        }
    }

    if let Some(dns) = response {
        if let Err(e) = context.sender.unbounded_send((dns, addr)) {
            error!("unbounded_send: {}", e);
        }
    } else {
        error!("response is none")
    }
}

async fn http2_connection(remote_addr: SocketAddr, config: Arc<ClientConfig>, domain: &str) -> io::Result<(SendRequest<Bytes>, Connection<TlsStream<TcpStream>>)> {
    let connection = TcpStream::connect(&remote_addr).await?;
    connection.set_keepalive(Some(Duration::from_secs(1)))?;
    connection.set_nodelay(true)?;

    let tls_connector = TlsConnector::from(config);
    let tls_connection = tls_connector.connect(DNSNameRef::try_from_ascii_str(domain).unwrap(), connection).await?;
    match handshake(tls_connection).await {
        Ok(result) => Ok(result),
        Err(e) => {
            Err(io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

async fn http2_response(response_future: ResponseFuture) -> Result<(Bytes, Option<Duration>), ()> {
    let mut duration = None;
    let response = match response_future.await {
        Ok(response) => response,
        Err(e) => {
            error!("response_future: {}", e);
            return Err(());
        }
    };
    let (header, mut body) = response.into_parts();

    if header.status != 200 {
        error!("header.status != 200");
        return Err(());
    }

    let headers = &header.headers;

    match headers.get("content-type") {
        Some(value) => {
            if value != "application/dns-message" {
                error!("content-type != application/dns-message");
                return Err(());
            }
        }
        None => {
            error!("content-type is None");
            return Err(());
        }
    }

    if let Some(value) = headers.get("cache-control") {
        for i in value.to_str().unwrap().split(",") {
            let key_value: Vec<&str> = i.splitn(2, "=").map(|s| s.trim()).collect();
            if key_value.len() == 2 && key_value[0] == "max-age" {
                if let Ok(value) = key_value[1].parse::<u64>() {
                    duration.replace(Duration::from_secs(value));
                }
            }
        }
    }

    let mut buffer = BytesMut::new();
    while let Some(result) = body.data().await {
        match result {
            Ok(b) => {
                let buffer_len = buffer.len();
                let b_len = b.len();

                if buffer_len < MAXIMUM_DNS_PACKET_SIZE {
                    if buffer_len + b_len < MAXIMUM_DNS_PACKET_SIZE {
                        buffer.extend(b);
                    } else {
                        buffer.extend(b.slice(0..MAXIMUM_DNS_PACKET_SIZE - buffer_len));
                    }
                }

                if let Err(e) =  body.flow_control().release_capacity(b_len) {
                    error!("release_capacity: {}", e);
                    return Err(())
                }
            }
            Err(e) => {
                error!("data: {}", e);
                return Err(())
            }
        }
    }

    Ok((buffer.freeze(), duration))
}
