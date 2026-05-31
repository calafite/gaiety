pub mod info;
pub mod init;
pub mod list;
pub mod new;
pub mod rm;

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
    name = "zrt-loader",
    version,
    about = "Zsh Runtime Module Loader",
    styles = styles()
)]
pub struct Cli {
    /// Directory containing the modules
    #[arg(short, long, global = true, default_value = ".")]
    pub dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate and emit the Zsh initialization script
    Init,
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
    },
    /// Remove a module and its directory
    Rm {
        /// Name of the module to remove
        module: String,
    },
}

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run(cli.dir),
        Commands::Li
