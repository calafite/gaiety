mod cli;
mod commands;
mod core;
mod emitter;
mod resolver;
mod sources;
mod validator;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::execute(cli)
}
