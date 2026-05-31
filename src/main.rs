mod loader;
mod manifest;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "zrt-loader", version, about = "Zsh Runtime Module Loader")]
struct Cli {
    #[arg(short, long, global = true, default_value = ".")]
    dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut loader = loader::Loader::new(cli.dir);

    match cli.command {
        Commands::Init => {
            let zsh_code = loader.generate_init()?;
            print!("{}", zsh_code);
        }
        Commands::List => {
            loader.print_list()?;
        }
    }

    Ok(())
}
