use crate::RemoteHost;

use clap::ArgMatches;

#[cfg(feature = "socks5")]
use std::borrow::Cow;
use std::io::Error as IoError;
use std::net::SocketAddr;
#[cfg(feature = "socks5")]
use std::net::{IpAddr, SocketAddrV4, SocketAddrV6};

#[cfg(feature = "socks5")]
use tokio_socks::TargetAddr;

use tokio::net::lookup_host;

#[cfg(feature = "socks5")]
use url::{Host, ParseError, Url};

#[derive(Debug)]
pub enum RemoteHostError {
    #[cfg(feature = "socks5")]
    Socks5(ParseError),
    #[cfg(feature = "socks5")]
    Socks5Scheme(String),
    CouldNotResolve(String),
    UnknownPort(String),
    UnknownHost(String),
}

#[cfg(feature = "socks5")]
impl From<ParseError> for RemoteHostError {
    fn from(e: ParseError) -> Self {
        RemoteHostError::Socks5(e)
    }
}

impl From<IoError> for RemoteHostError {
    fn from(e: IoError) -> Self {
        RemoteHostError::CouldNotResolve(e.to_string())
    }
}

#[cfg(feature = "socks5")]
fn get_socks5_url(arg_matches: &ArgMatches) -> Result<Option<Url>, ParseError> {
    if let Some(socks5) = arg_matches.value_of("socks5") {
        let url = Url::parse(&socks5)?;
        return Ok(Some(url));
    }

    Ok(None)
}
#[cfg(feature = "socks5")]
async fn get_socks5_remote_addrs(url: &Url) -> Result<Vec<SocketAddr>, RemoteHostError> {
    if let Some(host) = url.host() {
        match host {
            Host::Domain(domain) => {
                if let Some(port) = url.port_or_known_default() {
                    let host = format!("{}:{}", host, port);
                    let remote_addrs = lookup_host(host).await?;
                    let remote_addrs = remote_addrs.collect();
                    Ok(remote_addrs)
                } else {
                    Err(RemoteHostError::UnknownPort(domain.to_string()))
                }
            }
            Host::Ipv4(ipv4_addr) => {
                if let Some(port) = url.port_or_known_default() {
                    let remote_addr = SocketAddr::V4(SocketAddrV4::new(ipv4_addr, port));
                    let remote_addrs = vec![remote_addr];
                    Ok(remote_addrs)
                } else {
                    Err(RemoteHostError::UnknownPort(ipv4_addr.to_string()))
                }
            }
            Host::Ipv6(ipv6_addr) => {
                if let Some(port) = url.port_or_known_default() {
                    let remote_addr = SocketAddr::V6(SocketAddrV6::new(ipv6_addr, port, 0, 0));
                    let remote_addrs = vec![remote_addr];
                    Ok(remote_addrs)
                } else {
                    Err(RemoteHostError::UnknownPort(ipv6_addr.to_string()))
                }
            }
        }
    } else {
        Err(RemoteHostError::UnknownHost(url.to_string()))
    }
}
#[cfg(feature = "socks5")]
fn get_resolve(url: &Url) -> Result<bool, RemoteHostError> {
    match url.scheme() {
        "socks5" => Ok(true),
        "socks5h" => Ok(false),
        scheme => Err(RemoteHostError::Socks5Scheme(scheme.to_string())),
    }
}
#[cfg(feature = "socks5")]
async fn get_dest_addrs(
    arg_matches: &ArgMatches<'static>,
    url: &Url,
) -> Result<Vec<TargetAddr<'static>>, RemoteHostError> {
    let remote_host = String::from(arg_matches.value_of("remote-host").unwrap());

    let dest_addr: Vec<&str> = remote_host.rsplitn(2, ':').collect();
    if dest_addr.len() != 2 {
        return Err(RemoteHostError::UnknownHost(remote_host));
    }
    let host = dest_addr[1];
    let port = if let Ok(port) = dest_addr[0].parse() {
        port
    } else {
        return Err(RemoteHostError::UnknownPort(remote_host));
    };

    match host.parse::<IpAddr>() {
        Ok(dest_addr) => {
            let dest_addr = SocketAddr::new(dest_addr, port);
            let dest_addrs = vec![TargetAddr::Ip(dest_addr)];
            Ok(dest_addrs)
        }
        Err(_) => {
            let resolve = get_resolve(url)?;
            if resolve {
                let dest_addrs = lookup_host(&remote_host).await?;
                let dest_addrs = dest_addrs.map(TargetAddr::Ip).collect();
                Ok(dest_addrs)
            } else {
                let dest_addr = TargetAddr::Domain(Cow::Owned(host.to_string()), port);
                let dest_addrs = vec![dest_addr];
                Ok(dest_addrs)
            }
        }
    }
}
#[cfg(feature = "socks5")]
fn get_credentials(url: &Url) -> Option<(String, String)> {
    let username = url.username();
    let password = url.password();

    if username.is_empty() {
        None
    } else if let Some(password) = password {
        Some((username.to_string(), password.to_string()))
    } else {
        None
    }
}
#[cfg(feature = "socks5")]
async fn get_socks5(
    arg_matches: &ArgMatches<'static>,
) -> Result<
    Option<(
        Vec<SocketAddr>,
        Option<(String, String)>,
        Vec<TargetAddr<'static>>,
    )>,
    RemoteHostError,
> {
    let url = get_socks5_url(arg_matches)?;
    if let Some(url) = url {
        let remote_addrs = get_socks5_remote_addrs(&url).await?;
        let credentials = get_credentials(&url);
        let dest_addrs = get_dest_addrs(arg_matches, &url).await?;
        Ok(Some((remote_addrs, credentials, dest_addrs)))
    } else {
        Ok(None)
    }
}

async fn get_remote_addrs(
    arg_matches: &ArgMatches<'static>,
) -> Result<Vec<SocketAddr>, RemoteHostError> {
    let remote_host = String::from(arg_matches.value_of("remote-host").unwrap());

    match remote_host.parse() {
        Ok(addr) => Ok(vec![addr]),
        Err(_) => {
            let addr = lookup_host(remote_host).await?;
            Ok(addr.collect())
        }
    }
}

pub async fn get_remote_host(
    arg_matches: &ArgMatches<'static>,
) -> Result<RemoteHost, RemoteHostError> {
    #[cfg(feature = "socks5")]
    match get_socks5(arg_matches).await? {
        Some((remote_addrs, credentials, dest_addrs)) => {
            let remote_host = RemoteHost::Socks5(remote_addrs, credentials, dest_addrs);
            Ok(remote_host)
        }
        None => {
            let remote_addrs = get_remote_addrs(arg_matches).await?;
            let remote_host = RemoteHost::Direct(remote_addrs);
            Ok(remote_host)
        }
    }
    #[cfg(not(feature = "socks5"))]
    {
        let remote_addrs = get_remote_addrs(arg_matches).await?;
        let remote_host = RemoteHost::Direct(remote_addrs);
        Ok(remote_host)
    }
}
