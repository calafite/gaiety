mod commands;
mod loader;
mod manifest;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = commands::Cli::parse();
    commands::execute(cli)
}
