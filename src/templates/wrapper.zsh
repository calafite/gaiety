gai() {
    local _gai_cache="${GAI_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/gaiety/init.zsh}"

    case "$1" in
        reload)
            if [[ -n "$2" ]]; then
                local mod_path="$3"

                if [[ -z "$mod_path" ]]; then
                    mod_path=$({{GAIETY_BIN}} path "$2" 2>/dev/null)
                fi

                if [[ -z "$mod_path" || ! -f "$mod_path" ]]; then
                    echo "\033[1;31m!\033[0m module not found or missing init.zsh: $2"
                    return 1
                fi

                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] reloading module $2...\033[0m"

                local _gai_mod_reset="_gai_reset_$2"
                if typeset -f "$_gai_mod_reset" > /dev/null 2>&1; then
                    "$_gai_mod_reset"
                fi

                source "$mod_path"
                echo "\033[1m\033[32m✓ \033[0mreloaded $2"
            else
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] syncing and reloading modules...\033[0m"
                _gai_reset
                {{GAIETY_BIN}} sync && source "$_gai_cache"
            fi
            ;;
        sync)
            {{GAIETY_BIN}} sync "${@:2}"
            ;;
        browse)
            local out name mod_path
            out=$({{GAIETY_BIN}} browse)
            [[ -z "$out" ]] && return 0

            name="${out%%$'\t'*}"
            mod_path="${out##*$'\t'}"

            gai reload "$name" "$mod_path"
            ;;
        install)
            {{GAIETY_BIN}} "$@" || return $?
            echo ""
            echo "\033[1m\033[34m=> \033[0m\033[2mRun 'gai reload' to load the new module now.\033[0m"
            ;;
        update)
            {{GAIETY_BIN}} "$@"
            local _gai_exit=$?
            if [[ $_gai_exit -eq 0 ]]; then
                echo "\033[1m\033[34m=> \033[0m\033[2mRun 'gai reload' to apply any updates.\033[0m"
            fi
            return $_gai_exit
            ;;
        list|info|new|rm|rename|profile|path)
            {{GAIETY_BIN}} "$@"
            ;;
        *)
            echo "Usage: gai <command> [args]"
            echo ""
            echo "Commands:"
            echo "  reload [<name>]            Reload all modules, or just <name>"
            echo "  sync                       Write the init script to the cache file"
            echo "  browse                     Browse modules interactively (requires fzf)"
            echo "  list                       List all modules and their status"
            echo "  info <name>                Show metadata and public API for a module"
            echo "  path <name>                Print the path to a module's init.zsh"
            echo "  new <name>                 Scaffold a new module"
            echo "  install <spec>             Install a module from a git repository"
            echo "  update [<name>]            Update installed module(s)"
            echo "  rm <name>                  Remove a module"
            echo "  rename <old> <new>         Rename a module"
            echo "  profile                    Benchmark module load times"
            ;;
    esac
}
