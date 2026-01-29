// src/core/mod.rs
pub mod config;
pub mod keygen;

use biblatex::{Chunk, Spanned};

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

// NEW HELPER
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}
