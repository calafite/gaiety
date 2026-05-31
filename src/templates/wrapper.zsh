gai() {
    case "$1" in
        reload)
            if [[ -n "$2" ]]; then
                local mod_path="$3"
                
                if [[ -z "$mod_path" ]]; then
                    mod_path=$(gaiety path "$2" 2>/dev/null)
                fi
              
                if [[ -z "$mod_path" || ! -f "$mod_path" ]]; then
                    echo "\033[1;31m!\033[0m module not found or missing init.zsh: $2"
                    return 1
                fi
                
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] reloading module $2...\033[0m"
                source "$mod_path"
                echo "\033[1m\033[32m✓ \033[0mreloaded $2"
            else
                echo "\033[1m\033[34m=> \033[0m\033[2m[gai] running reset and reloading modules...\033[0m"
                _gai_reset
                eval "$(gaiety init)"
                echo "\033[1m\033[32m✓ \033[0mgaiety reloaded"
            fi
            ;;
        browse)
            local out name mod_path
            out=$(gaiety browse)
            [[ -z "$out" ]] && return 0
            
            name="${out%%$'\t'*}"
            mod_path="${out##*$'\t'}"
            
            gai reload "$name" "$mod_path"
            ;;
        list|info|new|rm|rename)
            gaiety "$@"
            ;;
        *)
            echo "Usage: gai [reload [<name>]|browse|list|info <name>|new <name>|rm <name>|rename <old> <new>]"
            ;;
    esac
}
