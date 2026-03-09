// src/logic/formatter.rs
//
// Clean formatter: always produces correct BibTeX/BibLaTeX output from parsed
// chunk data. No span-based source recovery — the merger handles inter-entry
// preservation (comments, @string, whitespace), and entries are always
// reformatted through this module.

use crate::core::keygen::{KeyGenConfig, UnicodeMode};
use biblatex::Entry;
use std::fmt::Write;

/// Formats a single bibliography entry as a BibTeX string.
pub fn format_entry(entry: &Entry, config: &KeyGenConfig) -> String {
    let mut out = String::new();

    // 1. Indentation
    let indent = if config.indent_char == '\t' {
        "\t".repeat(config.indent_width as usize)
    } else {
        " ".repeat(config.indent_width as usize)
    };

    // 2. Header: @type{key,
    let _ = writeln!(out, "@{}{{{},", entry.entry_type.to_bibtex(), entry.key);

    // 3. Fields — priority order first, then remaining sorted alphabetically
    let mut written_fields = std::collections::HashSet::new();

    for field_name in &config.field_order {
        if let Some(chunks) = entry.fields.get(field_name) {
            write_field(&mut out, field_name, chunks, &indent, &config.unicode_mode);
            written_fields.insert(field_name.clone());
        }
    }

    let mut remaining_keys: Vec<_> = entry.fields.keys().collect();
    remaining_keys.sort();

    for field_name in remaining_keys {
        if !written_fields.contains(field_name) {
            let chunks = entry.fields.get(field_name).unwrap();
            write_field(&mut out, field_name, chunks, &indent, &config.unicode_mode);
        }
    }

    // 4. Footer
    out.push('}');

    // Trim trailing whitespace from the last field line
    out.trim_end().to_string()
}

/// Writes a single field: `    fieldname = {value},\n`
fn write_field(
    out: &mut String,
    key: &str,
    chunks: &[biblatex::Spanned<biblatex::Chunk>],
    indent: &str,
    unicode_mode: &UnicodeMode,
) {
    let mut value = String::new();

    for chunk in chunks {
        match &chunk.v {
            biblatex::Chunk::Normal(t) => {
                match unicode_mode {
                    UnicodeMode::Utf8 => value.push_str(t),
                    UnicodeMode::Latex => value.push_str(&encode_latex(t)),
                }
            }
            biblatex::Chunk::Verbatim(t) => {
                // Verbatim content (URLs, DOIs) — never encode
                value.push_str(t);
            }
            biblatex::Chunk::Math(t) => {
                // Math content — wrap in $...$, never encode inside
                value.push('$');
                value.push_str(t);
                value.push('$');
            }
        }
    }

    let _ = write!(out, "{}{} = {{{}}},\n", indent, key, value);
}

/// Encodes Unicode accented characters to LaTeX accent macros.
///
/// Uses the `\"{a}` form (command outside braces) which is safe across
/// both legacy (OT1/T1) and modern (TU/fontspec) font encodings.
/// The `{\"a}` form causes "Paragraph completed before \TU\" errors.
///
/// Only runs when UnicodeMode::Latex is selected.
/// Normal ASCII (apostrophes, quotes, hyphens, etc.) passes through unchanged.
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
            'ï' => out.push_str("\\\"{i}"),
            'Ï' => out.push_str("\\\"{I}"),
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
            'ý' => out.push_str("\\'{y}"),
            'Ý' => out.push_str("\\'{Y}"),
            // Grave
            'è' => out.push_str("\\`{e}"),
            'È' => out.push_str("\\`{E}"),
            'à' => out.push_str("\\`{a}"),
            'À' => out.push_str("\\`{A}"),
            'ì' => out.push_str("\\`{i}"),
            'Ì' => out.push_str("\\`{I}"),
            'ò' => out.push_str("\\`{o}"),
            'Ò' => out.push_str("\\`{O}"),
            'ù' => out.push_str("\\`{u}"),
            'Ù' => out.push_str("\\`{U}"),
            // Circumflex
            'ê' => out.push_str("\\^{e}"),
            'Ê' => out.push_str("\\^{E}"),
            'â' => out.push_str("\\^{a}"),
            'Â' => out.push_str("\\^{A}"),
            'î' => out.push_str("\\^{i}"),
            'Î' => out.push_str("\\^{I}"),
            'ô' => out.push_str("\\^{o}"),
            'Ô' => out.push_str("\\^{O}"),
            'û' => out.push_str("\\^{u}"),
            'Û' => out.push_str("\\^{U}"),
            // Tilde
            'ñ' => out.push_str("\\~{n}"),
            'Ñ' => out.push_str("\\~{N}"),
            'ã' => out.push_str("\\~{a}"),
            'Ã' => out.push_str("\\~{A}"),
            'õ' => out.push_str("\\~{o}"),
            'Õ' => out.push_str("\\~{O}"),
            // Caron / háček
            'č' => out.push_str("\\v{c}"),
            'Č' => out.push_str("\\v{C}"),
            'š' => out.push_str("\\v{s}"),
            'Š' => out.push_str("\\v{S}"),
            'ž' => out.push_str("\\v{z}"),
            'Ž' => out.push_str("\\v{Z}"),
            'ř' => out.push_str("\\v{r}"),
            'Ř' => out.push_str("\\v{R}"),
            // Special
            'ß' => out.push_str("\\ss{}"),
            'å' => out.push_str("\\aa{}"),
            'Å' => out.push_str("\\AA{}"),
            'ø' => out.push_str("\\o{}"),
            'Ø' => out.push_str("\\O{}"),
            'æ' => out.push_str("\\ae{}"),
            'Æ' => out.push_str("\\AE{}"),
            'ç' => out.push_str("\\c{c}"),
            'Ç' => out.push_str("\\c{C}"),
            'ł' => out.push_str("\\l{}"),
            'Ł' => out.push_str("\\L{}"),
            'đ' => out.push_str("\\dj{}"),
            // Everything else passes through as-is
            _ => out.push(c),
        }
    }
    out
}
