use crate::commands::{browse, info, init, install, list, new, path, profile, rename, rm, sync, update};
use anyhow::Result;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .placeholder(AnsiColor::Yellow.on_default())
}

#[derive(Parser, Debug)]
#[command(
    name = "gaiety",
    version,
    about = "Zsh Runtime Module Loader",
    styles = styles()
)]
pub struct Cli {
    /// Colon-separated list of module directories
    #[arg(short, long, global = true, env = "GAI_DIRS", default_value = ".")]
    pub dirs: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Emit the zsh initialization script to stdout
    Init,

    /// Write the initialization script to the cache file
    Sync {
        /// Override the default cache path (defaults to $GAI_CACHE or ~/.cache/gaiety/init.zsh)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Browse modules interactively with fzf; selecting a module reloads it
    Browse,

    /// List all modules and their status
    List,

    /// Show metadata and public API for a module
    Info {
        /// Name of the module to inspect
        module: String,
    },

    /// Print the absolute path to a module's init.zsh
    Path {
        /// Name of the module
        module: String,
    },

    /// Install a module from a git repository
    Install {
        /// Repository spec: user/repo, user/repo@branch, github:user/repo,
        /// gitlab:user/repo, or a full https URL
        spec: String,

        /// Override the module name derived from the repository name
        #[arg(short, long)]
        name: Option<String>,

        /// Branch to clone (overrides any inline @branch in the spec)
        #[arg(short, long)]
        branch: Option<String>,

        /// Write the module to this directory instead of the default write dir
        #[arg(short, long)]
        target: Option<PathBuf>,
    },

    /// Update installed module(s) from their git remotes
    Update {
        /// Update only this module; defaults to all modules that have a [source] section
        module: Option<String>,
    },

    /// Scaffold a new module from the built-in templates
    New {
        /// Name for the new module (must match [a-zA-Z_][a-zA-Z0-9_]*)
        module: String,
        /// Write the new module to this directory instead of the default write dir
        #[arg(short, long)]
        target: Option<PathBuf>,
    },

    /// Remove a module and renumber the remaining modules in its directory
    Rm {
        /// Name of the module to remove
        module: String,
        /// Restrict the search to this directory (useful when the same name
        /// exists in multiple GAI_DIRS entries)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Rename a module, update its manifest, and rewrite all dependents
    Rename {
        /// Current module name
        old: String,
        /// New module name
        new: String,
    },

    /// Benchmark the source time of every loaded module
    Profile,
}

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run(cli.dirs),
        Commands::Sync { output } => sync::run(cli.dirs, output),
        Commands::Browse => browse::run(cli.dirs),
        Commands::List => list::run(cli.dirs),
        Commands::Info { module } => info::run(cli.dirs, module),
        Commands::Path { module } => path::run(cli.dirs, module),
        Commands::Install { spec, name, branch, target } => {
            install::run(cli.dirs, spec, name, branch, target)
        }
        Commands::Update { module } => update::run(cli.dirs, module),
        Commands::New { module, target } => new::run(cli.dirs, module, target),
        Commands::Rm { module, dir } => rm::run(cli.dirs, module, dir),
        Commands::Rename { old, new } => rename::run(cli.dirs, old, new),
        Commands::Profile => profile::run(cli.dirs),
    }
}
