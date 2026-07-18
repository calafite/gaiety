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

    let cache_path = crate::commands::sync::default_cache_path();
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
        let lua_path = parent.join("wrapper.lua");
        let bin = &crate::core::common::exe_path();
        let lua_code = include_str!("../templates/wrapper.lua").replace("{{GAIETY_BIN}}", bin);
        let _ = std::fs::write(&lua_path, &lua_code);
    }

    print!("{}", zsh_code);
    Ok(())
}
