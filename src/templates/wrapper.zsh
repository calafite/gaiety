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
        resolve)
            {{GAIETY_BIN}} "$@" || return $?
            echo ""
            echo "\033[1m\033[34m=> \033[0m\033[2mRun 'gai reload' to load any newly installed modules.\033[0m"
            ;;
        list|info|new|rm|rename|prune|profile|path)
            {{GAIETY_BIN}} "$@"
            ;;
        *)
            local c="\033[1;36m"   # bold cyan  — header / command names
            local g="\033[0;32m"   # green      — arguments
            local d="\033[2m"      # dim        — descriptions
            local b="\033[1;34m"   # bold blue  — hint prefix
            local r="\033[0m"      # reset

            echo ""
            echo "  ${c}:: Gaiety${r}  ${d}Zsh Runtime Module Loader${r}"
            echo ""
            echo "  ${d}Usage:${r}  ${c}gai${r} ${g}<command>${r} ${d}[args]${r}"
            echo ""
            echo "  ${d}Module management${r}"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "list"                "List all modules and their status"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "info <name>"         "Show metadata and public API"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "new <name>"          "Scaffold a new module"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "rename <old> <new>"  "Rename a module and update dependents"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "rm <name>"           "Remove a module"
            echo ""
            echo "  ${d}Remote packages${r}"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "install <spec>"      "Install from a git repository"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "update [<name>]"     "Pull updates for installed module(s)"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "resolve"             "Install missing remote dependencies"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "prune"               "Remove unused implicit dependencies"
            echo ""
            echo "  ${d}Runtime${r}"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "reload [<name>]"     "Reload all modules, or just <name>"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "sync"                "Write the init script to the cache file"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "browse"              "Browse modules interactively (requires fzf)"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "profile"             "Benchmark module load times"
            printf "    ${c}%-22s${r} ${d}%s${r}\n" "path <name>"         "Print the path to a module's init.zsh"
            echo ""
            echo "  ${b}=>${r}  ${d}Set \$GAI_DIRS to a colon-separated list of module directories.${r}"
            echo ""
            ;;
    esac
}
