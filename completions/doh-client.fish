complete -c doh-client -s l -l listen-addr -d 'Listen address [default: 127.0.0.1:53]' -r
complete -c doh-client -s r -l remote-host -d 'Remote address/domain to the DOH server (see below)' -r
complete -c doh-client -s d -l domain -d 'The domain name of the remote server' -r
complete -c doh-client -l retries -d 'The number of retries to connect to the remote server' -r
complete -c doh-client -s t -l timeout -d 'The time in seconds after that the connection would be closed if no response is received from the server' -r
complete -c doh-client -s p -l path -d 'The path of the URI' -r
complete -c doh-client -s c -l cache-size -d 'The size of the private HTTP cache
If the size is 0 then the private HTTP cache is not used (ignores cache-control)' -r
complete -c doh-client -l client-auth-certs -d 'The path to the pem file, which contains the certificates for the client authentication' -r
complete -c doh-client -l client-auth-key -d 'The path to the pem file, which contains the key for the client authentication' -r
complete -c doh-client -l proxy-host -d 'Socks5 or HTTP CONNECT proxy host (see below)' -r
complete -c doh-client -l proxy-scheme -d 'The protocol of the proxy' -r -f -a "{socks5	,socks5h	,http	,https	}"
complete -c doh-client -l proxy-credentials -d 'The credentials for the proxy' -r
complete -c doh-client -l proxy-https-cafile -d 'The path to the pem file, which contains the trusted CA certificates for the https proxy
If no path is given then the platform\'s native certificate store will be used' -r
complete -c doh-client -l proxy-https-domain -d 'The domain name of the https proxy' -r
complete -c doh-client -s h -l help -d 'Print help information'
complete -c doh-client -s V -l version -d 'Print version information'
complete -c doh-client -l listen-activation -d 'Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS'
complete -c doh-client -s g -l get -d 'Use the GET method for the HTTP/2.0 request'
complete -c doh-client -l cache-fallback -d 'Use expired cache entries if no response is received from the server'
