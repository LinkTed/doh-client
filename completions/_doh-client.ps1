
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'doh-client' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'doh-client'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'doh-client' {
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Listen address [default: 127.0.0.1:53]')
            [CompletionResult]::new('--listen-addr', 'listen-addr', [CompletionResultType]::ParameterName, 'Listen address [default: 127.0.0.1:53]')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Remote address/domain to the DOH server (see below)')
            [CompletionResult]::new('--remote-host', 'remote-host', [CompletionResultType]::ParameterName, 'Remote address/domain to the DOH server (see below)')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'The domain name of the remote server')
            [CompletionResult]::new('--domain', 'domain', [CompletionResultType]::ParameterName, 'The domain name of the remote server')
            [CompletionResult]::new('--retries', 'retries', [CompletionResultType]::ParameterName, 'The number of retries to connect to the remote server')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'The time in seconds after that the connection would be closed if no response is received from the server')
            [CompletionResult]::new('--timeout', 'timeout', [CompletionResultType]::ParameterName, 'The time in seconds after that the connection would be closed if no response is received from the server')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'The path of the URI')
            [CompletionResult]::new('--path', 'path', [CompletionResultType]::ParameterName, 'The path of the URI')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'The size of the private HTTP cache
If the size is 0 then the private HTTP cache is not used (ignores cache-control)')
            [CompletionResult]::new('--cache-size', 'cache-size', [CompletionResultType]::ParameterName, 'The size of the private HTTP cache
If the size is 0 then the private HTTP cache is not used (ignores cache-control)')
            [CompletionResult]::new('--client-auth-certs', 'client-auth-certs', [CompletionResultType]::ParameterName, 'The path to the pem file, which contains the certificates for the client authentication')
            [CompletionResult]::new('--client-auth-key', 'client-auth-key', [CompletionResultType]::ParameterName, 'The path to the pem file, which contains the key for the client authentication')
            [CompletionResult]::new('--proxy-host', 'proxy-host', [CompletionResultType]::ParameterName, 'Socks5 or HTTP CONNECT proxy host (see below)')
            [CompletionResult]::new('--proxy-scheme', 'proxy-scheme', [CompletionResultType]::ParameterName, 'The protocol of the proxy')
            [CompletionResult]::new('--proxy-credentials', 'proxy-credentials', [CompletionResultType]::ParameterName, 'The credentials for the proxy')
            [CompletionResult]::new('--proxy-https-cafile', 'proxy-https-cafile', [CompletionResultType]::ParameterName, 'The path to the pem file, which contains the trusted CA certificates for the https proxy
If no path is given then the platform''s native certificate store will be used')
            [CompletionResult]::new('--proxy-https-domain', 'proxy-https-domain', [CompletionResultType]::ParameterName, 'The domain name of the https proxy')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--listen-activation', 'listen-activation', [CompletionResultType]::ParameterName, 'Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Use the GET method for the HTTP/2.0 request')
            [CompletionResult]::new('--get', 'get', [CompletionResultType]::ParameterName, 'Use the GET method for the HTTP/2.0 request')
            [CompletionResult]::new('--cache-fallback', 'cache-fallback', [CompletionResultType]::ParameterName, 'Use expired cache entries if no response is received from the server')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
