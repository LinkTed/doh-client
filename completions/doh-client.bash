_doh-client() {
    local i cur prev opts cmds
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${i}" in
            "$1")
                cmd="doh__client"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        doh__client)
            opts="-h -V -l -r -d -t -p -g -c --help --version --listen-addr --listen-activation --remote-host --domain --retries --timeout --path --get --cache-size --cache-fallback --client-auth-certs --client-auth-key --proxy-host --proxy-scheme --proxy-credentials --proxy-https-cafile --proxy-https-domain <CAFILE>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --listen-addr)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -l)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --remote-host)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -r)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --domain)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -t)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -p)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cache-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --client-auth-certs)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --client-auth-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --proxy-host)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --proxy-scheme)
                    COMPREPLY=($(compgen -W "" -- "${cur}"))
                    return 0
                    ;;
                --proxy-credentials)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --proxy-https-cafile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --proxy-https-domain)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _doh-client -o bashdefault -o default doh-client
