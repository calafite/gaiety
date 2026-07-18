use crate::core::loader::Loader;
use crate::validator::commands::CommandValidator;
use anyhow::Result;
use colored::Colorize;

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    for warning in CommandValidator::comps(&modules) {
        eprintln!("{} {}", "warn:".bold().yellow(), warning);
    }

    let zsh_code = loader.generate_init(&modules)?;
    print!("{}", zsh_code);
    Ok(())
}
