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
    #[arg(short, long, global = true, env = "GAI_DIRS", default_value = ".")]
    pub dirs: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init,

    Sync {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    Browse,

    List,

    Info {
        module: String,
    },

    Path {
        module: String,
    },

    Install {
        spec: String,

        #[arg(short, long)]
        name: Option<String>,

        #[arg(short, long)]
        branch: Option<String>,

        #[arg(short, long)]
        target: Option<PathBuf>,
    },

    Update {
        module: Option<String>,
    },

    New {
        module: String,
        #[arg(short, long)]
        target: Option<PathBuf>,
    },

    Rm {
        module: String,
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    Rename {
        old: String,
        new: String,
    },

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
