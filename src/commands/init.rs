use crate::loader::Loader;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(dirs: String) -> Result<()> {
    let loader = Loader::new(&dirs)?;
    let zsh_code = loader.generate_init()?;
    print!("{}", zsh_code);
    Ok(())
}
