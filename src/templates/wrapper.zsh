# ZRT Shell Wrapper
zrt() {
    case "$1" in
        reload)
            echo "\033[1m\033[34m=> \033[0m\033[2m[zrt] running reset and reloading modules...\033[0m"
            _zrt_reset
            eval "$(zrt-loader init)"
            echo "\033[1m\033[32m✓ \033[0mzrt reloaded"
            ;;
        list|info|new|rm)
            zrt-loader "$@"
            ;;
        *)
            echo "Usage: zrt [reload|list|info <name>|new <name>|rm <name>]"
            ;;
    esac
}
