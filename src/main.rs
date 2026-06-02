mod cli;
mod commands;
mod loader;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::execute(cli)
}
