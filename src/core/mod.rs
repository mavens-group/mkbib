// src/core/mod.rs
pub mod config;
pub mod keygen;

use biblatex::{Chunk, Spanned};
use std::path::Path;

/// Helper to safely get string from a list of chunks.
/// Preserves raw content (including LaTeX commands inside Math chunks).
/// Use for round-tripping and data operations where the exact content matters.
pub fn bib_to_string(val: &[Spanned<Chunk>]) -> String {
    val.iter()
        .map(|c| match &c.v {
            Chunk::Normal(s) => s.clone(),
            Chunk::Verbatim(s) => s.clone(),
            Chunk::Math(s) => s.clone(),
        })
        .collect()
}

/// Human-readable display string: strips LaTeX commands from Math chunks.
/// Use for UI display, key generation, duplicate detection — anywhere
/// users or algorithms need clean text without LaTeX noise.
pub fn bib_to_display_string(val: &[Spanned<Chunk>]) -> String {
    val.iter()
        .map(|c| match &c.v {
            Chunk::Normal(s) => s.clone(),
            Chunk::Verbatim(s) => s.clone(),
            Chunk::Math(s) => strip_math_for_display(s),
        })
        .collect()
}

/// Strips LaTeX math markup to produce readable text.
/// `\mathrm{CO_{2}}` → `CO2`, `\ce{H2O}` → `H2O`,
/// `x^{2}` → `x2`, `\alpha` → `α` (common symbols).
fn strip_math_for_display(math: &str) -> String {
    let mut out = String::with_capacity(math.len());
    let chars: Vec<char> = math.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\\' && i + 1 < len {
            // Try to match known LaTeX commands
            let rest: String = chars[i..].iter().collect();
            if let Some((replacement, skip)) = match_latex_command(&rest) {
                out.push_str(replacement);
                i += skip;
                continue;
            }
            // Unknown command — skip the backslash and command name
            i += 1; // skip '\'
            while i < len && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            // Skip optional {} after command
            if i < len && chars[i] == '{' {
                let mut depth = 1;
                i += 1;
                while i < len && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        out.push(chars[i]);
                    }
                    i += 1;
                }
            }
        } else if chars[i] == '{' || chars[i] == '}' {
            // Skip bare braces
            i += 1;
        } else if chars[i] == '_' || chars[i] == '^' {
            // Skip sub/superscript markers, content follows
            i += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    out
}

