use crate::context::Context;
use crate::{Cache, DohError, DohResult};
use bytes::Bytes;
use dns_message_parser::{Dns, Question};
use futures::channel::mpsc::UnboundedSender;
use futures::lock::Mutex;
use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::timeout as create_timeout;

fn send_response(
    dns_response: &mut Dns,
    id: u16,
    addr: SocketAddr,
    sender: &UnboundedSender<(Bytes, SocketAddr)>,
) -> DohResult<()> {
    dns_response.id = id;
    let bytes = dns_response.to_bytes()?;
    sender.unbounded_send((bytes, addr))?;
    Ok(())
}

enum CacheReturn<'a> {
    Found(DohResult<()>),
    NotFound(Option<(&'a Mutex<Cache<Question, Dns>>, Question)>),
}

#[allow(clippy::needless_lifetimes)]
async fn get_response_from_cache<'a>(
    context: &'a Context,
    dns_request: &Dns,
    addr: &SocketAddr,
) -> CacheReturn<'a> {
    if let Some(cache) = &context.cache {
        let questions = &dns_request.questions;
        if questions.len() == 1 {
            let question = &questions[0];
            let mut guard_cache = cache.lock().await;
            let entry = if context.cache_fallback {
                guard_cache.get_expired(question)
            } else {
                guard_cache.get(question)
            };

            if let Some(dns_response) = entry {
                let id = dns_request.id;
                let sender = &context.sender;
                let addr = *addr;
                debug!("Question is found in cache");
                CacheReturn::Found(send_response(dns_response, id, addr, sender))
            } else {
                debug!("Question is not found in cache");
                CacheReturn::NotFound(Some((cache, question.clone())))
            }
        } else {
            debug!("The amount of questions is not equal 1");
            CacheReturn::NotFound(None)
        }
    } else {
        debug!("Cache is disable");
        CacheReturn::NotFound(None)
    }
}

async fn get_response(
    context: &Context,
    cache_question: &Option<(&Mutex<Cache<Question, Dns>>, Question)>,
    response: (
        impl Future<Output = DohResult<(Dns, Option<Duration>)>>,
        u32,
    ),
    id: u16,
    addr: &SocketAddr,
) -> Option<DohResult<()>> {
    let (response_future, connection_id) = response;
    let timeout = context.timeout;
    match create_timeout(timeout, response_future).await {
        Ok(Ok((mut dns_response, duration))) => {
            let addr = *addr;
            let sender = &context.sender;
            let result = send_response(&mut dns_response, id, addr, sender);
            if let Some(duration) = duration {
                if let Some((cache, question)) = cache_question {
                    let mut guard_cache = cache.lock().await;
                    debug!(
                        "Add records in cache: {}, {}, {:?}",
                        question, dns_response, duration
                    );
                    guard_cache.put(question.clone(), dns_response, duration);
                }
            }
            return Some(result);
        }
        Ok(Err(e)) => {
            error!("Could not retrieve DNS response from server: {}", e);
        }
        Err(e) => {
            error!("Timeout: {}", e);
        }
    }
    let mut guard_remote_session = context.remote_session.lock().await;
    guard_remote_session.disconnect(connection_id);
    None
}

async fn get_response_from_remote(
    context: &Context,
    cache_question: &Option<(&Mutex<Cache<Question, Dns>>, Question)>,
    dns_request: &mut Dns,
    addr: &SocketAddr,
) -> Option<DohResult<()>> {
    let mut guard_remote_session = context.remote_session.lock().await;
    let result = guard_remote_session.start_request(dns_request).await;
    drop(guard_remote_session);
    match result {
        Ok(response) => {
            let id = dns_request.id;
            get_response(context, cache_question, response, id, addr).await
        }
        Err(e) => {
            info!("Could not contact DNS server: {}", e);
            None
        }
    }
}

#[allow(clippy::needless_lifetimes)]
async fn get_response_from_cache_fallback<'a>(
    context: &'a Context,
    cache_question: Option<(&Mutex<Cache<Question, Dns>>, Question)>,
    dns_request: &Dns,
    addr: SocketAddr,
) -> Option<DohResult<()>> {
    if context.cache_fallback {
        if let Some((cache, question)) = &cache_question {
            let mut guard_cache = cache.lock().await;
            if let Some(dns_response) = guard_cache.get_expired_fallback(question) {
                let id = dns_request.id;
                let sender = &context.sender;
                debug!("Question is found in cache fallback");
                Some(send_response(dns_response, id, addr, sender))
            } else {
                debug!("Question is not found in cache fallback");
                None
            }
        } else {
            debug!("Question cannot be cached");
            None
        }
    } else {
        debug!("Cache fallback is disable");
        None
    }
}

pub async fn request_handler(
    msg: Bytes,
    addr: SocketAddr,
    context: &'static Context,
) -> DohResult<()> {
    let mut dns_request = Dns::decode(&msg)?;
    if dns_request.is_response() {
        return Err(DohError::DnsNotRequest(dns_request));
    }

    let cache = get_response_from_cache(context, &dns_request, &addr).await;
    let cache_question = match cache {
        CacheReturn::Found(result) => return result,
        CacheReturn::NotFound(cache_question) => cache_question,
    };

    let remote = get_response_from_remote(context, &cache_question, &mut dns_request, &addr).await;
    if let Some(result) = remote {
        return result;
    }

    let fallback =
        get_response_from_cache_fallback(context, cache_question, &dns_request, addr).await;
    if let Some(result) = fallback {
        return result;
    }

    Err(DohError::CouldNotGetResponse(dns_request))
}
