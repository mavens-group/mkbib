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

    let _ = writeln!(out, "{}{} = {{{}}},", indent, key, value);
}

/// Encodes Unicode characters to LaTeX macros.
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
            // ----- Umlauts / diaeresis -----
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
            'ÿ' => out.push_str("\\\"{y}"),
            // ----- Acute -----
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
            'ć' => out.push_str("\\'{c}"),
            'Ć' => out.push_str("\\'{C}"),
            'ń' => out.push_str("\\'{n}"),
            'Ń' => out.push_str("\\'{N}"),
            'ś' => out.push_str("\\'{s}"),
            'Ś' => out.push_str("\\'{S}"),
            'ź' => out.push_str("\\'{z}"),
            'Ź' => out.push_str("\\'{Z}"),
            // ----- Grave -----
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
            // ----- Circumflex -----
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
            'ĉ' => out.push_str("\\^{c}"),
            'ŝ' => out.push_str("\\^{s}"),
            // ----- Tilde -----
            'ñ' => out.push_str("\\~{n}"),
            'Ñ' => out.push_str("\\~{N}"),
            'ã' => out.push_str("\\~{a}"),
            'Ã' => out.push_str("\\~{A}"),
            'õ' => out.push_str("\\~{o}"),
            'Õ' => out.push_str("\\~{O}"),
            // ----- Caron / háček -----
            'č' => out.push_str("\\v{c}"),
            'Č' => out.push_str("\\v{C}"),
            'š' => out.push_str("\\v{s}"),
            'Š' => out.push_str("\\v{S}"),
            'ž' => out.push_str("\\v{z}"),
            'Ž' => out.push_str("\\v{Z}"),
            'ř' => out.push_str("\\v{r}"),
            'Ř' => out.push_str("\\v{R}"),
            'ě' => out.push_str("\\v{e}"),
            'Ě' => out.push_str("\\v{E}"),
            'ň' => out.push_str("\\v{n}"),
            'Ň' => out.push_str("\\v{N}"),
            'ď' => out.push_str("\\v{d}"),
            'Ď' => out.push_str("\\v{D}"),
            'ť' => out.push_str("\\v{t}"),
            'Ť' => out.push_str("\\v{T}"),
            // ----- Double acute (Hungarian) -----
            'ő' => out.push_str("\\H{o}"),
            'Ő' => out.push_str("\\H{O}"),
            'ű' => out.push_str("\\H{u}"),
            'Ű' => out.push_str("\\H{U}"),
            // ----- Ogonek (Polish/Lithuanian) -----
            'ą' => out.push_str("\\k{a}"),
            'Ą' => out.push_str("\\k{A}"),
            'ę' => out.push_str("\\k{e}"),
            'Ę' => out.push_str("\\k{E}"),
            // ----- Breve -----
            'ğ' => out.push_str("\\u{g}"),
            'Ğ' => out.push_str("\\u{G}"),
            'ă' => out.push_str("\\u{a}"),
            'Ă' => out.push_str("\\u{A}"),
            // ----- Dot above -----
            'ż' => out.push_str("\\.{z}"),
            'Ż' => out.push_str("\\.{Z}"),
            'İ' => out.push_str("\\.{I}"),
            // ----- Cedilla -----
            'ç' => out.push_str("\\c{c}"),
            'Ç' => out.push_str("\\c{C}"),
            'ş' => out.push_str("\\c{s}"),
            'Ş' => out.push_str("\\c{S}"),
            'ţ' => out.push_str("\\c{t}"),
            'Ţ' => out.push_str("\\c{T}"),
            // ----- Special letters -----
            'ß' => out.push_str("\\ss{}"),
            'å' => out.push_str("\\aa{}"),
            'Å' => out.push_str("\\AA{}"),
            'ø' => out.push_str("\\o{}"),
            'Ø' => out.push_str("\\O{}"),
            'æ' => out.push_str("\\ae{}"),
            'Æ' => out.push_str("\\AE{}"),
            'œ' => out.push_str("\\oe{}"),
            'Œ' => out.push_str("\\OE{}"),
            'ł' => out.push_str("\\l{}"),
            'Ł' => out.push_str("\\L{}"),
            'đ' => out.push_str("\\dj{}"),
            'Đ' => out.push_str("\\DJ{}"),
            'ð' => out.push_str("\\dh{}"),
            'Ð' => out.push_str("\\DH{}"),
            'þ' => out.push_str("\\th{}"),
            'Þ' => out.push_str("\\TH{}"),
            'ı' => out.push_str("{\\i}"),
            // ----- Typographic characters -----
            '–' => out.push_str("--"),        // en-dash
            '—' => out.push_str("---"),       // em-dash
            '\u{2018}' => out.push('`'),      // left single quote '
            '\u{2019}' => out.push('\''),     // right single quote '
            '\u{201C}' => out.push_str("``"), // left double quote "
            '\u{201D}' => out.push_str("''"), // right double quote "
            '\u{2026}' => out.push_str("\\ldots{}"), // ellipsis …
            // ----- Unicode math symbols (common in publisher metadata) -----
            '\u{2212}' => out.push_str("$-$"),         // minus sign −
            '×' => out.push_str("$\\times$"),          // multiplication ×
            '±' => out.push_str("$\\pm$"),             // plus-minus
            '∓' => out.push_str("$\\mp$"),             // minus-plus
            '·' => out.push_str("$\\cdot$"),           // middle dot (math)
            '≤' => out.push_str("$\\leq$"),            // less-equal
            '≥' => out.push_str("$\\geq$"),            // greater-equal
            '≈' => out.push_str("$\\approx$"),         // approximately
            '≠' => out.push_str("$\\neq$"),            // not equal
            '∞' => out.push_str("$\\infty$"),          // infinity
            '→' => out.push_str("$\\rightarrow$"),     // right arrow
            '←' => out.push_str("$\\leftarrow$"),      // left arrow
            '↔' => out.push_str("$\\leftrightarrow$"), // bidirectional arrow
            '∂' => out.push_str("$\\partial$"),        // partial derivative
            '∇' => out.push_str("$\\nabla$"),          // nabla
            '°' => out.push_str("$^{\\circ}$"),        // degree sign
            '∼' => out.push_str("$\\sim$"),            // tilde operator
            // ----- Greek letters (Unicode → LaTeX math) -----
            'α' => out.push_str("$\\alpha$"),
            'β' => out.push_str("$\\beta$"),
            'γ' => out.push_str("$\\gamma$"),
            'δ' => out.push_str("$\\delta$"),
            'ε' => out.push_str("$\\epsilon$"),
            'ζ' => out.push_str("$\\zeta$"),
            'η' => out.push_str("$\\eta$"),
            'θ' => out.push_str("$\\theta$"),
            'ι' => out.push_str("$\\iota$"),
            'κ' => out.push_str("$\\kappa$"),
            'λ' => out.push_str("$\\lambda$"),
            'μ' => out.push_str("$\\mu$"),
            'ν' => out.push_str("$\\nu$"),
            'ξ' => out.push_str("$\\xi$"),
            'π' => out.push_str("$\\pi$"),
            'ρ' => out.push_str("$\\rho$"),
            'σ' => out.push_str("$\\sigma$"),
            'τ' => out.push_str("$\\tau$"),
            'υ' => out.push_str("$\\upsilon$"),
            'φ' => out.push_str("$\\phi$"),
            'χ' => out.push_str("$\\chi$"),
            'ψ' => out.push_str("$\\psi$"),
            'ω' => out.push_str("$\\omega$"),
            'Γ' => out.push_str("$\\Gamma$"),
            'Δ' => out.push_str("$\\Delta$"),
            'Θ' => out.push_str("$\\Theta$"),
            'Λ' => out.push_str("$\\Lambda$"),
            'Ξ' => out.push_str("$\\Xi$"),
            'Π' => out.push_str("$\\Pi$"),
            'Σ' => out.push_str("$\\Sigma$"),
            'Φ' => out.push_str("$\\Phi$"),
            'Ψ' => out.push_str("$\\Psi$"),
            'Ω' => out.push_str("$\\Omega$"),
            // Everything else passes through as-is
            _ => out.push(c),
        }
    }
    out
}
