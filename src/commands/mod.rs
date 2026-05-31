pub mod browse;
pub mod info;
pub mod init;
pub mod list;
pub mod new;
pub mod rm;
pub mod rename;

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
    /// Generate and emit the Zsh initialization script
    Init,
    /// Browse modules interactively (requires fzf)
    Browse,
    /// List all modules and their current status
    List,
    /// View detailed metadata and API for a module
    Info {
        /// Name of the module
        module: String,
    },
    /// Create a new module from templates
    New {
        /// Name of the new module
        module: String,
        /// Directory to create the module in (defaults to the last in GAI_DIRS)
        #[arg(short, long)]
        target: Option<PathBuf>,
    },
    /// Remove a module and renumber remaining modules in its directory
    Rm {
        /// Name of the module to remove
        module: String,
        /// Only remove the module if it lives in this directory.
        /// Useful when the same module name exists in multiple GAI_DIRS entries.
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Rename a module
    Rename {
        /// Current module name
        old: String,
        /// New module name
        new: String,
    },
}

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run(cli.dirs),
        Commands::Browse => browse::run(cli.dirs),
        Commands::List => list::run(cli.dirs),
        Commands::Info { module } => info::run(cli.dirs, module),
        Commands::New { module, target } => new::run(cli.dirs, module, target),
        Commands::Rm { module, dir } => rm::run(cli.dirs, module, dir),
        Commands::Rename { old, new } => rename::run(cli.dirs, old, new),
    }
}
