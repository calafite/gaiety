use crate::loader::Loader;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(dir: PathBuf) -> Result<()> {
    let loader = Loader::new(dir);
    let zsh_code = loader.generate_init()?;
    print!("{}", zsh_code);
    Ok(())
}
