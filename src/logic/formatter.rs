// src/logic/formatter.rs

use crate::core::keygen::KeyGenConfig;
use biblatex::Entry;
use std::fmt::Write;

/// Formats an entry.
/// If `original_source` is provided, unedited fields are copied byte-for-byte.
pub fn format_entry(entry: &Entry, config: &KeyGenConfig, original_source: Option<&str>) -> String {
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
            write_field(&mut out, key, chunks, &indent, original_source);
            written_fields.insert(key.clone());
        }
    }

    // Remaining Fields
    let mut remaining_keys: Vec<_> = entry.fields.keys().collect();
    remaining_keys.sort();

    for key in remaining_keys {
        if !written_fields.contains(key) {
            let chunks = entry.fields.get(key).unwrap();
            write_field(&mut out, key, chunks, &indent, original_source);
        }
    }

    // 4. Footer
    out.push('}');

    // STRICT TRIM: Removes the trailing newline from the last field
    out.trim().to_string()
}

fn write_field(
    out: &mut String,
    key: &str,
    chunks: &[biblatex::Spanned<biblatex::Chunk>],
    indent: &str,
    original_source: Option<&str>,
) {
    let mut s = String::new();
    let mut appended_from_source = false;

    // HYBRID LOGIC: Recover original source formatting if possible
    if let Some(src) = original_source {
        if !chunks.is_empty() {
            // 1. Find the bounding box of the content
            let start = chunks[0].span.start;
            let end = chunks.last().unwrap().span.end;

            // 2. Validate bounds
            if start < end && end <= src.len() {
                // 3. Extract the "Core" content (e.g. "Test {DNA")
                let mut current_end = end;

                // ✅ FIX: Removed 'mut' from slice (warning fixed)
                let slice = src[start..current_end].to_string();

                // 4. BALANCE BRACES
                // We count '{' and '}' in the slice. If open > closed, we likely missed
                // the closing braces that are part of the value.
                let mut open_count = slice.chars().filter(|c| *c == '{').count();
                let mut close_count = slice.chars().filter(|c| *c == '}').count();

                // Expand rightwards to capture missing '}'
                while open_count > close_count && current_end < src.len() {
                    let next_char = src[current_end..].chars().next().unwrap();
                    current_end += next_char.len_utf8();

                    if next_char == '}' {
                        close_count += 1;
                    } else if next_char == '{' {
                        open_count += 1;
                    }
                }

                // Final slice check: If braces match, we use the source.
                if open_count == close_count {
                    s.push_str(&src[start..current_end]);
                    appended_from_source = true;
                }
            }
        }
    }

    if !appended_from_source {
        // Fallback: Use parsed value (User edited this, or new entry)
        for chunk in chunks {
            match &chunk.v {
                biblatex::Chunk::Normal(t) => s.push_str(&encode_latex(t)),
                biblatex::Chunk::Verbatim(t) => s.push_str(t),
                biblatex::Chunk::Math(t) => {
                    // Math chunks must be re-wrapped in $...$ delimiters
                    s.push('$');
                    s.push_str(t);
                    s.push('$');
                }
            }
        }
    }

    // Force \n explicitly
    let _ = write!(out, "{}{} = {{{}}},\n", indent, key, s);
}

/// Re-encodes Unicode to LaTeX accent macros (used only for NEW or EDITED fields).
///
/// Uses the `\"{a}` form (command outside braces) instead of `{\"a}` (command inside braces).
/// The `{\"a}` form causes "Paragraph completed before \TU\" errors under fontspec/XeLaTeX/LuaLaTeX
/// because the accent command expands inside a group where the TU encoding mapping hasn't finished.
/// The `\"{a}` form is safe across both legacy (OT1/T1) and modern (TU) font encodings.
///
/// Only actual Unicode accented characters are converted — normal ASCII (including apostrophes,
/// quotes, etc.) passes through unchanged.
fn encode_latex(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            // Umlauts / diaeresis
            'ä' => out.push_str("\\\"{a}"),
            'Ä' => out.push_str("\\\"{A}"),
            'ö' => out.push_str("\\\"{o}"),
            'Ö' => out.push_str("\\\"{O}"),
            'ü' => out.push_str("\\\"{u}"),
            'Ü' => out.push_str("\\\"{U}"),
            'ë' => out.push_str("\\\"{e}"),
            'Ë' => out.push_str("\\\"{E}"),
            // Acute
            'é' => out.push_str("\\'{e}"),
            'É' => out.push_str("\\'{E}"),
            'á' => out.push_str("\\'{a}"),
            'Á' => out.push_str("\\'{A}"),
            'í' => out.push_str("\\'{i}"),
            'Í' => out.push_str("\\'{I}"),
            'ó' => out.push_str("\\'{o}"),
            'Ó' => out.push_str("\\'{O}"),
            'ú' => out.push_str("\\'{u}"),
            'Ú' => out.push_str("\\'{U}"),
            // Grave
            'è' => out.push_str("\\`{e}"),
            'È' => out.push_str("\\`{E}"),
            'à' => out.push_str("\\`{a}"),
            'À' => out.push_str("\\`{A}"),
            // Circumflex
            'ê' => out.push_str("\\^{e}"),
            'Ê' => out.push_str("\\^{E}"),
            'â' => out.push_str("\\^{a}"),
            'Â' => out.push_str("\\^{A}"),
            'ô' => out.push_str("\\^{o}"),
            'Ô' => out.push_str("\\^{O}"),
            'û' => out.push_str("\\^{u}"),
            'Û' => out.push_str("\\^{U}"),
            // Tilde
            'ñ' => out.push_str("\\~{n}"),
            'Ñ' => out.push_str("\\~{N}"),
            'ã' => out.push_str("\\~{a}"),
            'Ã' => out.push_str("\\~{A}"),
            // Special
            'ß' => out.push_str("\\ss{}"),
            'å' => out.push_str("\\aa{}"),
            'Å' => out.push_str("\\AA{}"),
            'ø' => out.push_str("\\o{}"),
            'Ø' => out.push_str("\\O{}"),
            'ç' => out.push_str("\\c{c}"),
            'Ç' => out.push_str("\\c{C}"),
            // Everything else (including ASCII apostrophes, quotes, etc.) passes through as-is
            _ => out.push(c),
        }
    }
    out
}
