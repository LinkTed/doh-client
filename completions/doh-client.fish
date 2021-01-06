complete -c doh-client -n "__fish_use_subcommand" -s l -l listen-addr -d 'Listen address [default: 127.0.0.1:53]'
complete -c doh-client -n "__fish_use_subcommand" -s r -l remote-host -d 'Remote address/hostname to the DOH server (If a hostname is used then another DNS server has to be configured)'
complete -c doh-client -n "__fish_use_subcommand" -s d -l domain -d 'The domain name of the remote server'
complete -c doh-client -n "__fish_use_subcommand" -l retries -d 'The number of retries to connect to the remote server'
complete -c doh-client -n "__fish_use_subcommand" -s t -l timeout -d 'The time in seconds after that the connection would be closed if no response is received from the server'
complete -c doh-client -n "__fish_use_subcommand" -s p -l path -d 'The path of the URI'
complete -c doh-client -n "__fish_use_subcommand" -s c -l cache-size -d 'The size of the private HTTP cache
If the size is 0 then the private HTTP cache is not used (ignores cache-control)'
complete -c doh-client -n "__fish_use_subcommand" -l socks5 -d 'Socks5 proxy URL
CAUTION: If a domain name is used instead of an IP address the system resolver will be used to resolve the IP address of the proxy. If the `doh-client` is configured as system resolver, then it will NOT WORK. It is recommended to always use an IP address for the socks proxy.
(example: socks5://user:password@example.com or socks5h://example.com)'
complete -c doh-client -n "__fish_use_subcommand" -l listen-activation -d 'Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS'
complete -c doh-client -n "__fish_use_subcommand" -s g -l get -d 'Use the GET method for the HTTP/2.0 request'
complete -c doh-client -n "__fish_use_subcommand" -l cache-fallback -d 'Use expired cache entries if no response is received from the server'
complete -c doh-client -n "__fish_use_subcommand" -s h -l help -d 'Prints help information'
complete -c doh-client -n "__fish_use_subcommand" -s V -l version -d 'Prints version information'
