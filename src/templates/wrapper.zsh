gai() {
    case "$1" in
        reload)
            echo "\033[1m\033[34m=> \033[0m\033[2m[gai] running reset and reloading modules...\033[0m"
            _gai_reset
            eval "$(gaiety init)"
            echo "\033[1m\033[32m✓ \033[0mgaiety reloaded"
            ;;
        browse)
            local out
            out=$(gaiety browse)
            [[ "$out" == "reload" ]] && gai reload
            ;;
        list|info|new|rm|rename)
            gaiety "$@"
            ;;
        *)
            echo "Usage: gai [reload|browse|list|info <name>|new <name>|rm <name>|rename <old> <new>]"
            ;;
    esac
}
