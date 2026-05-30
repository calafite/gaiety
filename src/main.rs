mod loader;
mod manifest;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "zrt-loader", version, about = "Zsh Runtime Module Loader")]
struct Cli {
    /// Directory containing the modules
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let loader = loader::Loader::new(cli.dir);
    loader.run()?;

    Ok(())
}
