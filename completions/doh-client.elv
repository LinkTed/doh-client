
use builtin;
use str;

set edit:completion:arg-completer[doh-client] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'doh-client'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'doh-client'= {
            cand -l 'Listen address [default: 127.0.0.1:53]'
            cand --listen-addr 'Listen address [default: 127.0.0.1:53]'
            cand -r 'Remote address/domain to the DOH server (see below)'
            cand --remote-host 'Remote address/domain to the DOH server (see below)'
            cand -d 'The domain name of the remote server'
            cand --domain 'The domain name of the remote server'
            cand --retries 'The number of retries to connect to the remote server'
            cand -t 'The time in seconds after that the connection would be closed if no response is received from the server'
            cand --timeout 'The time in seconds after that the connection would be closed if no response is received from the server'
            cand -p 'The path of the URI'
            cand --path 'The path of the URI'
            cand -c 'The size of the private HTTP cache
If the size is 0 then the private HTTP cache is not used (ignores cache-control)'
            cand --cache-size 'The size of the private HTTP cache
If the size is 0 then the private HTTP cache is not used (ignores cache-control)'
            cand --client-auth-certs 'The path to the pem file, which contains the certificates for the client authentication'
            cand --client-auth-key 'The path to the pem file, which contains the key for the client authentication'
            cand --proxy-host 'Socks5 or HTTP CONNECT proxy host (see below)'
            cand --proxy-scheme 'The protocol of the proxy'
            cand --proxy-credentials 'The credentials for the proxy'
            cand --proxy-https-cafile 'The path to the pem file, which contains the trusted CA certificates for the https proxy
If no path is given then the platform''s native certificate store will be used'
            cand --proxy-https-domain 'The domain name of the https proxy'
            cand -h 'Print help information'
            cand --help 'Print help information'
            cand -V 'Print version information'
            cand --version 'Print version information'
            cand --listen-activation 'Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS'
            cand -g 'Use the GET method for the HTTP/2.0 request'
            cand --get 'Use the GET method for the HTTP/2.0 request'
            cand --cache-fallback 'Use expired cache entries if no response is received from the server'
        }
    ]
    $completions[$command]
}
