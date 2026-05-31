# gaiety

Minimalist zsh runtime module loader. Write modular shell config, declare dependencies and public APIs in manifests. Gaiety handles load order, validation and cleanup.

---

### Shell setup

In `.zshrc`:

```zsh
export GAI_DIRS="/usr/share/gaiety/modules:~/.config/gaiety/modules"
eval "$(gai init)"
```

`GAI_DIRS` is colon-separated. Directories load left to right. Put system-wide modules first, personal ones last. The last directory is the default target for `gai new` and `gai rm`.

---

## Module structure

Each module is a directory with two files:

```
~/.config/gaiety/modules/
  01_list/
    module.toml
    init.zsh
  02_comp/
    module.toml
    init.zsh
```

The numeric prefix controls load order. Gaps are fine, as gaiety renumbers automatically on removal.

### module.toml

```toml
[module]
name        = "list"
description = "directory listing with eza/exa"
version     = "1.0.0"
deps        = []
# deps = [{ name = "other_module", version = ">=1.0.0" }]
tags        = []

# skip if any of these binaries are unavailable
requires_cmd     = []

# skip if none of these binaries are available
requires_any_cmd = ["eza", "exa"]

[api]
# functions this module exposes ~ unloaded on reload
functions  = ["ls", "ll", "la", "lt", "lta", "help_ls"]

# variables this module sets ~ unset on reload
variables  = []

# aliases = { top = "btop" }
# completions = { "lt" = "_rt_comp_dirs" }
```

### init.zsh

Plain zsh. The manifest declares what it exposes, the script implements it. Prefix internal functions with `_modulename_` to avoid collisions.

```zsh
_list_ls() { eza --icons --group-directories-first "$@"; }

ls() { _list_ls "$@"; }
ll() { eza -lh --icons --group-directories-first "$@"; }
```

---

## Commands

```
gai init                    emit the zsh initialization script
gai list                    list all modules and their status
gai info <name>             show metadata and public api for a module
gai new <name>              scaffold a new module from templates
gai rename <old> <new>      rename a module and update all dependents
gai rm <name>               remove a module and renumber remaining
```

### gai list

```
:: Module Registry

  list            loaded    v1.0.0   deps:[]
  zoxide          skipped   v1.0.0   deps:[]
    ↳ none of these commands found: zoxide
```

### gai new

Creates a numbered directory with `module.toml` and `init.zsh` from templates. Prefix is assigned automatically.

```zsh
gai new mything
# creates: 03_mything/module.toml
#          03_mything/init.zsh
```

Use `--target` to write to a specific directory:

```zsh
gai new mything --target /usr/share/gaiety/modules
```

### gai rename

Renames the module directory, updates its `module.toml`, and rewrites any `deps` references across all modules.

```zsh
gai rename oldname newname
```

### gai rm

Prompts for confirmation, deletes the directory, and renumbers remaining modules.

```zsh
gai rm mything
# ? remove module 'mything'? [y/N]
```

---

## Module status

A module is skipped if:

- `requires_cmd` ~ one of the listed commands is missing from `PATH`
- `requires_any_cmd` ~ none of the listed commands are in `PATH`
- `deps` ~ a dependency was not loaded (cascades)

Skipped modules show up in `gai list` with a reason. Their `init.zsh` is not sourced and their API is not registered.

---

## Multiple directories

Modules across all `GAI_DIRS` are treated as a single unified registry: nique names, unique prefixes, shared dependency graph. If the same module name exists in multiple directories, the last directory wins.

---

## Reload

```zsh
gai reload
```

Calls `_gai_reset` (unsets all registered functions, variables and aliases), then re-evals `gai init`.

---

## Completions

```toml
[api]
completions = { "lt" = "_rt_comp_dirs" }
```

The function must exist somewhere in a loaded `init.zsh`, gaiety warns at load time if it can't find it.

```zsh
# directories only
_rt_comp_dirs() { _path_files -/; }

# files and directories
_rt_comp_paths() { _path_files; }
```

---
