mod cli;
mod commands;
mod core;
mod sources;
mod resolver;
mod validator;
mod emitter;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::execute(cli)
}
