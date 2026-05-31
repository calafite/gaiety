gai() {
    case "$1" in
        reload)
            if [[ -n "$2" ]]; then
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] reloading module $2...\033[0m"
                source "$3"
                echo "\033[1m\033[32m✓ \033[0mreloaded $2"
            else
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] running reset and reloading modules...\033[0m"
                _gai_reset
                eval "$(gaiety init)"
                echo "\033[1m\033[32m✓ \033[0mgaiety reloaded"
            fi
            ;;
        browse)
            local out name path
            out=$(gaiety browse)
            [[ -z "$out" ]] && return 0
            name="${out%%$'\t'*}"
            path="${out##*$'\t'}"
            gai reload "$name" "$path"
            ;;
        list|info|new|rm|rename)
            gaiety "$@"
            ;;
        *)
            echo "Usage: gai [reload [<name>]|browse|list|info <name>|new <name>|rm <name>|rename <old> <new>]"
            ;;
    esac
}
