// src/logic/formatter.rs

use crate::core::keygen::KeyGenConfig;
use biblatex::Entry;
use std::fmt::Write;

pub fn format_entry(entry: &Entry, config: &KeyGenConfig) -> String {
    let mut out = String::new();

    // 1. Indentation
    let indent = if config.indent_char == '\t' {
        "\t".repeat(config.indent_width as usize)
    } else {
        " ".repeat(config.indent_width as usize)
    };

    // 2. Header
    let _ = writeln!(out, "@{}{{{},", entry.entry_type.to_bibtex(), entry.key);

    // 3. Fields
    let mut written_fields = std::collections::HashSet::new();

    // Priority Fields
    for key in &config.field_order {
        if let Some(chunks) = entry.fields.get(key) {
            write_field(&mut out, key, chunks, &indent);
            written_fields.insert(key.clone());
        }
    }

    // Remaining Fields
    let mut remaining_keys: Vec<_> = entry.fields.keys().collect();
    remaining_keys.sort();

    for key in remaining_keys {
        if !written_fields.contains(key) {
            let chunks = entry.fields.get(key).unwrap();
            write_field(&mut out, key, chunks, &indent);
        }
    }

    // 4. Footer
    out.push('}');

    // ✅ STRICT FIX: Trim start and end.
    // This removes any newline writeln! added after the last field
    // and ensures the block is tight.
    out.trim().to_string()
}

fn write_field(
    out: &mut String,
    key: &str,
    chunks: &[biblatex::Spanned<biblatex::Chunk>],
    indent: &str,
) {
    let mut s = String::new();
    for chunk in chunks {
        match &chunk.v {
            biblatex::Chunk::Normal(t) => s.push_str(t),
            biblatex::Chunk::Verbatim(t) => s.push_str(t),
            biblatex::Chunk::Math(t) => s.push_str(t),
        }
    }
    // indent key = {value},
    let _ = writeln!(out, "{}{} = {{{}}},", indent, key, s);
}
