# ============================================================
# {{MODULE_NAME}}
# ============================================================

# Internal implementation (prefix with _{{MODULE_NAME}}_*)
_{{MODULE_NAME}}_help() {
    echo "\033[1;36m:: {{MODULE_NAME}} Module\033[0m"
    echo "Edit this help text in init.zsh"
}

# Public functions are mapped to internal ones here
# Do not use aliases for function mapping, use real functions:
help_{{MODULE_NAME}}() {
    _{{MODULE_NAME}}_help "$@"
}
