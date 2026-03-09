// src/core/mod.rs
pub mod config;
pub mod keygen;

use biblatex::{Chunk, Spanned};
use std::path::Path;

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

pub fn create_backup(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        let mut backup_path = path.to_path_buf();
        if let Some(stem) = path.file_stem() {
            let mut new_name = stem.to_os_string();
            new_name.push(".bib.bak"); // Creates trial.bib.bak
            backup_path.set_file_name(new_name);
        } else {
            // Fallback
            backup_path.set_extension("bak");
        }

        std::fs::copy(path, backup_path)?;
    }
    Ok(())
}
