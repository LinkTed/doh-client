# doh-client
`doh-client` is a DNS over HTTPS client, which opens a local UDP (DNS) port and forwards all DNS queries to a remote
HTTP/2.0 server. By default, the client will connect to the Cloudflare DNS service. It uses [Tokio](https://tokio.rs/)
for all asynchronous IO operations and [Rustls](https://github.com/ctz/rustls) to connect to the HTTPS server.
The client uses a private HTTP cache (see [RFC 7234](https://tools.ietf.org/html/rfc7234#section-5.2)) to increase the 
performance if the `--cache-size` is not zero.
[![Build Status](https://travis-ci.org/LinkTed/doh-client.svg?branch=master)](https://travis-ci.org/LinkTed/doh-client)
[![dependency status](https://deps.rs/repo/github/linkted/doh-client/status.svg)](https://deps.rs/repo/github/linkted/doh-client)
[![Latest version](https://img.shields.io/crates/v/doh-client.svg)](https://crates.io/crates/doh-client)
[![License](https://img.shields.io/crates/l/doh-client.svg)](https://opensource.org/licenses/BSD-3-Clause)

## Getting Started
`doh-client` is written in [Rust](https://www.rust-lang.org/). To build it you need the 
[Rust](https://www.rust-lang.org/) compiler and build system `cargo`.

### Build
```
$ cargo build
```
Or to build it as a release build:
```
$ cargo build --release
```

### Run
To run the binary, you need one positional argument (see [Usage](#Usage)).
```
$ ./doh-client /path/to/the/ca/file.pem
```
For example, if you use [Arch Linux](https://www.archlinux.org/) then the following command uses the system cert store:
```
# ./doh-client /etc/ca-certificates/extracted/tls-ca-bundle.pem
```

#### Linux (`systemd`)
To run the `doh-client` as a daemon and without `root` under Linux with `systemd` as init system follow the instructions.
This example will connect to the Cloudflare DNS service.
1. Build the binary (see [Build](#Build)).
2. Copy the binary to `/usr/local/bin` as `root`:
   ```
   # cp target/release/doh-client /usr/local/bin/
   ```
3. Copy the config files to `/etc/systemd/system/` as `root`:
   ```
   # cp doh-client.service doh-client.socket /etc/systemd/system
   ```
   If the location of the binary is different from above then change the path in `doh-client.service` under `ExecStart`. 
   In the config file `doh-client.service` the path of the CA file is set to 
   `/etc/ca-certificates/extracted/tls-ca-bundle.pem`, adjust the path before going further (The path should be correct 
   if you use [Arch Linux](https://www.archlinux.org/)).
4. Reload `systemd` manager configuration:
   ```
   # systemctl daemon-reload
   ```
5. Enable the `doh-client` as a daemon:
   ```
   # systemctl enable doh-client
   ```
6. Reboot the system or start the daemon manually:
   ```
   # systemctl start doh-client
   ```
7. Adjust the `/etc/resolv.conf` by adding the following line:
   ```
   nameserver 127.0.0.1
   ```
##### Additional
If [AppArmor](https://gitlab.com/apparmor/apparmor/wikis/home/) is used then the `doh-client` profile from the 
repository can be applied to [AppArmor](https://gitlab.com/apparmor/apparmor/wikis/home/).
1. Copy the profile file `usr.local.bin.doh-client` to `/etc/apparmor.d/` as `root`:
   ```
   # cp usr.local.bin.doh-client /etc/apparmor.d/
   ```
   If the location of the CA file is different from `/etc/ca-certificates/extracted/tls-ca-bundle.pem` then change the 
   path in `usr.local.bin.doh-client`.
2. Reboot the system or reload all profiles:
   ```
   # systemctl restart apparmor.service
   ```

#### Mac OS (`launchd`)
To run the `doh-client` as a daemon and without `root` under Mac OS with `launchd` as init system.
This example will connect to the Cloudflare DNS service.
1. Build the binary (see [Build](#Build)).
2. Copy the binary to `/usr/local/bin` as `root`: 
   ```
   # cp target/release/doh-client /usr/local/bin/
   ```
3. Copy the `launchd` config files to `/Library/LaunchDaemons/` as `root`:
   ```
   # cp com.doh-client.daemon.plist /Library/LaunchDaemons
   ```
   If the location of the binary is different from above then change the path in `com.doh-client.daemon.plist` under 
   `ProgramArguments`. In the config file `com.doh-client.daemon.plist` the path of the CA file is set to 
   `/usr/local/share/doh-client/DigiCert_Global_Root_CA.pem`, download the pem file under the following 
   [link](https://dl.cacerts.digicert.com/DigiCertGlobalRootCA.crt). Before copy the pem file to 
   `/usr/local/share/doh-client/`, make the directory `doh-client` with `mkdir`.
4. Load and start the config file as follow:
   ```
   # launchctl load -w /Library/LaunchDaemons/com.doh-client.daemon.plist
   ```
5. Adjust the `/etc/resolv.conf` by adding the following line:
   ```
   nameserver 127.0.0.1
   ```

## Usage
`doh-client` has one required positional argument, `CAFILE` which sets the path to a pem file, which contains the 
trusted CA certificates.
```
$ ./doh-client --help
DNS over HTTPS client 1.4.2
link.ted@mailbox.org
Open a local UDP (DNS) port and forward DNS queries to a remote HTTP/2.0 server.
By default, the client will connect to the Cloudflare DNS service.

USAGE:
    doh-client [FLAGS] [OPTIONS] <CAFILE>

FLAGS:
        --cache-fallback       Use expired cache entries if no response is received from the server
    -g, --get                  Use the GET method for the HTTP/2.0 request
    -h, --help                 Prints help information
        --listen-activation    Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS
    -v                         Sets the level of verbosity
    -V, --version              Prints version information

OPTIONS:
    -c, --cache-size <UNSIGNED LONG>    The size of the private HTTP cache
                                        If the size is 0 then the private HTTP cache is not used (ignores cache-control)
                                        [default: 1024]
    -d, --domain <Domain>               The domain name of the remote server [default: cloudflare-dns.com]
    -l, --listen-addr <Addr>            Listen address [default: 127.0.0.1:53]
    -p, --path <STRING>                 The path of the URI [default: dns-query]
    -r, --remote-addr <Addr>            Remote address [default: 1.1.1.1:443]
        --retries <UNSIGNED INT>        The number of retries to connect to the remote server [default: 3]
    -t, --timeout <UNSIGNED LONG>       The time in seconds after that the connection would be closed if no response is
                                        received from the server [default: 2]

ARGS:
    <CAFILE>    The path to the pem file, which contains the trusted CA certificates
```

## Cache performance
To demonstrate that the private HTTP cache (see [RFC 7234](https://tools.ietf.org/html/rfc7234#section-5.2)) increases 
the performance of the client, make a request to `github.com`:
```
$ dig github.com +nocookie

; <<>> DiG 9.13.5 <<>> github.com +nocookie
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 19752
;; flags: qr rd ra; QUERY: 1, ANSWER: 2, AUTHORITY: 0, ADDITIONAL: 1

;; OPT PSEUDOSECTION:
; EDNS: version: 0, flags:; udp: 1452
;; QUESTION SECTION:
;github.com.            IN  A

;; ANSWER SECTION:
github.com.     8   IN  A   192.30.253.112
github.com.     8   IN  A   192.30.253.113

;; Query time: 35 msec
;; SERVER: 127.0.0.1#53(127.0.0.1)
;; WHEN: Sa Jan 05 20:00:20 CET 2019
;; MSG SIZE  rcvd: 71
```
The query took 35 milliseconds. If the request is made again (**quick**, before the response is removed from the cache):
```
$ dig github.com +nocookie

; <<>> DiG 9.13.5 <<>> github.com +nocookie
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 52653
;; flags: qr rd ra; QUERY: 1, ANSWER: 2, AUTHORITY: 0, ADDITIONAL: 1

;; OPT PSEUDOSECTION:
; EDNS: version: 0, flags:; udp: 1452
;; QUESTION SECTION:
;github.com.            IN  A

;; ANSWER SECTION:
github.com.     8   IN  A   192.30.253.112
github.com.     8   IN  A   192.30.253.113

;; Query time: 0 msec
;; SERVER: 127.0.0.1#53(127.0.0.1)
;; WHEN: Sa Jan 05 20:00:21 CET 2019
;; MSG SIZE  rcvd: 71
```
Now, the query took 0 milliseconds, because it was cached.

How long is a DNS request and response in the cache?  
- This depends on the response of HTTP header `control-cache: max-age=XXX`. For example, if the server responds with a 
  `control-cache: max-age=100` then the DNS request and response is in the cache for 100 seconds. After 100 seconds, 
  the client will forward the request to the server again.
