# doh-client
`doh-client` is a DNS over HTTPS client, which opens a local UDP (DNS) port and forwards all DNS queries to a remote
HTTP/2.0 server. By default the client will connect to the Cloudflare DNS service. It uses [Tokio](https://tokio.rs/)
for all asynchronous IO operations and [Rustls](https://github.com/ctz/rustls) to connect to the HTTPS server.  
[![Latest version](https://img.shields.io/crates/v/doh-client.svg)](https://crates.io/crates/doh-client)
[![License](https://img.shields.io/crates/l/doh-client.svg)](https://opensource.org/licenses/BSD-3-Clause)

## Getting Started
`doh-client` is written in Rust. To build it you need the Rust compiler and build system `cargo`.

### Build
```
$ cargo build
```
or to build it as release build
```
$ cargo build --release
```

### Run
To run the binary, you need one option (see [Options](#Options))
```
$ ./doh-client --cafile /path/to/the/ca/file.pem
```
For example if you use [Arch Linux](https://www.archlinux.org/) then the following command uses the system cert store:
```
# ./doh-client --cafile /etc/ca-certificates/extracted/tls-ca-bundle.pem
```
#### Linux (`systemd`)
To run the `doh-client` as daemon and without `root` under Linux with `systemd` as init system:
1. Build the binary see [Build](#Build).
2. Copy as root the `systemd` config files to `/etc/systemd/system/` as follow:
   ```
   # cp doh-client.service doh-client.socket /etc/systemd/system
   ```
3. Reload `systemd` manager configuration:
   ```
   # systemctl daemon-reload
   ```
4. Enable the `doh-client` as a daemon:
   ```
   # systemctl enable doh-client
   ```
5. Reboot the system or start the daemon manually:
   ```
   # systemctl start doh-client
   ```
6. Adjust the `/etc/resolv.conf` by add the following line:
   ```
   nameserver 127.0.0.1
   ```

## Options
`doh-client` has one required option, `--cafile` which sets path to a pem file, which contains the trusted CA
certificates.
```
$ ./doh-client --help
DNS over HTTPS client 1.0
link.ted@mailbox.org
Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.
By default the client will connect to the Cloudflare DNS service.

USAGE:
    doh-client [FLAGS] [OPTIONS] --cafile <FILE>

FLAGS:
    -h, --help                 Prints help information
        --listen-activation    Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS
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
