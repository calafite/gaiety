local c  = "\27[1;36m"   -- bold cyan
local g  = "\27[0;32m"   -- green
local d  = "\27[2m"      -- dim
local b  = "\27[1;34m"   -- bold blue
local r  = "\27[0m"      -- reset
local br = "\27[1;31m"   -- bold red
local empty = ""

local SH_RELOAD_MOD = [=[
printf '\033[1;34m=> \033[0m\033[2m[gai] reloading module %%s...\033[0m\n' %s
if typeset -f _gai_reset_%s > /dev/null 2>&1; then
    _gai_reset_%s
fi
source %s
printf '\033[1;32m✓ \033[0mreloaded %%s\n' %s
]=]

local SH_RELOAD_ALL = [=[
printf '\033[1;34m=> \033[0m\033[2m[gai] syncing and reloading modules...\033[0m\n'
if typeset -f _gai_reset > /dev/null 2>&1; then
    _gai_reset
fi
%s sync && source %s
]=]

local GAIETY_BIN = "{{GAIETY_BIN}}"

local function title(name, desc)
    return string.format("  %s:: %s%s  %s%s%s", c, name, r, d, desc, r)
end

local function usage(cmd, args)
    return string.format("  %sUsage:%s  %sgai%s %s%s%s %s%s%s", d, r, c, r, g, cmd, r, d, args, r)
end

local function section(name)
    return string.format("  %s%s%s", d, name, r)
end

local function entry(cmd, desc)
    return string.format("    %s%-22s%s %s%s%s", c, cmd, r, d, desc, r)
end

local function info(text)
    return string.format("  %s=>%s  %s%s%s", b, r, d, text, r)
end

local function success(text, pad)
    local prefix = (pad and "\n" or empty) .. b .. "=> " .. r .. d
    return prefix .. text .. r .. "\n"
end

-- Prints help/usage documentation.
local function print_help()
    local help_text = {
        empty,
        title("Gaiety", "Zsh Runtime Module Loader"),
        empty,
        usage("<command>", "[args]"),
        empty,
        section("Module management"),
        entry("list", "List all modules and their status"),
        entry("info <name>", "Show metadata and public API"),
        entry("new <name>", "Scaffold a new module"),
        entry("rename <old> <new>", "Rename a module and update dependents"),
        entry("rm <name>", "Remove a module"),
        empty,
        section("Remote packages"),
        entry("install <spec>", "Install from a git repository"),
        entry("update [<name>]", "Pull updates for installed module(s)"),
        entry("resolve", "Install missing remote dependencies"),
        entry("prune", "Remove unused implicit dependencies"),
        empty,
        section("Runtime"),
        entry("reload [<name>]", "Reload all modules, or just <name>"),
        entry("sync", "Write the init script to the cache file"),
        entry("browse", "Browse modules interactively (requires fzf)"),
        entry("profile", "Benchmark module load times"),
        entry("path <name>", "Print the path to a module's init.zsh"),
        empty,
        info("Set $GAI_DIRS to a colon-separated list of module directories."),
        empty
    }
    print(table.concat(help_text, "\n"))
end

-- Safely quotes a string for POSIX-compliant shell execution.
local function quote(s)
    if not s then return "''" end
    return "'" .. string.gsub(tostring(s), "'", "'\\''") .. "'"
end

-- Formats the Gaiety binary command path, quoting it if needed.
local function bin_cmd()
    if GAIETY_BIN:sub(1, 1) == '"' or GAIETY_BIN:sub(1, 1) == "'" then
        return GAIETY_BIN
    elseif GAIETY_BIN:find(" ") then
        return quote(GAIETY_BIN)
    else
        return GAIETY_BIN
    end
end

-- Checks if a file exists at the given path.
local function has_file(path)
    local f = io.open(path, "r")
    if f then
        f:close()
        return true
    end
    return false
end

-- Resolves the Zsh cache init script path.
local function cache_path()
    local gai_cache = os.getenv("GAI_CACHE")
    if gai_cache and gai_cache ~= empty then
        return gai_cache
    end

    local base = os.getenv("XDG_CACHE_HOME")
    if not base or base == empty then
        local home = os.getenv("HOME") or empty
        base = home ~= empty and (home .. "/.cache") or ".cache"
    end
    return base .. "/gaiety/init.zsh"
end

-- Executes a command and returns its standard output.
local function exec_out(cmd)
    local handle = io.popen(cmd)
    if not handle then return empty end
    local result = handle:read("*a")
    handle:close()
    return result or empty
end

-- Runs a command via os.execute, compatible across Lua versions.
local function run_bin(args)
    local quoted = {}
    for _, val in ipairs(args) do
        table.insert(quoted, quote(val))
    end
    local cmd = bin_cmd() .. " " .. table.concat(quoted, " ")
    local ok, _, code = os.execute(cmd)

    if ok == true then
        return 0
    elseif ok == false or ok == nil then
        return code or 1
    else
        return ok
    end
end

-- Prints the reload script for a single module.
local function show_reload(name, path)
    local safe_name = name:gsub("[^%w_]", "_")
    local script = string.format(
        SH_RELOAD_MOD,
        quote(name),
        safe_name,
        safe_name,
        quote(path),
        quote(name)
    )
    print(script)
end

-- Prints the reload script for all modules.
local function reload_all()
    local cache = cache_path()
    local script = string.format(SH_RELOAD_ALL, bin_cmd(), quote(cache))
    print(script)
end

-- Executes the browse dialog and handles selection output.
local function browse_mods()
    local out = exec_out(bin_cmd() .. " browse")
    if out and out ~= empty then
        out = out:gsub("%s+$", empty)
        local name, path = out:match("^([^\t]+)\t(.*)$")
        if name and path then
            show_reload(name, path)
        end
    end
end

-- CLI Dispatcher
local success_msgs = {
    install = success("Run 'gai reload' to load the new module now.", true),
    update  = success("Run 'gai reload' to apply any updates.", false),
    resolve = success("Run 'gai reload' to load any newly installed modules.", true),
}

local known_cmds = {
    sync = true, list = true, info = true, new = true,
    rm = true, rename = true, prune = true, profile = true,
    path = true, install = true, update = true, resolve = true
}

local args = arg or {}
local action = args[1]

if not action then
    print_help()
    os.exit(0)
end

if action == "reload" then
    local mod = args[2]
    if mod and mod ~= empty then
        local path = args[3]
        if not path or path == empty then
            path = exec_out(bin_cmd() .. " path " .. quote(mod)):gsub("%s+$", empty)
        end
        if path == empty or not has_file(path) then
            io.stderr:write(br .. "! " .. r .. "module not found or missing init.zsh: " .. mod .. "\n")
            os.exit(1)
        end
        show_reload(mod, path)
    else
        reload_all()
    end

elseif action == "browse" then
    browse_mods()

elseif known_cmds[action] then
    local code = run_bin(args)
    if code == 0 then
        local msg = success_msgs[action]
        if msg then
            io.write(msg)
        end
    end
    os.exit(code)

else
    print_help()
    os.exit(0)
end
