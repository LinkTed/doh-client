use crate::context::Context;
use crate::{DohError, DohResult};
use bytes::Bytes;
use dns_message_parser::{Dns, RCode};
use log::{debug, error};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout as create_timeout;

pub async fn resolve_request(msg: Bytes, context: &Context) -> DohResult<Bytes> {
    let mut dns_request = Dns::decode(msg)?;
    if dns_request.is_response() {
        return Err(DohError::DnsNotRequest(dns_request));
    }

    let id = dns_request.id;

    // 1. Check cache
    if let Some(cache) = &context.cache {
        if dns_request.questions.len() == 1 {
            let question = &dns_request.questions[0];
            let mut guard_cache = cache.lock().await;
            let entry = if context.cache_fallback {
                guard_cache.get_expired(question)
            } else {
                guard_cache.get(question)
            };
            if let Some(dns_response) = entry {
                let mut resp = dns_response.clone();
                resp.id = id;
                debug!("Question is found in cache");
                return Ok(resp.encode()?.into());
            }
        }
    }

    // 2. Fetch from remote DoH
    let mut guard_remote = context.remote_session.lock().await;
    let remote_res = guard_remote.start_request(&mut dns_request).await;
    drop(guard_remote);

    if let Ok((response_future, connection_id)) = remote_res {
        match create_timeout(context.timeout, response_future).await {
            Ok(Ok((mut dns_response, duration))) => {
                if let Some(duration) = duration {
                    if let Some(cache) = &context.cache {
                        if dns_request.questions.len() == 1 {
                            let question = &dns_request.questions[0];
                            let mut guard_cache = cache.lock().await;
                            debug!(
                                "Add records in cache: {}, {}, {:?}",
                                question, dns_response, duration
                            );
                            guard_cache.put(question.clone(), dns_response.clone(), duration);
                        }
                    }
                }
                dns_response.id = id;
                return Ok(dns_response.encode()?.into());
            }
            Ok(Err(e)) => {
                error!("Could not retrieve DNS response from server: {}", e);
            }
            Err(e) => {
                error!("Timeout: {}", e);
            }
        }
        let mut guard_remote = context.remote_session.lock().await;
        guard_remote.disconnect(connection_id);
    }

    // 3. Fallback cache
    if context.cache_fallback {
        if let Some(cache) = &context.cache {
            if dns_request.questions.len() == 1 {
                let question = &dns_request.questions[0];
                let mut guard_cache = cache.lock().await;
                if let Some(dns_response) = guard_cache.get_expired_fallback(question) {
                    let mut resp = dns_response.clone();
                    resp.id = id;
                    debug!("Question is found in cache fallback");
                    return Ok(resp.encode()?.into());
                }
            }
        }
    }

    // Upstream failed: return SERVFAIL DNS response to client instead of dropping TCP connection
    let mut servfail_resp = dns_request;
    servfail_resp.flags.qr = true;
    servfail_resp.flags.rcode = RCode::ServFail;
    Ok(servfail_resp.encode()?.into())
}

pub async fn request_handler(msg: Bytes, addr: SocketAddr, context: &Context) -> DohResult<()> {
    let resp_bytes = resolve_request(msg, context).await?;
    context.sender.send_to(resp_bytes.as_ref(), addr).await?;
    Ok(())
}

pub async fn tcp_connection_handler(
    mut stream: tokio::net::TcpStream,
    context: Arc<Context>,
) -> DohResult<()> {
    let peer_addr = stream.peer_addr()?;
    debug!("New TCP DNS connection from {}", peer_addr);

    loop {
        let mut len_buf = [0u8; 2];
        if stream.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let packet_len = u16::from_be_bytes(len_buf) as usize;
        if packet_len == 0 {
            break;
        }

        let mut msg_buf = vec![0u8; packet_len];
        if stream.read_exact(&mut msg_buf).await.is_err() {
            break;
        }

        let msg = Bytes::from(msg_buf);
        match resolve_request(msg, context.as_ref()).await {
            Ok(resp_bytes) => {
                let resp_len = (resp_bytes.len() as u16).to_be_bytes();
                if stream.write_all(&resp_len).await.is_err() {
                    break;
                }
                if stream.write_all(resp_bytes.as_ref()).await.is_err() {
                    break;
                }
                if stream.flush().await.is_err() {
                    break;
                }
            }
            Err(e) => {
                debug!("TCP query handling failed for {}: {}", peer_addr, e);
                break;
            }
        }
    }
    Ok(())
}
