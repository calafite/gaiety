# gaiety

Minimalist zsh runtime module loader. Write modular shell config, declare dependencies and public APIs in manifests. Gaiety handles load order, validation and cleanup.

---

### Shell setup

In `.zshrc`:

```zsh
export GAI_DIRS="/usr/share/gaiety/modules:~/.config/gaiety/modules"
export GAI_CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/gaiety/init.zsh"
[[ -f "$GAI_CACHE" ]] || gaiety sync
source "$GAI_CACHE"
```

`GAI_DIRS` is colon-separated. Directories load left to right. Put system-wide modules first, personal ones last. The last directory is the default target for `gai new` and `gai rm`.

`gaiety sync` writes the init script to `$GAI_CACHE` once and automatically background-compiles (`zcompile`) all scripts for performance. After that, startup is just a `source`. Run `gai reload` after adding or editing modules, it resyncs and re-sources automatically.

---

## Module structure

Each module is a directory with two files:

```text
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

# defer sourcing until first use ~ see lazy loading below
defer_on_cmd = false

# managed by gai install ~ do not edit manually
# [source]
# url    = "https://github.com/user/repo.git"
# branch = "main"
# pin    = "abcdef1"
```

### Version constraints

Dep entries accept standard semver operators:

```text
=1.2.3          exact
>=1.2.3         at least
>1.2.3          strictly greater
<=1.2.3         at most
<1.2.3          strictly less
~1.2.3          patch-compatible  (>=1.2.3, <1.3.0)
^1.2.3          minor-compatible  (>=1.2.3, <2.0.0)
>=1.0, <2.0     compound (comma-separated)
```

Short versions are accepted: `1` and `1.2` are treated as `1.0.0` and `1.2.0`. Pre-release versions (`1.0.0-alpha`) are supported in both `version` and constraints. Note that `>=1.0.0` does not match `1.0.1-alpha` ~ to match a pre-release the constraint must include one: `>=1.0.0-alpha`.

### init.zsh

Plain zsh. The manifest declares what it exposes, the script implements it. Prefix internal functions with `_modulename_` to avoid collisions.

```zsh
# internal implementation
_list_ls() { eza --icons --group-directories-first "$@"; }

# public api
ls() { _list_ls "$@"; }
ll() { eza -lh --icons --group-directories-first "$@"; }
```

---

## Commands

```text
gaiety init                 emit the zsh initialization script
gai list                    list all modules and their status
gai info <name>             show metadata and public api
gai new <name>              scaffold a new module
gai rename <old> <new>      rename a module and update dependents
gai rm <name>               remove a module
gai install <spec>          install from a git repository
gai update [<name>]         pull updates for installed module(s)
gai reload [<name>]         reload all modules, or just <name>
gai sync                    write the init script to the cache file
gai browse                  browse modules interactively (requires fzf)
gai profile                 benchmark module load times
gai path <name>             print the path to a module's init.zsh
```

### gai list

```text
:: Module Registry

  list            loaded    v1.0.0   deps:[]
  zoxide          lazy      v1.0.0   deps:[]
  skim            skipped   v1.0.0   deps:[]
    ↳ none of these commands found: sk
```

Modules with `defer_on_cmd = true` show as `lazy` rather than `loaded`.

### gai info

Shows full metadata for a single module, including whether it is lazy-loaded.

```text
:: Module: list

  status         loaded
  file           01_list
  path           ~/.config/gaiety/modules/01_list
  desc           directory listing with eza/exa
  version        1.0.0
  deps           -
  tags           -
  lazy           no

  Public API
    functions:
      ls
      ll
      la
      lt
      lta
      help_ls
```

A lazy module shows `lazy  yes` and its `init.zsh` is not sourced until one of its declared functions is first called.

### gai browse

Interactive fuzzy finder for your modules ~ requires `fzf`.
Shows module status, version, and metadata. Hit enter to instantly reload the selected module in your current session.

```zsh
gai browse
```

### gai install

Downloads a module from a git repository. Generates the `module.toml` and `init.zsh` wrapper automatically. If the repository contains multiple modules (a collection), all valid modules inside it will be installed.

```zsh
# github shorthand
gai install zsh-users/zsh-autosuggestions

# specific branch
gai install zsh-users/zsh-syntax-highlighting@develop

# full url
gai install https://gitlab.com/user/repo.git
```

Accepts `--name` to override the derived module name, `--branch` to explicitly target a branch, and `--target` to place the module in a specific `GAI_DIRS` directory.

### gai update

Pulls updates for all installed modules that have a `[source]` block in their manifest.

```zsh
# update all managed modules
gai update

# update a specific module
gai update zsh_autosuggestions
```

### gai new

Creates a numbered directory with `module.toml` and `init.zsh` from templates. Prefix is assigned automatically.

```zsh
gai new mything
# creates: 03_mything/module.toml
#          03_mything/init.zsh
```

New modules are created with `defer_on_cmd = true` by default. Remove or set to `false` if the module needs to run code at shell startup (e.g. setting variables, running `eval "$(tool init zsh)"`).

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
# ? remove module 'mything'? [y/n]
```

Use `--dir` to restrict the search to a specific directory if the same module name exists in multiple locations.

### gai profile

Benchmarks the source time of every loaded module by running each `init.zsh` in an isolated zsh subprocess and reporting elapsed time.

```text
:: Module Load Profile

Module                 Time (ms)  Relative
────────────────────────────────────────────────────
zoxide                  12.431 ms  ████████████████████
list                     4.209 ms  ███████
skim                     1.876 ms  ███
fzf_fm                   0.341 ms  █
────────────────────────────────────────────────────
Total                   18.857 ms
```

Times are color-coded: green under 1 ms, yellow 1-5 ms, red above 5 ms. Lazy modules are shown in blue with a `(def)` suffix, their time reflects the one-time cost paid on first use, not at shell startup.

Use `gai profile` to identify slow modules and decide which ones to make lazy.

---

## Lazy loading

Setting `defer_on_cmd = true` in `[api]` tells gaiety not to source the module's `init.zsh` at shell startup. Instead, thin stub functions are generated for every name in `functions`, `aliases`, and `completions`. The first time any of those names is invoked, the real `init.zsh` is sourced and the call is forwarded transparently.

```toml
[api]
functions    = ["ls", "ll", "la"]
defer_on_cmd = true
```

This is safe for modules that only define functions or aliases. It is **not** appropriate for modules that need to run code eagerly at startup; for example, modules that export variables, call `eval "$(tool init zsh)"`, or set up keybindings. Those modules should use `defer_on_cmd = false`.

`gai list` shows lazy modules with the status `lazy`. `gai info <name>` shows `lazy  yes` in the metadata block. `gai profile` marks lazy modules with `(def)` and reports their deferred source time.

---

## Module status

A module is skipped if:

* `requires_cmd` ~ one of the listed commands is missing from `PATH`
* `requires_any_cmd` ~ none of the listed commands are in `PATH`
* `deps` ~ a dependency was not loaded (cascades)

Skipped modules show up in `gai list` with a reason. Their `init.zsh` is not sourced and their API is not registered.

---

## Multiple directories

Modules across all `GAI_DIRS` are treated as a single unified registry: unique names, unique prefixes, shared dependency graph. If the same module name exists in multiple directories, the last directory wins.

---

## Reload

You can reload the entire registry or just target a single module if you're actively working on it.

```zsh
# reload all modules
gai reload

# reload just one module
gai reload mything
```

When reloading everything, gaiety calls `_gai_reset` (unsets all registered functions, variables and aliases), then re-evaluates `gaiety init`. When reloading a single module, it just sources that module's `init.zsh` directly.

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
