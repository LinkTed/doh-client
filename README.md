# doh-client
`doh-client` is a DNS over HTTPS client, which opens a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.
The default values connect to the Cloudflare DNS.
It uses [Tokio](https://tokio.rs/) for the asynchronous IO operations and [Rustls](https://github.com/ctz/rustls) to connect to the HTTPS server.

## Getting Started
`doh-client` is written in Rust. To build it you need the Rust compiler and the build system `cargo`.

### Build
```
$ cargo build
```
or to build it as release build
```
$ cargo build --release
```

### Run
To run the binary, you need one option (see [Options](Options))
```
$ ./doh-client --cafile /path/to/the/ca/file.pem
```
For example if you use [Arch Linux](https://www.archlinux.org/) then the following command use the system cert store:
```
# ./doh-client --cafile /etc/ca-certificates/extracted/tls-ca-bundle.pem
```

## Options
`doh-client` have one required option, `--cafile` the path to the pem file, which contains the trusted CA certificates. 
```
$ ./doh-client --help
DNS over HTTPS client 1.0
link.ted@mailbox.org
Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.
The default values connect to the Cloudflare DNS.

USAGE:
    doh-client [FLAGS] [OPTIONS] --cafile <FILE>

FLAGS:
    -h, --help                 Prints help information
        --listen-activation    Use file descriptor 3 as UDP socket
    -v                         Sets the level of verbosity
    -V, --version              Prints version information

OPTIONS:
    -c, --cafile <FILE>              The path to the pem file, which contains the trusted CA certificates
    -d, --domain <Domain>            The domain name of the remote server [default: cloudflare-dns.com]
    -l, --listen-addr <Addr>         Listen address [default: 127.0.0.1:53]
    -r, --remote-addr <Addr>         Remote address [default: 1.1.1.1:443]
        --retries <UNSIGNED INT>     The number of reties to connect to the remote server [default: 3]
        --timeout <UNSIGNED LONG>    The time in seconds after that the connection would be closed if no response is
                                     received from the server [default: 2]
```
