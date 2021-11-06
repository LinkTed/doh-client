#[cfg(feature = "http-proxy")]
use crate::helper::load_root_store;
use crate::RemoteHost;
use clap::ArgMatches;
#[cfg(feature = "http-proxy")]
use rustls::ClientConfig;
use std::io::Error as IoError;
#[cfg(feature = "socks5")]
use std::net::{IpAddr, SocketAddr};
#[cfg(feature = "http-proxy")]
use std::sync::Arc;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum RemoteHostError {
    #[cfg(any(feature = "socks5", feature = "http-proxy"))]
    #[error("Could not parse proxy scheme: {0}")]
    ProxyScheme(String),
    #[cfg(any(feature = "socks5", feature = "http-proxy"))]
    #[error("Could not parse proxy credentials: {0}")]
    ProxyCredentials(String),
    #[error("IO Error: {0}")]
    Io(#[from] IoError),
    #[error("Unknown port: {0}")]
    UnknownPort(String),
    #[error("Unknown hostm and port: {0}")]
    UnknownHostPort(String),
}

fn parse_host_port(host_port: &str) -> Result<(&str, u16), RemoteHostError> {
    let host_port_vec: Vec<&str> = host_port.rsplitn(2, ':').collect();
    if host_port_vec.len() != 2 {
        return Err(RemoteHostError::UnknownHostPort(host_port.to_owned()));
    }
    let host = host_port_vec[1];
    if let Ok(port) = host_port_vec[0].parse() {
        Ok((host, port))
    } else {
        Err(RemoteHostError::UnknownPort(host.to_owned()))
    }
}

fn get_remote_host_port(
    arg_matches: &ArgMatches<'static>,
) -> Result<(String, u16), RemoteHostError> {
    let remote_host_port = arg_matches.value_of("remote-host").unwrap();
    let (remote_host, remote_port) = parse_host_port(remote_host_port)?;
    Ok((remote_host.to_owned(), remote_port))
}

fn get_direct(arg_matches: &ArgMatches<'static>) -> Result<RemoteHost, RemoteHostError> {
    let (remote_host, remote_port) = get_remote_host_port(arg_matches)?;
    let remote_host = RemoteHost::Direct(remote_host, remote_port);
    Ok(remote_host)
}

#[cfg(any(feature = "socks5", feature = "http-proxy"))]
async fn get_proxy_host_port(
    arg_matches: &ArgMatches<'static>,
) -> Result<(String, u16), RemoteHostError> {
    let proxy_host_port = arg_matches.value_of("proxy-host").unwrap();
    let (proxy_host, proxy_port) = parse_host_port(proxy_host_port)?;
    Ok((proxy_host.to_owned(), proxy_port))
}

#[cfg(feature = "socks5")]
async fn get_proxy_remote_addrs(
    arg_matches: &ArgMatches<'static>,
) -> Result<Vec<SocketAddr>, RemoteHostError> {
    let remote_host = arg_matches.value_of("remote-host").unwrap();
    let (host, port) = parse_host_port(remote_host)?;
    match host.parse::<IpAddr>() {
        Ok(host) => {
            let remote_addrs = vec![SocketAddr::new(host, port)];
            Ok(remote_addrs)
        }
        Err(_) => {
            let remote_addrs = tokio::net::lookup_host(remote_host).await?;
            let remote_addrs = remote_addrs.collect();
            Ok(remote_addrs)
        }
    }
}

#[cfg(any(feature = "socks5", feature = "http-proxy"))]
fn get_proxy_credentials(
    arg_matches: &ArgMatches<'static>,
) -> Result<Option<(String, String)>, RemoteHostError> {
    if let Some(proxy_credentials) = arg_matches.value_of("proxy-credentials") {
        let proxy_credentials_vec: Vec<&str> = proxy_credentials.splitn(2, ':').collect();
        if proxy_credentials_vec.len() == 2 {
            let username = proxy_credentials_vec[0].to_owned();
            let password = proxy_credentials_vec[1].to_owned();
            Ok(Some((username, password)))
        } else {
            Err(RemoteHostError::ProxyCredentials(
                proxy_credentials.to_owned(),
            ))
        }
    } else {
        Ok(None)
    }
}

#[cfg(feature = "http-proxy")]
fn get_proxy_https_client_config(
    arg_matches: &ArgMatches<'static>,
) -> Result<ClientConfig, RemoteHostError> {
    let https_cafile = arg_matches.value_of("proxy-https-cafile");
    let root_store = load_root_store(https_cafile)?;
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    config
        .alpn_protocols
        .push(vec![0x68, 0x74, 0x74, 0x70, 0x2f, 0x31, 0x2e, 0x31]); // http/1.1
    Ok(config)
}

#[cfg(feature = "http-proxy")]
fn get_proxy_https_domain(arg_matches: &ArgMatches<'static>) -> String {
    arg_matches
        .value_of("proxy-https-domain")
        .unwrap()
        .to_owned()
}

#[cfg(any(feature = "socks5", feature = "http-proxy"))]
async fn get_proxy(arg_matches: &ArgMatches<'static>) -> Result<RemoteHost, RemoteHostError> {
    let proxy_scheme = arg_matches.value_of("proxy-scheme");
    if let Some(proxy_scheme) = proxy_scheme {
        let (proxy_host, proxy_port) = get_proxy_host_port(arg_matches).await?;
        let credentials = get_proxy_credentials(arg_matches)?;
        match proxy_scheme {
            #[cfg(feature = "socks5")]
            "socks5" => {
                let remote_addrs = get_proxy_remote_addrs(arg_matches).await?;
                Ok(RemoteHost::Socks5(
                    proxy_host,
                    proxy_port,
                    credentials,
                    remote_addrs,
                ))
            }
            #[cfg(feature = "socks5")]
            "socks5h" => {
                let (remote_host, remote_port) = get_remote_host_port(arg_matches)?;
                Ok(RemoteHost::Socks5h(
                    proxy_host,
                    proxy_port,
                    credentials,
                    remote_host,
                    remote_port,
                ))
            }
            #[cfg(feature = "http-proxy")]
            "http" => {
                let (remote_host, remote_port) = get_remote_host_port(arg_matches)?;
                Ok(RemoteHost::HttpProxy(
                    proxy_host,
                    proxy_port,
                    credentials,
                    remote_host,
                    remote_port,
                ))
            }
            #[cfg(feature = "http-proxy")]
            "https" => {
                let (remote_host, remote_port) = get_remote_host_port(arg_matches)?;
                let https_client_config = get_proxy_https_client_config(arg_matches)?;
                let https_client_config = Arc::new(https_client_config);
                let https_domain = get_proxy_https_domain(arg_matches);
                Ok(RemoteHost::HttpsProxy(
                    proxy_host,
                    proxy_port,
                    credentials,
                    remote_host,
                    remote_port,
                    https_client_config,
                    https_domain,
                ))
            }
            scheme => Err(RemoteHostError::ProxyScheme(scheme.to_string())),
        }
    } else {
        get_direct(arg_matches)
    }
}

#[cfg(all(not(feature = "socks5"), not(feature = "http-proxy")))]
async fn get_proxy(_: &ArgMatches<'static>) -> Result<RemoteHost, RemoteHostError> {
    Err(RemoteHostError::Io(IoError::new(
        std::io::ErrorKind::Other,
        "Feature native-certs is not enabled",
    )))
}

pub async fn get_remote_host(
    arg_matches: &ArgMatches<'static>,
) -> Result<RemoteHost, RemoteHostError> {
    if cfg!(any(feature = "socks5", feature = "http-proxy")) {
        get_proxy(arg_matches).await
    } else {
        get_direct(arg_matches)
    }
}
