use clap::{App, Arg};

/// Get the `clap::App` object for the argument parsing.
pub fn get_app() -> App<'static, 'static> {
    let app = App::new("DNS over HTTPS client")
        .version("2.2.0")
        .author("link.ted@mailbox.org")
        .about(
            "Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.\n\
        By default, the client will connect to the Cloudflare DNS service.\n\
        This binary uses the env_logger as logger implementations. \
        See https://github.com/sebasmagri/env_logger/",
        )
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
                .value_name("Addr/Name")
                .help(
                    "Remote address/hostname to the DOH server \
                (If a hostname is used then another DNS server has to be configured)",
                )
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
            Arg::with_name("cafile")
                .takes_value(true)
                .value_name("CAFILE")
                .help("The path to the pem file, which contains the trusted CA certificates")
                .required(true),
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
                If the size is 0 then the private HTTP cache is not used (ignores cache-control)",
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

    #[cfg(feature = "socks5")]
    let app = app.arg(
        Arg::with_name("socks5")
            .long("socks5")
            .takes_value(true)
            .value_name("URL")
            .help(
                "Socks5 proxy URL\n\
                    (example: socks5://user:password@example.com or socks5h://example.com)",
            )
            .required(false),
    );
    app
}
