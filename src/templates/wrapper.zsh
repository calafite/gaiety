gai() {
    local _gai_cache="${GAI_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/gaiety/init.zsh}"
    local _gai_dir="${_gai_cache:h}"

    case "$1" in
        reload|browse)
            local cmd
            cmd=$(lua "$_gai_dir/wrapper.lua" "$@") || return $?
            if [[ -n "$cmd" ]]; then
                eval "$cmd"
            fi
            ;;
        *)
            lua "$_gai_dir/wrapper.lua" "$@"
            ;;
    esac
}
