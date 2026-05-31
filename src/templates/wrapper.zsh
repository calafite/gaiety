gai() {
    case "$1" in
        reload)
            if [[ -n "$2" ]]; then
                local path
                path=$(gaiety path "$2") || return 1
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] reloading module $2...\033[0m"
                source "$path"
                echo "\033[1m\033[32m✓ \033[0mreloaded $2"
            else
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] running reset and reloading modules...\033[0m"
                _gai_reset
                eval "$(gaiety init)"
                echo "\033[1m\033[32m✓ \033[0mgaiety reloaded"
            fi
            ;;
        browse)
            local name
            name=$(gaiety browse)
            [[ -n "$name" ]] && gai reload "$name"
            ;;
        list|info|new|rm|rename)
            gaiety "$@"
            ;;
        *)
            echo "Usage: gai [reload [<name>]|browse|list|info <name>|new <name>|rm <name>|rename <old> <new>]"
            ;;
    esac
}
