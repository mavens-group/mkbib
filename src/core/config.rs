use crate::core::keygen::KeyGenConfig;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

fn get_config_path() -> anyhow::Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "mkbib", "mkbib-rs")
        .ok_or_else(|| anyhow::anyhow!("could not determine the configuration directory"))?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir)?;
    Ok(config_dir.join("config.toml"))
}

pub fn save(config: &KeyGenConfig) -> anyhow::Result<()> {
    let path = get_config_path()?;
    let toml_str = toml::to_string_pretty(config)?;
    fs::write(path, toml_str)?;
    Ok(())
}

pub fn load() -> anyhow::Result<KeyGenConfig> {
    let path = get_config_path()?;
    if !path.exists() {
        return Ok(KeyGenConfig::default());
    }
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}