/// Matches known LaTeX commands and returns (replacement_text, chars_to_skip).
fn match_latex_command(rest: &str) -> Option<(&'static str, usize)> {
    // Wrapper commands whose content should be kept
    let wrappers = [
        ("\\mathrm{", 8),
        ("\\textrm{", 8),
        ("\\textit{", 8),
        ("\\textbf{", 8),
        ("\\text{", 6),
        ("\\ce{", 4),
    ];
    for (prefix, _skip) in &wrappers {
        if rest.starts_with(prefix) {
            // Return None here — the caller will skip the command name,
            // then process the brace content, keeping the inner text.
            return None;
        }
    }

    // Greek letter substitutions (most common in scientific titles)
    let greeks: &[(&str, &str)] = &[
        ("\\alpha", "α"),
        ("\\beta", "β"),
        ("\\gamma", "γ"),
        ("\\delta", "δ"),
        ("\\epsilon", "ε"),
        ("\\zeta", "ζ"),
        ("\\eta", "η"),
        ("\\theta", "θ"),
        ("\\iota", "ι"),
        ("\\kappa", "κ"),
        ("\\lambda", "λ"),
        ("\\mu", "μ"),
        ("\\nu", "ν"),
        ("\\xi", "ξ"),
        ("\\pi", "π"),
        ("\\rho", "ρ"),
        ("\\sigma", "σ"),
        ("\\tau", "τ"),
        ("\\upsilon", "υ"),
        ("\\phi", "φ"),
        ("\\chi", "χ"),
        ("\\psi", "ψ"),
        ("\\omega", "ω"),
        ("\\Gamma", "Γ"),
        ("\\Delta", "Δ"),
        ("\\Theta", "Θ"),
        ("\\Lambda", "Λ"),
        ("\\Xi", "Ξ"),
        ("\\Pi", "Π"),
        ("\\Sigma", "Σ"),
        ("\\Phi", "Φ"),
        ("\\Psi", "Ψ"),
        ("\\Omega", "Ω"),
    ];
    for (cmd, repl) in greeks {
        if rest.starts_with(cmd) {
            // Ensure the command is terminated (not a prefix of a longer word)
            let cmd_len = cmd.len();
            if rest.len() == cmd_len
                || !rest
                    .as_bytes()
                    .get(cmd_len)
                    .is_some_and(|b| b.is_ascii_alphabetic())
            {
                return Some((repl, cmd_len));
            }
        }
    }

    // Other common symbols
    let symbols: &[(&str, &str)] = &[
        ("\\pm", "±"),
        ("\\mp", "∓"),
        ("\\times", "×"),
        ("\\cdot", "·"),
        ("\\leq", "≤"),
        ("\\geq", "≥"),
        ("\\neq", "≠"),
        ("\\approx", "≈"),
        ("\\sim", "~"),
        ("\\infty", "∞"),
        ("\\rightarrow", "→"),
        ("\\leftarrow", "←"),
        ("\\leftrightarrow", "↔"),
        ("\\partial", "∂"),
        ("\\nabla", "∇"),
    ];
    for (cmd, repl) in symbols {
        if rest.starts_with(cmd) {
            let cmd_len = cmd.len();
            if rest.len() == cmd_len
                || !rest
                    .as_bytes()
                    .get(cmd_len)
                    .is_some_and(|b| b.is_ascii_alphabetic())
            {
                return Some((repl, cmd_len));
            }
        }
    }

    None
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

// ----------------------------------------------------------------------------
// BibTeX pre-processor: normalize non-standard month macros
//
// The biblatex crate only knows the standard 3-letter month macros
// (jan..dec). Publishers (APS, some Wiley imprints) emit `month = sept`
// or `month = September` as bare identifiers, which trigger
// "unknown abbreviation" parse errors.
//
// This runs BEFORE Bibliography::parse() at every ingress point
// (file open, DOI fetch, manual paste, inline edit save).
// ----------------------------------------------------------------------------

/// Canonicalize a bare month value to its standard 3-letter macro.
/// Returns None if the value isn't recognizable as a month.
fn canonicalize_month(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some("jan"),
        "feb" | "february" => Some("feb"),
        "mar" | "march" => Some("mar"),
        "apr" | "april" => Some("apr"),
        "may" => Some("may"),
        "jun" | "june" => Some("jun"),
        "jul" | "july" => Some("jul"),
        "aug" | "august" => Some("aug"),
        "sep" | "sept" | "september" => Some("sep"),
        "oct" | "october" => Some("oct"),
        "nov" | "november" => Some("nov"),
        "dec" | "december" => Some("dec"),
        _ => None,
    }
}

