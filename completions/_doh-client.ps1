
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
                $element.Value.StartsWith('-')) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'doh-client' {
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Listen address [default: 127.0.0.1:53]')
            [CompletionResult]::new('--listen-addr', 'listen-addr', [CompletionResultType]::ParameterName, 'Listen address [default: 127.0.0.1:53]')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Remote address/hostname to the DOH server (If a hostname is used then another DNS server has to be configured)')
            [CompletionResult]::new('--remote-host', 'remote-host', [CompletionResultType]::ParameterName, 'Remote address/hostname to the DOH server (If a hostname is used then another DNS server has to be configured)')
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
            [CompletionResult]::new('--socks5', 'socks5', [CompletionResultType]::ParameterName, 'Socks5 proxy URL
CAUTION: If a domain name is used instead of an IP address the system resolver will be used to resolve the IP address of the proxy. If the `doh-client` is configured as system resolver, then it will NOT WORK. It is recommended to always use an IP address for the socks proxy.
(example: socks5://user:password@example.com or socks5h://example.com)')
            [CompletionResult]::new('--listen-activation', 'listen-activation', [CompletionResultType]::ParameterName, 'Use file descriptor 3 under Unix as UDP socket or launch_activate_socket() under Mac OS')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Use the GET method for the HTTP/2.0 request')
            [CompletionResult]::new('--get', 'get', [CompletionResultType]::ParameterName, 'Use the GET method for the HTTP/2.0 request')
            [CompletionResult]::new('--cache-fallback', 'cache-fallback', [CompletionResultType]::ParameterName, 'Use expired cache entries if no response is received from the server')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Prints help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Prints help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Prints version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Prints version information')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
