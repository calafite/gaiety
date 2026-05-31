gai() {
    case "$1" in
        reload)
            echo "\033[1m\033[34m=> \033[0m\033[2m[gai] running reset and reloading modules...\033[0m"
            _gai_reset
            eval "$(gaiety init)"
            echo "\033[1m\033[32m✓ \033[0mgaiety reloaded"
            ;;
        list|info|new|rm)
            gaiety "$@"
            ;;
        *)
            echo "Usage: gai [reload|list|info <name>|new <name>|rm <name>]"
            ;;
    esac
}
