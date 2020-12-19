use crate::{DohError, DohResult};
use bytes::{Bytes, BytesMut};
use dns_message_parser::{Dns, MAXIMUM_DNS_PACKET_SIZE};
use h2::client::ResponseFuture;
use h2::RecvStream;
use http::response::Parts;
use std::time::Duration;
use std::u64::MAX;

fn check_header_status(header: &Parts) -> DohResult<()> {
    if header.status.is_success() {
        Ok(())
    } else {
        Err(DohError::HeaderStatus(header.status))
    }
}

fn check_header_content_type(header: &Parts) -> DohResult<()> {
    match header.headers.get("content-type") {
        Some(value) => {
            if value == "application/dns-message" {
                Ok(())
            } else {
                Err(DohError::HeaderContentType(value.clone()))
            }
        }
        None => Err(DohError::HeaderNoContentType),
    }
}

fn get_duration(header: &Parts) -> Option<Duration> {
    if let Some(value) = header.headers.get("cache-control") {
        let value = value.to_str().unwrap();
        for i in value.split(',') {
            let key_value: Vec<&str> = i.splitn(2, '=').map(|s| s.trim()).collect();
            if key_value.len() == 2 && key_value[0] == "max-age" {
                if let Ok(value) = key_value[1].parse::<u64>() {
                    return Some(Duration::from_secs(value));
                }
            }
        }
    }

    None
}

fn get_min_ttl(dns: &Dns) -> Option<Duration> {
    let mut min_ttl = MAX;

    for answer in &dns.answers {
        let ttl = *answer.get_ttl() as u64;
        if ttl < min_ttl {
            min_ttl = ttl;
        }
    }

    if min_ttl == MAX {
        for authority in &dns.authorities {
            let ttl = *authority.get_ttl() as u64;
            if ttl < min_ttl {
                min_ttl = ttl;
            }
        }
    }

    if min_ttl == MAX {
        for additional in &dns.additionals {
            let ttl = *additional.get_ttl() as u64;
            if ttl < min_ttl {
                min_ttl = ttl;
            }
        }
    }

    if min_ttl == MAX || min_ttl == 0 {
        None
    } else {
        Some(Duration::from_secs(min_ttl))
    }
}

async fn get_body(recv_stream: &mut RecvStream) -> DohResult<Bytes> {
    let mut body = BytesMut::new();
    while let Some(result) = recv_stream.data().await {
        let b = result?;
        let body_len = body.len();
        let b_len = b.len();

        recv_stream.flow_control().release_capacity(b_len)?;

        if body_len < MAXIMUM_DNS_PACKET_SIZE {
            if body_len + b_len < MAXIMUM_DNS_PACKET_SIZE {
                body.extend(b);
            } else {
                body.extend(b.slice(0..MAXIMUM_DNS_PACKET_SIZE - body_len));
                break;
            }
        } else {
            break;
        }
    }
    Ok(body.freeze())
}

async fn get_dns_response(recv_stream: &mut RecvStream) -> DohResult<Dns> {
    let body = get_body(recv_stream).await?;
    let dns_response = Dns::decode(&body)?;
    if !dns_response.is_response() {
        return Err(DohError::DnsNotResponse(dns_response));
    }
    Ok(dns_response)
}

pub(super) async fn response_handler(
    response_future: ResponseFuture,
) -> DohResult<(Dns, Option<Duration>)> {
    let response = response_future.await?;
    let (header, mut recv_stream) = response.into_parts();

    check_header_status(&header)?;
    check_header_content_type(&header)?;
    let mut duration = get_duration(&header);

    let dns_response = get_dns_response(&mut recv_stream).await?;

    if duration.is_none() {
        duration = get_min_ttl(&dns_response);
    }

    Ok((dns_response, duration))
}

#[cfg(test)]
mod tests {
    use http::response::Builder;

    use super::check_header_status;

    #[test]
    fn test_check_header_status_200() {
        let response = Builder::new().status(200).body(()).unwrap();

        let (parts, _) = response.into_parts();

        let result = check_header_status(&parts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_header_status_400() {
        let response = Builder::new().status(400).body(()).unwrap();

        let (parts, _) = response.into_parts();

        let result = check_header_status(&parts);
        assert!(result.is_err());
    }
}
