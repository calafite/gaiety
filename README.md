# gaiety

Minimalist zsh-based runtime module loader. Write modular shell configuration files, declare dependencies and public APIs in manifests. Gaiety handles load order, validation and cleanup.

---
### Shell setup

On `.zshrc`:

```zsh
export GAI_DIRS="/usr/share/gaiety/modules:~/.config/gaiety/modules"
eval "$(gai init)"
```

`GAI_DIRS` is colon-separated. Directories are loaded left to right. Put system-wide modules first, personal ones last. The last directory is used as the default target for `gai new` and `gai rm`.

---

## Module structure

Each module is a directory containing two files:

```
~/.config/gaiety/modules/
  01_list/
    module.toml
    init.zsh
  02_comp/
    module.toml
    init.zsh
```

The numeric prefix controls load order. Gaps are fine, as gaiety renumbers automatically when you remove a module.

### module.toml

```toml
[module]
name        = "list"
description = "Directory listing with eza/exa"
version     = "1.0.0"
deps        = []
tags        = []

# gaiety skips if ANY of these binaries are unavailable
requires_cmd     = []

# gaiety skips if ALL of these binaries are unavailable
requires_any_cmd = ["eza", "exa"]

[api]
# the functions this module exposes
# unloaded on reload
functions  = ["ls", "ll", "la", "lt", "lta", "help_ls"]

# variables this module sets (unset on reload)
variables  = []

# shell aliases (registered and unregistered automatically)
# aliases = { top = "btop" }

# completion bindings: { "command" = "_completion_fn" }
# completions = { "lt" = "_rt_comp_dirs" }
```

### init.zsh

Plain zsh scripts. The manifest declares what it exposes, whilst the script implements it.
It is convention to prefix internal functions with `_modulename_` to avoid name collisions.

```zsh
_list_ls() { eza --icons --group-directories-first "$@"; }

ls() { _list_ls "$@"; }
ll() { eza -lh --icons --group-directories-first "$@"; }
```

---

## Commands

```
gai init          Emit the Zsh initialization script (used in .zshrc)
gai list          List all modules and their status
gai info <name>   Show metadata and public API for a module
gai new <name>    Scaffold a new module from templates
gai rm <name>     Remove a module and renumber remaining
```

### gai list

```
:: Module Registry

  core            loaded    v1.0.0   deps:[]
  list            loaded    v1.0.0   deps:[core]
  zoxide          skipped   v1.0.0   deps:[core]
    ↳ none of these commands found: zoxide
```

### gai new

Creates a numbered directory with `module.toml` and `init.zsh` scaffolded from templates. The prefix is assigned automatically based on existing modules.

```zsh
gai new mything
# creates: 03_mything/module.toml
#          03_mything/init.zsh
```

Use `--target` to write to a specific directory instead of the default:

```zsh
gai new mything --target /usr/share/gaiety/modules
```

### gai rm

Prompts for confirmation, deletes the module directory, and renumbers remaining modules.

```zsh
gai rm mything
# ? Remove module 'mything'? [y/N]
```

---

## Module status

A module can be skipped at load time if:

- `requires_cmd` ~ one of the listed commands is not in `PATH`
- `requires_any_cmd` ~ none of the listed commands are in `PATH`
- `deps` ~ a declared dependency was not loaded (cascades)

Skipped modules are visible in `gai list` with a reason. Their `init.zsh` is not sourced and their API is not registered.

---

## Reload

```zsh
gai reload
```

Calls `_gai_reset` (unsets all registered functions, variables, and aliases), then re-evals `gai init`. Defined in the wrapper sourced by `eval "$(gai init)"`.

---

## Completions

To bind a completion function to a command, add it to `module.toml`:

```toml
[api]
completions = { "lt" = "_rt_comp_dirs" }
```

The function must be defined somewhere in a loaded `init.zsh`. gaiety will warn at load time if it can't find the function.

Common completions:

```zsh
# directories only
_rt_comp_dirs() { _path_files -/; }

# files and directories
_rt_comp_paths() { _path_files; }
```

---

## Tips

- Module names must match `[a-zA-Z_][a-zA-Z0-9_]*`
- Use `requires_any_cmd` for modules with multiple binary options (e.g. `eza`/`exa`)
- Keep internal functions prefixed. Public API is what goes in `functions` in the manifest
- `gai info <name>` is useful for debugging what a module actually exposes
