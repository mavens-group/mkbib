// src/core/mod.rs
pub mod config;
pub mod keygen;

use biblatex::{Chunk, Spanned};
use std::fs;
use std::path::{Path, PathBuf};

/// Helper to safely get string from a list of chunks
pub fn bib_to_string(val: &[Spanned<Chunk>]) -> String {
    val.iter()
        .map(|c| match &c.v {
            Chunk::Normal(s) => s.clone(),
            Chunk::Verbatim(s) => s.clone(),
            Chunk::Math(s) => s.clone(),
            // FIX: Removed unreachable "_ => ..." branch
        })
        .collect()
}

pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

pub fn create_backup(file_path: &Path) -> std::io::Result<PathBuf> {
    let mut backup_path = file_path.to_path_buf();

    if let Some(extension) = backup_path.extension() {
        let mut ext_str = extension.to_os_string();
        ext_str.push(".bak");
        backup_path.set_extension(ext_str);
    } else {
        backup_path.set_extension("bak");
    }

    fs::copy(file_path, &backup_path)?;
    Ok(backup_path)
}
