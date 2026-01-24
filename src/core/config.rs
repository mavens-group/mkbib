use crate::core::keygen::KeyGenConfig;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

fn get_config_path() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "mkbib", "mkbib-rs") {
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = fs::create_dir_all(config_dir);
        }
        return Some(config_dir.join("config.toml"));
    }
    None
}

pub fn save(config: &KeyGenConfig) {
    if let Some(path) = get_config_path() {
        if let Ok(toml_str) = toml::to_string_pretty(config) {
            let _ = fs::write(path, toml_str);
        }
    }
}

pub fn load() -> KeyGenConfig {
    if let Some(path) = get_config_path() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(cfg) = toml::from_str(&content) {
                return cfg;
            }
        }
    }
    KeyGenConfig::default()
}
