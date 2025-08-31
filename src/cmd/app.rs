use std::net::SocketAddr;

use clap::value_parser;
use clap::{crate_authors, crate_description, crate_version, Arg, ArgAction, Command};

const ABOUT: &str =
    "Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.\n\
    By default, the client will connect to the Cloudflare DNS service.\n\
    This binary uses the env_logger as logger implementations. \n\
    See https://github.com/sebasmagri/env_logger/";

const AFTER_HELP: &str =
    "CAUTION: If a domain name is used for a <Addr/Domain:Port> value instead of an IP address \
    the system resolver will be used to resolve the IP address of the domain name. If the \
    `doh-client` is configured as system resolver, then it will NOT WORK. It is recommended to \
    always use an IP address for <Addr/Domain:Port> values.\n";

#[cfg(any(feature = "socks5", feature = "http-proxy"))]
fn proxy_args(app: Command) -> Command {
    use clap::ArgAction;

    let mut proxy_host = Arg::new("proxy-host")
        .long("proxy-host")
        .action(ArgAction::Set)
        .value_name("Addr/Domain:Port")
        .required(false)
        .requires("proxy-scheme");

    let mut proxy_scheme = Arg::new("proxy-scheme")
        .long("proxy-scheme")
        .action(ArgAction::Set)
        .help("The protocol of the proxy")
        .required(false)
        .requires("proxy-host");

    if cfg!(all(feature = "socks5", feature = "http-proxy")) {
        proxy_host = proxy_host.help("Socks5 or HTTP CONNECT proxy host (see below)");
        proxy_scheme = proxy_scheme.value_parser(["socks5", "socks5h", "http", "https"]);
    } else if cfg!(all(feature = "socks5", not(feature = "http-proxy"))) {
        proxy_host = proxy_host.help("Socks5 proxy host (see below)");
        proxy_scheme = proxy_scheme.value_parser(["socks5", "socks5h"]);
    } else {
        proxy_host = proxy_host.help("HTTP CONNECT proxy host (see below)");
        proxy_scheme = proxy_scheme.value_parser(["http", "https"]);
    }

    let command = app.arg(proxy_host).arg(proxy_scheme).arg(
        Arg::new("proxy-credentials")
            .long("proxy-credentials")
            .action(ArgAction::Set)
            .value_name("Username:Password")
            .help("The credentials for the proxy")
            .requires_all(&["proxy-host", "proxy-scheme"][..]),
    );

    if cfg!(feature = "http-proxy") {
        let arg = Arg::new("proxy-https-cafile")
            .value_name("CAFILE")
            .long("proxy-https-cafile")
            .action(ArgAction::Set);
        let arg = if cfg!(feature = "native-certs") {
            arg.help(
                "The path to the pem file, which contains the trusted CA \
                      certificates for the https proxy\n\
                      If no path is given then the platform's native certificate store \
                      will be used",
            )
            .required(false)
        } else {
            arg.help(
                "The path to the pem file, which contains the trusted CA \
                      certificates for the https proxy",
            )
            .required_if_eq("proxy-scheme", "https")
        };
        command.arg(arg).arg(
            Arg::new("proxy-https-domain")
                .action(ArgAction::Set)
                .value_name("Domain")
                .long("proxy-https-domain")
                .help("The domain name of the https proxy")
                .required_if_eq("proxy-scheme", "https"),
        )
    } else {
        command
    }
}

fn cafile(command: Command) -> Command {
    let arg = Arg::new("cafile")
        .action(ArgAction::Set)
        .value_name("CAFILE");
    let arg = if cfg!(feature = "native-certs") {
        arg.help(
            "The path to the pem file, which contains the trusted CA certificates\n\
                  If no path is given then the platform's native certificate store will be \
                  used",
        )
        .required(false)
    } else {
        arg.help("The path to the pem file, which contains the trusted CA certificates")
            .required(true)
    };
    command.arg(arg)
}

/// Get the `clap::App` object for the argument parsing.
pub fn get_command() -> Command {
    let command = Command::new(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .arg(
            Arg::new("listen-addr")
                .value_parser(value_parser!(SocketAddr))
                .short('l')
                .long("listen-addr")
                .conflicts_with("listen-activation")
                .action(ArgAction::Set)
                .value_name("Addr:Port")
                .help("Listen address [default: 127.0.0.1:53]")
                .required(false),
        )
        .arg(
            Arg::new("listen-activation")
                .long("listen-activation")
                .action(ArgAction::SetTrue)
                .conflicts_with("listen-addr")
                .help(
                    "Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() \
                    under Mac OS",
                )
                .required(false),
        )
        .arg(
            Arg::new("remote-host")
                .short('r')
                .long("remote-host")
                .action(ArgAction::Set)
                .value_name("Addr/Domain:Port")
                .help("Remote address/domain to the DOH server (see below)")
                .default_value("1.1.1.1:443")
                .required(false),
        )
        .arg(
            Arg::new("domain")
                .short('d')
                .long("domain")
                .action(ArgAction::Set)
                .value_name("Domain")
                .help("The domain name of the remote server")
                .default_value("cloudflare-dns.com")
                .required(false),
        )
        .arg(
            Arg::new("retries")
                .value_parser(value_parser!(u32))
                .action(ArgAction::Set)
                .long("retries")
                .value_name("UNSIGNED INT")
                .help("The number of retries to connect to the remote server")
                .default_value("3")
                .required(false),
        )
        .arg(
            Arg::new("timeout")
                .value_parser(value_parser!(u64))
                .action(ArgAction::Set)
                .short('t')
                .long("timeout")
                .value_name("UNSIGNED LONG")
                .help(
                    "The time in seconds after that the connection would be closed if no response \
                    is received from the server",
                )
                .default_value("2")
                .required(false),
        )
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .action(ArgAction::Set)
                .value_name("STRING")
                .help("The path of the URI")
                .default_value("dns-query")
                .required(false),
        )
        .arg(
            Arg::new("get")
                .short('g')
                .long("get")
                .help("Use the GET method for the HTTP/2.0 request")
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("cache-size")
                .value_parser(value_parser!(usize))
                .long("cache-size")
                .short('c')
                .action(ArgAction::Set)
                .value_name("UNSIGNED LONG")
                .help(
                    "The size of the private HTTP cache\n\
                    If the size is 0 then the private HTTP cache is not used \
                    (ignores cache-control)",
                )
                .default_value("1024")
                .required(false),
        )
        .arg(
            Arg::new("cache-fallback")
                .action(ArgAction::SetTrue)
                .long("cache-fallback")
                .help("Use expired cache entries if no response is received from the server")
                .required(false),
        )
        .arg(
            Arg::new("client-auth-certs")
                .long("client-auth-certs")
                .action(ArgAction::Set)
                .value_name("CERTSFILE")
                .help(
                    "The path to the pem file, which contains the certificates for the client \
                      authentication",
                )
                .required(false)
                .requires("client-auth-key"),
        )
        .arg(
            Arg::new("client-auth-key")
                .long("client-auth-key")
                .action(ArgAction::Set)
                .value_name("KEYFILE")
                .help(
                    "The path to the pem file, which contains the key for the client \
                      authentication",
                )
                .required(false)
                .requires("client-auth-certs"),
        );

    let command = cafile(command);

    #[cfg(any(feature = "socks5", feature = "http-proxy"))]
    let command = proxy_args(command);

    command
}
