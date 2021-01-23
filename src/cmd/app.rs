use cfg_if::cfg_if;
use clap::{crate_authors, crate_description, crate_version, App, Arg};

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
fn proxy_args(app: App<'static, 'static>) -> App<'static, 'static> {
    use clap::ArgGroup;

    cfg_if! {
        if #[cfg(all(feature = "socks5", feature = "http-proxy"))] {
            let proxy_host_help = "Socks5 or HTTP CONNECT proxy host (see below)";
            let proxy_scheme_possible_values = ["socks5", "socks5h", "http", "https"];
        } else if #[cfg(all(feature = "socks5", not(feature = "http-proxy")))] {
            let proxy_host_help = "Socks5 proxy host (see below)";
            let proxy_scheme_possible_values = ["socks5", "socks5h"];
        } else {
            let proxy_host_help = "HTTP CONNECT proxy host (see below)";
            let proxy_scheme_possible_values = ["http", "https"];
        }
    }

    let app = app
        .arg(
            Arg::with_name("proxy-host")
                .long("proxy-host")
                .takes_value(true)
                .value_name("Addr/Domain:Port")
                .help(proxy_host_help)
                .required(false)
                .requires("proxy-scheme"),
        )
        .arg(
            Arg::with_name("proxy-scheme")
                .long("proxy-scheme")
                .takes_value(true)
                .possible_values(&proxy_scheme_possible_values[..])
                .help("The protocol of the proxy")
                .required(false)
                .requires("proxy-host"),
        )
        .arg(
            Arg::with_name("proxy-credentials")
                .long("proxy-credentials")
                .takes_value(true)
                .value_name("Username:Password")
                .help("The credentials for the proxy")
                .requires_all(&["proxy-host", "proxy-scheme"][..]),
        );

    cfg_if! {
        if #[cfg(feature = "http-proxy")] {
            let arg = Arg::with_name("proxy-https-cafile")
                .takes_value(true)
                .value_name("CAFILE")
                .long("proxy-https-cafile")
                .takes_value(true);
            cfg_if! {
                if #[cfg(feature = "native-certs")] {
                    let arg = arg
                        .help("The path to the pem file, which contains the trusted CA \
                              certificates for the https proxy\n\
                              If no path is given then the platform's native certificate store \
                              will be used")
                        .required(false);
                } else {
                    let arg = arg
                        .help("The path to the pem file, which contains the trusted CA \
                              certificates for the https proxy")
                        .required_if("proxy-scheme", "https")
                }
            }

            app.arg(arg).arg(
                Arg::with_name("proxy-https-domain")
                    .takes_value(true)
                    .value_name("Domain")
                    .long("proxy-https-domain")
                    .help("The domain name of the https proxy")
                    .required_if("proxy-scheme", "https"),
            )
            .group(
                ArgGroup::with_name("proxy")
                    .args(&["proxy-host", "proxy-scheme", "proxy-credentials",
                            "proxy-https-domain", "proxy-https-cafile"][..])
            )
        } else {
            app.group(
                ArgGroup::with_name("proxy")
                    .args(&["proxy-host", "proxy-scheme", "proxy-credentials"][..])
            )
        }
    }
}

fn cafile(app: App<'static, 'static>) -> App<'static, 'static> {
    let arg = Arg::with_name("cafile")
        .takes_value(true)
        .value_name("CAFILE");
    cfg_if! {
        if #[cfg(feature = "native-certs")] {
            let arg = arg
                .help("The path to the pem file, which contains the trusted CA certificates\n\
                      If no path is given then the platform's native certificate store will be \
                      used")
                .required(false);
        } else {
            let arg = arg
                .help("The path to the pem file, which contains the trusted CA certificates")
                .required(true);
        }
    }
    app.arg(arg)
}

/// Get the `clap::App` object for the argument parsing.
pub fn get_app() -> App<'static, 'static> {
    let app = App::new(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .arg(
            Arg::with_name("listen-addr")
                .short("l")
                .long("listen-addr")
                .conflicts_with("listen-activation")
                .takes_value(true)
                .value_name("Addr")
                .help("Listen address [default: 127.0.0.1:53]")
                .required(false),
        )
        .arg(
            Arg::with_name("listen-activation")
                .long("listen-activation")
                .help(
                    "Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() \
                    under Mac OS",
                )
                .required(false),
        )
        .arg(
            Arg::with_name("remote-host")
                .short("r")
                .long("remote-host")
                .takes_value(true)
                .value_name("Addr/Domain:Port")
                .help("Remote address/domain to the DOH server (see below)")
                .default_value("1.1.1.1:443")
                .required(false),
        )
        .arg(
            Arg::with_name("domain")
                .short("d")
                .long("domain")
                .takes_value(true)
                .value_name("Domain")
                .help("The domain name of the remote server")
                .default_value("cloudflare-dns.com")
                .required(false),
        )
        .arg(
            Arg::with_name("retries")
                .takes_value(true)
                .long("retries")
                .value_name("UNSIGNED INT")
                .help("The number of retries to connect to the remote server")
                .default_value("3")
                .required(false),
        )
        .arg(
            Arg::with_name("timeout")
                .takes_value(true)
                .short("t")
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
            Arg::with_name("path")
                .short("p")
                .long("path")
                .takes_value(true)
                .value_name("STRING")
                .help("The path of the URI")
                .default_value("dns-query")
                .required(false),
        )
        .arg(
            Arg::with_name("get")
                .short("g")
                .long("get")
                .help("Use the GET method for the HTTP/2.0 request")
                .required(false),
        )
        .arg(
            Arg::with_name("cache-size")
                .long("cache-size")
                .short("c")
                .takes_value(true)
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
            Arg::with_name("cache-fallback")
                .long("cache-fallback")
                .help("Use expired cache entries if no response is received from the server")
                .required(false),
        );

    let app = cafile(app);

    #[cfg(any(feature = "socks5", feature = "http-proxy"))]
    let app = proxy_args(app);

    app
}