/// Process a single `key = value` slice at field position (everything
/// between two field-separating commas, not including the commas).
/// Appends the possibly-rewritten slice to `out`.
fn rewrite_field_slice(slice: &str, out: &mut String) {
    let bytes = slice.as_bytes();
    let len = bytes.len();

    // Skip leading whitespace to find field key
    let mut p = 0;
    while p < len && matches!(bytes[p], b' ' | b'\t' | b'\n' | b'\r') {
        p += 1;
    }
    let key_start = p;
    while p < len && bytes[p].is_ascii_alphanumeric() {
        p += 1;
    }
    let key = &slice[key_start..p];

    if !key.eq_ignore_ascii_case("month") {
        out.push_str(slice);
        return;
    }

    // Skip whitespace to '='
    while p < len && matches!(bytes[p], b' ' | b'\t') {
        p += 1;
    }
    if p >= len || bytes[p] != b'=' {
        out.push_str(slice);
        return;
    }
    p += 1;

    // Skip whitespace after '='
    while p < len && matches!(bytes[p], b' ' | b'\t' | b'\n' | b'\r') {
        p += 1;
    }

    // Braced or quoted → already a literal, leave alone
    if p >= len || bytes[p] == b'{' || bytes[p] == b'"' {
        out.push_str(slice);
        return;
    }

    // Extract bare value (trim trailing whitespace)
    let val_start = p;
    let mut val_end = len;
    while val_end > val_start && matches!(bytes[val_end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        val_end -= 1;
    }
    let value = &slice[val_start..val_end];

    // Concatenation or empty → don't touch
    if value.is_empty() || value.contains('#') {
        out.push_str(slice);
        return;
    }
    // Numeric → parses fine as integer literal
    if value.chars().all(|c| c.is_ascii_digit()) {
        out.push_str(slice);
        return;
    }

    // Write prefix unchanged
    out.push_str(&slice[..val_start]);

    // Rewrite value
    let is_pure_alpha = value.chars().all(|c| c.is_ascii_alphabetic());
    if is_pure_alpha {
        if let Some(canonical) = canonicalize_month(value) {
            out.push_str(canonical);
        } else {
            // Unknown macro — wrap so parser sees a literal
            out.push('{');
            out.push_str(value);
            out.push('}');
        }
    } else {
        // Mixed (e.g. "sept-oct") — wrap to be safe
        out.push('{');
        out.push_str(value);
        out.push('}');
    }

    // Preserve trailing whitespace
    out.push_str(&slice[val_end..]);
}

/// Walks the BibTeX source, locates `month = …` fields at entry
/// top-level, and rewrites non-standard macro values to forms the
/// biblatex crate can parse. All other content is passed through
/// byte-for-byte.
pub fn normalize_month_macros(content: &str) -> String {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        // Only act at entry openers
        if bytes[i] != b'@' {
            // Advance past one UTF-8 scalar
            let start = i;
            i += 1;
            while i < len && (bytes[i] & 0xC0) == 0x80 {
                i += 1;
            }
            out.push_str(&content[start..i]);
            continue;
        }

        // '@' — consume entry type
        let entry_start = i;
        i += 1;
        while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let entry_type_end = i;
        let entry_type = &content[entry_start + 1..entry_type_end];

        // Skip whitespace to opening delimiter
        while i < len && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }

        // Not a real entry — bail out and emit what we consumed
        if i >= len || (bytes[i] != b'{' && bytes[i] != b'(') {
            out.push_str(&content[entry_start..i]);
            continue;
        }

        // @comment / @preamble / @string — don't touch bodies; find
        // matching close delimiter and pass through.
        let is_meta = matches!(
            entry_type.to_ascii_lowercase().as_str(),
            "comment" | "preamble" | "string"
        );

        let open = bytes[i];
        let close = if open == b'{' { b'}' } else { b')' };
        i += 1;
        out.push_str(&content[entry_start..i]);

        if is_meta {
            // Walk to matching close, tracking depth, passing through verbatim
            let mut depth = 1;
            let seg_start = i;
            while i < len && depth > 0 {
                match bytes[i] {
                    b'"' => {
                        i += 1;
                        while i < len && bytes[i] != b'"' {
                            if bytes[i] == b'\\' && i + 1 < len {
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        if i < len {
                            i += 1;
                        }
                    }
                    b'{' => {
                        depth += 1;
                        i += 1;
                    }
                    b if b == close => {
                        depth -= 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            out.push_str(&content[seg_start..i]);
            continue;
        }

        // Regular entry: walk fields at depth 1
        let mut depth = 1;
        let mut field_start = i;
        let mut seen_first_comma = false; // first comma ends the entry key

        while i < len && depth > 0 {
            let b = bytes[i];

            // Skip over quoted strings at any depth — they can't contain `,` at depth 1 for us
            if b == b'"' && depth == 1 {
                i += 1;
                while i < len && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            if b == b'{' {
                depth += 1;
                i += 1;
                continue;
            }
            if b == close {
                depth -= 1;
                if depth == 0 {
                    // Final trailing field (no comma after it)
                    let tail = &content[field_start..i];
                    if seen_first_comma {
                        rewrite_field_slice(tail, &mut out);
                    } else {
                        // Pathological: entry had no fields (just a key).
                        out.push_str(tail);
                    }
                    out.push(close as char);
                    i += 1;
                    break;
                }
                i += 1;
                continue;
            }

            // `,` at depth 1 separates key / fields
            if depth == 1 && b == b',' {
                let segment = &content[field_start..i];
                if !seen_first_comma {
                    // Entry key — emit verbatim
                    out.push_str(segment);
                    seen_first_comma = true;
                } else {
                    rewrite_field_slice(segment, &mut out);
                }
                out.push(',');
                i += 1;
                field_start = i;
                continue;
            }

            i += 1;
        }

        // Malformed / truncated entry — flush what we have
        if depth > 0 {
            out.push_str(&content[field_start..i]);
        }
    }

    out
}
