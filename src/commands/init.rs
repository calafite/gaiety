use crate::core::Loader;
use crate::validator::check_completions;
use anyhow::Result;
use colored::Colorize;

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let modules = loader.get_modules()?;

    for warning in check_completions(&modules) {
        eprintln!("{} {}", "warn:".bold().yellow(), warning);
    }

    let zsh_code = loader.generate_init_from(&modules)?;
    print!("{}", zsh_code);
    Ok(())
}
