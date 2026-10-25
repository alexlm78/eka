_eka() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="eka"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        eka)
            opts="-f -1 -C -x -m -l -n -g -o -G -T -w -d -a -A -h -s -k -F -p -R -S -t -v -X -U -u -c -r -I -B -N -b -Q -q -i -H -L -P -V --help --completions --format --numeric-uid-gid --no-group --tabsize --width --directory --all --almost-all --human-readable --si --size --kibibytes --block-size --classify --indicator-style --file-type --recursive --time --time-style --full-time --reverse --group-directories-first --ignore --hide --ignore-backups --literal --escape --quote-name --hide-control-chars --show-control-chars --quoting-style --inode --dereference-command-line --dereference --sort --color --version"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --completions)
                    COMPREPLY=($(compgen -W "bash elvish fish powershell zsh" -- "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "across commas horizontal long single-column verbose vertical" -- "${cur}"))
                    return 0
                    ;;
                --tabsize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --block-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --indicator-style)
                    COMPREPLY=($(compgen -W "none slash file-type classify" -- "${cur}"))
                    return 0
                    ;;
                -p)
                    COMPREPLY=($(compgen -W "none slash file-type classify" -- "${cur}"))
                    return 0
                    ;;
                --time)
                    COMPREPLY=($(compgen -W "modified accessed changed birth" -- "${cur}"))
                    return 0
                    ;;
                --time-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ignore)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -I)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hide)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --quoting-style)
                    COMPREPLY=($(compgen -W "literal shell shell-always shell-escape shell-escape-always c escape" -- "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -W "name none size time extension version" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "always auto never" -- "${cur}"))
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

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _eka -o nosort -o bashdefault -o default eka
else
    complete -F _eka -o bashdefault -o default eka
fi
