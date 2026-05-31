pub mod init;
pub mod list;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "zrt-loader", version, about = "Zsh Runtime Module Loader")]
pub struct Cli {
    #[arg(short, long, global = true, default_value = ".")]
    pub dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init,
    List,
}

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run(cli.dir),
        Commands::List => list::run(cli.dir),
    }
}
