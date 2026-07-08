// src/logic/library.rs

use crate::app::alert::AlertMsg;
use crate::app::AppModel;
use crate::core;
use crate::core::keygen::ChemFormulaStyle;
use crate::logic::abbreviator;
use crate::ui::details_dialog::DetailsDialogMsg;
use crate::ui::row::{BibEntry, BibEntryOutput};
use crate::ui::sidebar::SidebarMsg;
use biblatex::{Bibliography, Chunk, Spanned};
use relm4::{ComponentController, ComponentSender};
use std::collections::BTreeMap;

// ----------------------------------------------------------------------------
// 1. Helpers
// ----------------------------------------------------------------------------

fn ensure_unique(base_key: &str, bib: &Bibliography) -> String {
    if bib.get(base_key).is_none() {
        return base_key.to_string();
    }
    let mut suffix_char = 'a';
    loop {
        let candidate = format!("{}{}", base_key, suffix_char);
        if bib.get(&candidate).is_none() {
            return candidate;
        }
        if suffix_char == 'z' {
            break;
        }
        suffix_char = (suffix_char as u8 + 1) as char;
    }
    for i in 1..10000 {
        let candidate = format!("{}_{}", base_key, i);
        if bib.get(&candidate).is_none() {
            return candidate;
        }
    }
    // Last resort: append timestamp-like suffix
    format!(
        "{}_{}",
        base_key,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

fn make_normal_chunk(s: &str) -> Vec<Spanned<Chunk>> {
    vec![Spanned {
        v: Chunk::Normal(s.to_string()),
        span: 0..0, // Dummy span
    }]
}

/// Creates a list of properly typed chunks from text that may contain $...$ math.
/// Text outside $...$ becomes Chunk::Normal, text inside becomes Chunk::Math
/// (without the $ delimiters, matching biblatex's convention).
fn make_mixed_chunks(s: &str) -> Vec<Spanned<Chunk>> {
    let mut chunks = Vec::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(dollar_start) = remaining.find('$') {
            // Push Normal text before the $
            if dollar_start > 0 {
                chunks.push(Spanned {
                    v: Chunk::Normal(remaining[..dollar_start].to_string()),
                    span: 0..0,
                });
            }

            // Find closing $
            let after_open = &remaining[dollar_start + 1..];
            if let Some(dollar_end) = after_open.find('$') {
                let math_content = &after_open[..dollar_end];
                chunks.push(Spanned {
                    v: Chunk::Math(math_content.to_string()),
                    span: 0..0,
                });
                remaining = &after_open[dollar_end + 1..];
            } else {
                // Unmatched $ — treat rest as normal
                chunks.push(Spanned {
                    v: Chunk::Normal(remaining[dollar_start..].to_string()),
                    span: 0..0,
                });
                break;
            }
        } else {
            // No more $, rest is normal
            chunks.push(Spanned {
                v: Chunk::Normal(remaining.to_string()),
                span: 0..0,
            });
            break;
        }
    }

    if chunks.is_empty() {
        chunks.push(Spanned {
            v: Chunk::Normal(String::new()),
            span: 0..0,
        });
    }

    chunks
}

/// Normalizes a field's chunks: flattens Normal/Math to text, cleans up legacy
/// LaTeX escapes, re-splits into proper Normal/Math chunks, and wraps bare
/// math content in the user's chosen chemical formula style.
///
/// Verbatim chunks (URLs, DOIs) are preserved as-is and not processed.
fn normalize_field_chunks(
    chunks: &[Spanned<Chunk>],
    chem_style: &ChemFormulaStyle,
) -> Vec<Spanned<Chunk>> {
    // Separate Verbatim chunks from Normal/Math
    // We only normalize Normal/Math content; Verbatim passes through untouched.
    let mut has_non_verbatim = false;
    let mut has_verbatim = false;

    for chunk in chunks {
        match &chunk.v {
            Chunk::Verbatim(_) => has_verbatim = true,
            _ => has_non_verbatim = true,
        }
    }

    // If the field is entirely Verbatim (e.g. url, doi), return as-is
    if !has_non_verbatim {
        return chunks.to_vec();
    }

    // If mixed Verbatim + Normal/Math, only normalize the non-Verbatim parts
    // in-place. This is unusual but defensive.
    if has_verbatim {
        return chunks.to_vec();
    }

    // 1. Flatten Normal/Math chunks into a single string, re-inserting $...$ for Math
    let mut flat = String::new();
    for chunk in chunks {
        match &chunk.v {
            Chunk::Normal(t) => flat.push_str(t),
            Chunk::Math(t) => {
                flat.push('$');
                flat.push_str(t);
                flat.push('$');
            }
            Chunk::Verbatim(_) => unreachable!(), // filtered above
        }
    }

    // 2. Clean up legacy BibTeX escapes inside $...$ math regions
    let cleaned = clean_math_escapes(&flat);

    // 3. Re-split into proper Normal/Math chunks
    let mut result = make_mixed_chunks(&cleaned);

    // 4. Re-wrap bare math content in the chosen chemistry style.
    //    "Bare" means math like `_2` or `Co_{1+x}Fe_{1-x}` that isn't
    //    already wrapped in \mathrm{...} or \ce{...}.
    if *chem_style != ChemFormulaStyle::None {
        for chunk in &mut result {
            if let Chunk::Math(ref mut content) = chunk.v {
                let trimmed = content.trim();

                // Detect existing wrapper commands
                let existing_wrapper = if trimmed.starts_with("\\mathrm{") {
                    Some("mathrm")
                } else if trimmed.starts_with("\\ce{") {
                    Some("ce")
                } else if trimmed.starts_with("\\text{") {
                    Some("text")
                } else if trimmed.starts_with("\\textrm{") {
                    Some("textrm")
                } else {
                    None
                };

                if let Some(wrapper) = existing_wrapper {
                    // Already has a wrapper — extract inner content and re-wrap
                    // if switching styles. Use brace-depth matching for safety.
                    let prefix_len = wrapper.len() + 2; // "\\" + name + "{"
                    if let Some(inner) = extract_brace_content(trimmed, prefix_len - 1) {
                        match (chem_style, wrapper) {
                            (ChemFormulaStyle::Math, "ce")
                            | (ChemFormulaStyle::Math, "text")
                            | (ChemFormulaStyle::Math, "textrm") => {
                                *content = format!("\\mathrm{{{}}}", inner);
                            }
                            (ChemFormulaStyle::Mhchem, "mathrm")
                            | (ChemFormulaStyle::Mhchem, "text")
                            | (ChemFormulaStyle::Mhchem, "textrm") => {
                                *content = format!("\\ce{{{}}}", inner);
                            }
                            _ => {} // Same style or unrecognized — leave as-is
                        }
                    }
                } else if looks_like_formula(trimmed) {
                    // Bare formula content — wrap in chosen style
                    match chem_style {
                        ChemFormulaStyle::Math => {
                            *content = format!("\\mathrm{{{}}}", trimmed);
                        }
                        ChemFormulaStyle::Mhchem => {
                            *content = format!("\\ce{{{}}}", trimmed);
                        }
                        ChemFormulaStyle::None => {} // unreachable due to outer check
                    }
                }
                // If it doesn't look like a formula (e.g. pure math like
                // \alpha + \beta), leave it alone
            }
        }
    }

    result
}

/// Extracts the content inside the first brace-delimited group starting at
/// position `brace_start` in `text`. Uses proper brace-depth matching.
/// Returns None if the braces don't balance.
fn extract_brace_content(text: &str, brace_start: usize) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if brace_start >= chars.len() || chars[brace_start] != '{' {
        return None;
    }

    let mut depth = 0;
    let content_start = brace_start + 1;
    let mut i = brace_start;

    while i < chars.len() {
        if chars[i] == '{' {
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                // Found the matching close brace
                let content: String = chars[content_start..i].iter().collect();
                return Some(content);
            }
        }
        i += 1;
    }

    None // Unbalanced braces
}

/// Heuristic: does this math content look like a chemical formula?
/// True if it contains element-like patterns (uppercase letter + optional
/// lowercase + optional subscript/superscript).
/// False for pure math like `\alpha`, `x^2 + y^2`, `E = mc^2`.
fn looks_like_formula(math: &str) -> bool {
    // Chemical formulas typically have:
    //   - uppercase letters (element symbols)
    //   - subscripts with digits: _{2}, _{1+x}
    //   - no spaces (or very few)
    // Pure math typically has:
    //   - Greek letters (\alpha, \beta)
    //   - operators (+, -, =)
    //   - function names (\sin, \cos)

    // Quick checks: if it contains Greek letters, = sign, or common math
    // functions, it's probably pure math — not a chemical formula.
    let greek_and_math = [
        // Lowercase Greek
        "\\alpha",
        "\\beta",
        "\\gamma",
        "\\delta",
        "\\epsilon",
        "\\varepsilon",
        "\\zeta",
        "\\eta",
        "\\theta",
        "\\vartheta",
        "\\iota",
        "\\kappa",
        "\\lambda",
        "\\mu",
        "\\nu",
        "\\xi",
        "\\pi",
        "\\varpi",
        "\\rho",
        "\\varrho",
        "\\sigma",
        "\\varsigma",
        "\\tau",
        "\\upsilon",
        "\\phi",
        "\\varphi",
        "\\chi",
        "\\psi",
        "\\omega",
        // Uppercase Greek
        "\\Gamma",
        "\\Delta",
        "\\Theta",
        "\\Lambda",
        "\\Xi",
        "\\Pi",
        "\\Sigma",
        "\\Upsilon",
        "\\Phi",
        "\\Psi",
        "\\Omega",
        // Math functions and operators
        "\\sin",
        "\\cos",
        "\\tan",
        "\\log",
        "\\ln",
        "\\exp",
        "\\lim",
        "\\sum",
        "\\prod",
        "\\int",
        "\\frac",
        "\\sqrt",
        "\\vec",
        "\\hat",
        "\\bar",
        "\\dot",
        "\\tilde",
        // Relation operators
        "\\leq",
        "\\geq",
        "\\neq",
        "\\approx",
        "\\sim",
        "\\equiv",
        "\\propto",
        "\\parallel",
        "\\perp",
    ];

    if math.contains('=') {
        return false;
    }

    for pattern in &greek_and_math {
        if math.contains(pattern) {
            return false;
        }
    }

    // Must have at least one uppercase ASCII letter (element symbol)
    let has_element = math.chars().any(|c| c.is_ascii_uppercase());

    // Must have at least one subscript or digit adjacent to a letter
    let has_subscript = math.contains("_{") || math.contains("^{");
    let has_adjacent_digit = {
        let chars: Vec<char> = math.chars().collect();
        chars.windows(2).any(|w| {
            (w[0].is_ascii_alphabetic() && w[1].is_ascii_digit())
                || (w[0].is_ascii_digit() && w[1].is_ascii_uppercase())
        })
    };

    has_element && (has_subscript || has_adjacent_digit)
}

/// Cleans legacy BibTeX escapes inside $...$ math regions only.
fn clean_math_escapes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_math = false;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '$' {
            in_math = !in_math;
            result.push('$');
            i += 1;
        } else if in_math && chars[i] == '\\' && i + 1 < len {
            // Inside math: check for escape sequences to clean
            let next = chars[i + 1];
            match next {
                '_' | '{' | '}' | '&' | '#' | '%' => {
                    // Strip the backslash, keep the character
                    result.push(next);
                    i += 2;
                }
                _ => {
                    // Real LaTeX command inside math (\mathrm, \ce, etc.) — keep as-is
                    result.push('\\');
                    i += 1;
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

// Fixed Helper: Smart tag stripping with italic conversion
fn strip_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());

    // Track italic state for <i>/<em> → \textit{} conversion
    let mut italic_stack: Vec<&str> = Vec::new();
    let chars_vec: Vec<char> = input.chars().collect();
    let len = chars_vec.len();
    let mut i = 0;

    while i < len {
        if chars_vec[i] == '<' {
            // Find the end of this tag
            i += 1;
            let mut tag_content = String::new();
            while i < len && chars_vec[i] != '>' {
                tag_content.push(chars_vec[i]);
                i += 1;
            }
            if i < len {
                i += 1; // skip '>'
            }

            let tag_lower = tag_content.to_lowercase();
            let tag_lower = tag_lower.trim();

            // Handle specific tags with semantic meaning
            if tag_lower == "i"
                || tag_lower == "em"
                || tag_lower.starts_with("i ")
                || tag_lower.starts_with("em ")
            {
                // Opening italic tag → start \textit{
                output.push_str("\\textit{");
                italic_stack.push(if tag_lower.starts_with("i") {
                    "i"
                } else {
                    "em"
                });
            } else if tag_lower == "/i" || tag_lower == "/em" {
                // Closing italic tag → close }
                if !italic_stack.is_empty() {
                    output.push('}');
                    italic_stack.pop();
                }
            } else if tag_lower.contains("math")
                || tag_lower == "br"
                || tag_lower == "p"
                || tag_lower == "div"
                || tag_lower == "/p"
                || tag_lower == "/div"
            {
                output.push(' ');
            }
            // All other tags (including <sub>, <sup>, <b>, etc.) are silently stripped
        } else {
            output.push(chars_vec[i]);
            i += 1;
        }
    }

    // Close any unclosed italic braces
    for _ in &italic_stack {
        output.push('}');
    }

    // Decode HTML entities (common ones from publisher BibTeX)
    let decoded = decode_html_entities(&output);

    decoded.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Decodes HTML entities (named, decimal, and hex) to their Unicode equivalents.
fn decode_html_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '&' {
            // Try to find the closing semicolon
            i += 1;
            let mut entity = String::new();
            while i < len && chars[i] != ';' && entity.len() < 12 {
                entity.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == ';' {
                i += 1; // skip ';'
                if let Some(decoded) = decode_entity(&entity) {
                    result.push_str(decoded);
                } else if let Some(stripped) = entity.strip_prefix('#') {
                    // Numeric entity
                    if let Some(c) = decode_numeric_entity(stripped) {
                        result.push(c);
                    } else {
                        // Unrecognized — keep original
                        result.push('&');
                        result.push_str(&entity);
                        result.push(';');
                    }
                } else {
                    // Unrecognized named entity — keep original
                    result.push('&');
                    result.push_str(&entity);
                    result.push(';');
                }
            } else {
                // No semicolon found — not an entity, keep the ampersand
                result.push('&');
                result.push_str(&entity);
                // Don't skip the current char — it's the next char to process
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

fn decode_entity(entity: &str) -> Option<&'static str> {
    match entity {
        // Basic
        "lt" => Some("<"),
        "gt" => Some(">"),
        "amp" => Some("&"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        "nbsp" => Some(" "),
        // Dashes and hyphens
        "ndash" => Some("–"),
        "mdash" => Some("—"),
        "minus" => Some("−"),
        // Typographic
        "lsquo" => Some("\u{2018}"), // '
        "rsquo" => Some("\u{2019}"), // '
        "ldquo" => Some("\u{201C}"), // "
        "rdquo" => Some("\u{201D}"), // "
        "hellip" => Some("…"),
        // Math and science
        "times" => Some("×"),
        "divide" => Some("÷"),
        "plusmn" => Some("±"),
        "deg" => Some("°"),
        "micro" => Some("μ"),
        "middot" => Some("·"),
        "sim" | "tilde" => Some("∼"),
        "asymp" => Some("≈"),
        "ne" => Some("≠"),
        "le" | "leq" => Some("≤"),
        "ge" | "geq" => Some("≥"),
        "infin" => Some("∞"),
        "rarr" => Some("→"),
        "larr" => Some("←"),
        "harr" => Some("↔"),
        "part" => Some("∂"),
        "nabla" => Some("∇"),
        "prime" => Some("′"),
        "Prime" => Some("″"),
        // Greek letters (named entities)
        "alpha" => Some("α"),
        "beta" => Some("β"),
        "gamma" => Some("γ"),
        "delta" => Some("δ"),
        "epsilon" => Some("ε"),
        "zeta" => Some("ζ"),
        "eta" => Some("η"),
        "theta" => Some("θ"),
        "iota" => Some("ι"),
        "kappa" => Some("κ"),
        "lambda" => Some("λ"),
        "mu" => Some("μ"),
        "nu" => Some("ν"),
        "xi" => Some("ξ"),
        "pi" => Some("π"),
        "rho" => Some("ρ"),
        "sigma" => Some("σ"),
        "tau" => Some("τ"),
        "upsilon" => Some("υ"),
        "phi" => Some("φ"),
        "chi" => Some("χ"),
        "psi" => Some("ψ"),
        "omega" => Some("ω"),
        "Gamma" => Some("Γ"),
        "Delta" => Some("Δ"),
        "Theta" => Some("Θ"),
        "Lambda" => Some("Λ"),
        "Xi" => Some("Ξ"),
        "Pi" => Some("Π"),
        "Sigma" => Some("Σ"),
        "Phi" => Some("Φ"),
        "Psi" => Some("Ψ"),
        "Omega" => Some("Ω"),
        // Common accented characters
        "Agrave" => Some("À"),
        "Aacute" => Some("Á"),
        "Acirc" => Some("Â"),
        "Atilde" => Some("Ã"),
        "Auml" => Some("Ä"),
        "Aring" => Some("Å"),
        "Ccedil" => Some("Ç"),
        "Egrave" => Some("È"),
        "Eacute" => Some("É"),
        "Ecirc" => Some("Ê"),
        "Euml" => Some("Ë"),
        "Igrave" => Some("Ì"),
        "Iacute" => Some("Í"),
        "Icirc" => Some("Î"),
        "Iuml" => Some("Ï"),
        "Ntilde" => Some("Ñ"),
        "Ograve" => Some("Ò"),
        "Oacute" => Some("Ó"),
        "Ocirc" => Some("Ô"),
        "Otilde" => Some("Õ"),
        "Ouml" => Some("Ö"),
        "Ugrave" => Some("Ù"),
        "Uacute" => Some("Ú"),
        "Ucirc" => Some("Û"),
        "Uuml" => Some("Ü"),
        "agrave" => Some("à"),
        "aacute" => Some("á"),
        "acirc" => Some("â"),
        "atilde" => Some("ã"),
        "auml" => Some("ä"),
        "aring" => Some("å"),
        "ccedil" => Some("ç"),
        "egrave" => Some("è"),
        "eacute" => Some("é"),
        "ecirc" => Some("ê"),
        "euml" => Some("ë"),
        "igrave" => Some("ì"),
        "iacute" => Some("í"),
        "icirc" => Some("î"),
        "iuml" => Some("ï"),
        "ntilde" => Some("ñ"),
        "ograve" => Some("ò"),
        "oacute" => Some("ó"),
        "ocirc" => Some("ô"),
        "otilde" => Some("õ"),
        "ouml" => Some("ö"),
        "ugrave" => Some("ù"),
        "uacute" => Some("ú"),
        "ucirc" => Some("û"),
        "uuml" => Some("ü"),
        "szlig" => Some("ß"),
        "eth" => Some("ð"),
        "thorn" => Some("þ"),
        "AElig" => Some("Æ"),
        "aelig" => Some("æ"),
        "OElig" => Some("Œ"),
        "oelig" => Some("œ"),
        _ => None,
    }
}

/// Decodes numeric character references: &#NNN; or &#xHHH;
fn decode_numeric_entity(num_str: &str) -> Option<char> {
    let code = if num_str.starts_with('x') || num_str.starts_with('X') {
        u32::from_str_radix(&num_str[1..], 16).ok()?
    } else {
        num_str.parse::<u32>().ok()?
    };
    char::from_u32(code)
}

fn sanitize_entry_fields(entry: &mut biblatex::Entry, chem_style: &ChemFormulaStyle) {
    // Text fields: strip tags / convert formula markup when tags are present,
    // and ALWAYS decode HTML entities — publishers (esp. APS) embed entities
    // like &beta;, &ndash;, &#8722; directly in tag-less titles.
    let text_fields = ["title", "abstract", "journal", "journaltitle"];

    for field in text_fields {
        if let Some(chunks) = entry.fields.get(field) {
            let raw_text = core::bib_to_string(chunks);
            let mut text = raw_text.clone();

            if text.contains('<') && text.contains('>') {
                // Check if source contains math/formula markup BEFORE stripping
                let has_math_markup = has_formula_markup(&text);

                if has_math_markup && *chem_style != ChemFormulaStyle::None {
                    // Convert <sub>/<sup>/<math> to LaTeX FIRST, then strip remaining
                    // tags. strip_tags() also decodes HTML entities.
                    text = convert_html_math_to_latex(&text, chem_style);
                    text = strip_tags(&text);
                } else {
                    // No formula markup (or feature disabled) — strip tags normally
                    // (strip_tags() also decodes HTML entities).
                    text = strip_tags(&text);
                }
            } else if text.contains('&') {
                // No tags, but possible HTML entities in a tag-less title/journal.
                text = decode_html_entities(&text);
            }

            // Only update the field if something changed
            if text != raw_text {
                // Use make_mixed_chunks so $...$ math is stored as Chunk::Math,
                // preventing double-escaping by to_bibtex_string() and encode_latex
                entry.fields.insert(field.into(), make_mixed_chunks(&text));
            }
        }
    }

    // Name fields: decode HTML entities ONLY (e.g. M&uuml;ller -> Müller).
    // Never run tag/formula conversion here — <i>/<sub> logic is meaningless in
    // a person name, and a stray entity would otherwise pollute key generation.
    let name_fields = ["author", "editor"];

    for field in name_fields {
        if let Some(chunks) = entry.fields.get(field) {
            let raw_text = core::bib_to_string(chunks);
            if raw_text.contains('&') {
                let decoded = decode_html_entities(&raw_text);
                if decoded != raw_text {
                    // Names carry no math; store as a plain Normal chunk so the
                    // "and"-separated structure is preserved for author parsing.
                    entry.fields.insert(field.into(), make_normal_chunk(&decoded));
                }
            }
        }
    }
}

/// Returns true if the raw HTML text contains sub/sup/math tags indicating formulas.
fn has_formula_markup(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("<sub>")
        || lower.contains("<sub ")
        || lower.contains("<sup>")
        || lower.contains("<sup ")
        || lower.contains("<math>")
        || lower.contains("<math ")
        || lower.contains("<mml:math")
}

/// Converts HTML formula markup to LaTeX math notation.
///
/// This function finds segments containing `<sub>` or `<sup>` tags and converts them
/// to proper LaTeX. It works on the raw HTML *before* general tag stripping.
///
/// The conversion wraps each formula segment in `$\mathrm{...}$` or `$\ce{...}$`
/// depending on the user's preference.
///
/// Examples (with Math style):
///   "CO<sub>2</sub>"                  -> "CO$_{2}$"  (inline subscript)
///   "Ni<sub>2-x</sub>Cl<sub>2+x</sub>" -> "$\mathrm{Ni_{2-x}Cl_{2+x}}$"
///
/// Examples (with Mhchem style):
///   "CO<sub>2</sub>"                  -> "$\ce{CO_{2}}$"
fn convert_html_math_to_latex(text: &str, style: &ChemFormulaStyle) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the next <sub> or <sup> tag
        let sub_pos = find_tag_case_insensitive(remaining, "<sub");
        let sup_pos = find_tag_case_insensitive(remaining, "<sup");

        let next_pos = match (sub_pos, sup_pos) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        match next_pos {
            None => {
                // No more formula tags — append the rest and done
                result.push_str(remaining);
                break;
            }
            Some(tag_start) => {
                // Look backwards from the tag to find the start of the formula context.
                // We want to capture element symbols immediately before <sub>/<sup>.
                // E.g., in "study of CO<sub>2</sub> in", we want "CO<sub>2</sub>".
                let prefix = &remaining[..tag_start];
                let formula_text_start = find_formula_prefix_start(prefix);

                // Append everything before the formula
                result.push_str(&remaining[..formula_text_start]);

                // Now extract the full formula span (prefix + all consecutive element<sub>...</sub> groups)
                let formula_source = &remaining[formula_text_start..];
                let (latex, consumed) = extract_and_convert_formula(formula_source, style);

                result.push_str(&latex);
                remaining = &remaining[formula_text_start + consumed..];
            }
        }
    }

    result
}

/// Finds a tag in a case-insensitive manner, returns byte offset in the original string.
/// FIX #8: Operates on original string indices to avoid Unicode offset drift.
fn find_tag_case_insensitive(text: &str, tag: &str) -> Option<usize> {
    let tag_bytes: Vec<u8> = tag.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let tag_len = tag_bytes.len();
    if tag_len == 0 || text.len() < tag_len {
        return None;
    }
    let text_bytes = text.as_bytes();
    for i in 0..=(text_bytes.len() - tag_len) {
        let mut matches = true;
        for j in 0..tag_len {
            if text_bytes[i + j].to_ascii_lowercase() != tag_bytes[j] {
                matches = false;
                break;
            }
        }
        if matches {
            return Some(i);
        }
    }
    None
}

/// Given the text before a `<sub>`/`<sup>` tag, finds where the element symbols start.
/// E.g., for "study of CO", returns the byte offset of 'C'.
/// FIX #15: Only walks backwards over ASCII characters to avoid multibyte boundary issues.
fn find_formula_prefix_start(prefix: &str) -> usize {
    let bytes = prefix.as_bytes();
    let mut pos = bytes.len();

    // Walk backwards over ASCII element-like characters only.
    // Non-ASCII bytes (>127) stop the walk, so we never land on a multibyte boundary.
    while pos > 0 {
        let c = bytes[pos - 1];
        if c.is_ascii_alphabetic() || c.is_ascii_digit() || c == b'.' {
            pos -= 1;
        } else {
            break;
        }
    }

    // Make sure we start at an uppercase letter (element symbol start)
    while pos < bytes.len() && !bytes[pos].is_ascii_uppercase() {
        pos += 1;
    }

    if pos >= prefix.len() {
        prefix.len()
    } else {
        pos
    }
}

/// Extracts a complete formula (possibly spanning multiple <sub>/<sup> groups)
/// and converts it to LaTeX. Returns (latex_string, bytes_consumed_from_source).
fn extract_and_convert_formula(source: &str, style: &ChemFormulaStyle) -> (String, usize) {
    let mut pos = 0;
    let bytes = source.as_bytes();
    let len = source.len();

    // Collect segments: alternating plain text and sub/sup content
    let mut segments: Vec<FormulaSegment> = Vec::new();

    while pos < len {
        // Check for <sub> or <sup>
        let rest = &source[pos..];
        let rest_lower = rest.to_lowercase();

        if rest_lower.starts_with("<sub") {
            // Find the end of the opening tag
            if let Some(gt) = rest.find('>') {
                let content_start = pos + gt + 1;
                // Find </sub>
                if let Some(close_offset) =
                    find_tag_case_insensitive(&source[content_start..], "</sub>")
                {
                    let content = &source[content_start..content_start + close_offset];
                    let close_tag_end = content_start + close_offset + 6; // len("</sub>") = 6
                    segments.push(FormulaSegment::Sub(strip_tags_simple(content)));
                    pos = close_tag_end;
                    continue;
                }
            }
            break; // Malformed tag — stop
        } else if rest_lower.starts_with("<sup") {
            if let Some(gt) = rest.find('>') {
                let content_start = pos + gt + 1;
                if let Some(close_offset) =
                    find_tag_case_insensitive(&source[content_start..], "</sup>")
                {
                    let content = &source[content_start..content_start + close_offset];
                    let close_tag_end = content_start + close_offset + 6; // len("</sup>") = 6
                    segments.push(FormulaSegment::Sup(strip_tags_simple(content)));
                    pos = close_tag_end;
                    continue;
                }
            }
            break;
        } else if rest_lower.starts_with("<") {
            // Some other HTML tag — stop the formula here
            break;
        } else if pos < len && bytes[pos].is_ascii_alphabetic() {
            // Plain element text between tags (e.g., the "Cl" between "</sub>Cl<sub>")
            let start = pos;
            while pos < len && bytes[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            segments.push(FormulaSegment::Text(source[start..pos].to_string()));
        } else if pos < len && (bytes[pos].is_ascii_digit() || bytes[pos] == b'.') {
            // Bare digit after element but before next tag
            let start = pos;
            while pos < len && (bytes[pos].is_ascii_digit() || bytes[pos] == b'.') {
                pos += 1;
            }
            segments.push(FormulaSegment::Sub(source[start..pos].to_string()));
        } else {
            // Non-formula character (space, punctuation) — formula ends
            break;
        }
    }

    if segments.is_empty() {
        // Nothing to convert — return source text up to first non-alpha
        let end = source
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(source.len());
        return (source[..end].to_string(), end);
    }

    // Build the LaTeX output
    let latex = match style {
        ChemFormulaStyle::Math => {
            let mut inner = String::new();
            for seg in &segments {
                match seg {
                    FormulaSegment::Text(t) => inner.push_str(t),
                    FormulaSegment::Sub(s) => {
                        inner.push_str("_{");
                        inner.push_str(s);
                        inner.push('}');
                    }
                    FormulaSegment::Sup(s) => {
                        inner.push_str("^{");
                        inner.push_str(s);
                        inner.push('}');
                    }
                }
            }
            format!("$\\mathrm{{{}}}$", inner)
        }
        ChemFormulaStyle::Mhchem => {
            // mhchem handles subscripts natively — bare numbers after elements
            // are automatically subscripted, so CO2 renders as CO₂.
            // Only use ^{} for superscripts (charge notation like 2+).
            let mut inner = String::new();
            for seg in &segments {
                match seg {
                    FormulaSegment::Text(t) => inner.push_str(t),
                    FormulaSegment::Sub(s) => {
                        // mhchem: bare numbers are auto-subscripted.
                        // For complex subscripts like "1-x", use _{}.
                        if s.chars().all(|c| c.is_ascii_digit()) {
                            inner.push_str(s);
                        } else {
                            inner.push_str("_{");
                            inner.push_str(s);
                            inner.push('}');
                        }
                    }
                    FormulaSegment::Sup(s) => {
                        inner.push_str("^{");
                        inner.push_str(s);
                        inner.push('}');
                    }
                }
            }
            format!("$\\ce{{{}}}$", inner)
        }
        ChemFormulaStyle::None => {
            // Shouldn't reach here, but just flatten
            let mut inner = String::new();
            for seg in &segments {
                match seg {
                    FormulaSegment::Text(t) => inner.push_str(t),
                    FormulaSegment::Sub(s) | FormulaSegment::Sup(s) => inner.push_str(s),
                }
            }
            inner
        }
    };

    (latex, pos)
}

#[derive(Debug)]
enum FormulaSegment {
    Text(String),
    Sub(String),
    Sup(String),
}

/// Simple tag stripper for content inside <sub>/<sup> (handles nested tags like <i>)
fn strip_tags_simple(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

fn refresh_ui_list(model: &mut AppModel) {
    model.entries.guard().clear();
    for entry in model.bibliography.iter() {
        model.entries.guard().push_back(BibEntry::from_entry(entry));
    }
}

// ----------------------------------------------------------------------------
// 2. Core Actions
// ----------------------------------------------------------------------------

pub fn add_entry(model: &mut AppModel, mut entry: biblatex::Entry) {
    model.push_snapshot();
    sanitize_entry_fields(&mut entry, &model.key_config.chem_formula_style);

    if model.key_config.abbreviate_journals {
        if let Some(chunk_val) = entry.fields.get("journal") {
            let original = core::bib_to_string(chunk_val);
            let abbr = abbreviator::abbreviate_journal(&original);
            if !abbr.is_empty() && abbr != original {
                entry
                    .fields
                    .insert("journal".into(), make_normal_chunk(&abbr));
            }
        }
        if let Some(chunk_val) = entry.fields.get("journaltitle") {
            let original = core::bib_to_string(chunk_val);
            let abbr = abbreviator::abbreviate_journal(&original);
            if !abbr.is_empty() && abbr != original {
                entry
                    .fields
                    .insert("journaltitle".into(), make_normal_chunk(&abbr));
            }
        }
    }

    if entry.key.is_empty() {
        entry.key = core::keygen::generate_key(&entry, &model.key_config);
    }

    let unique_key = ensure_unique(&entry.key, &model.bibliography);
    entry.key = unique_key.clone();

    model.bibliography.insert(entry.clone());
    model
        .entries
        .guard()
        .push_front(BibEntry::from_entry(&entry));
    model.is_dirty = true;
    model.sidebar.emit(SidebarMsg::SetStatus(format!(
        "Added entry: {}",
        unique_key
    )));
}

pub fn handle_row_output(model: &mut AppModel, output: BibEntryOutput) {
    match output {
        BibEntryOutput::Delete(key) => {
            model.push_snapshot();
            model.bibliography.remove(&key);
            let index_to_remove = model.entries.iter().position(|e| e.key == key);
            if let Some(idx) = index_to_remove {
                model.entries.guard().remove(idx);
            }
            model.is_dirty = true;
            model
                .sidebar
                .emit(SidebarMsg::SetStatus(format!("Deleted entry: {}", key)));
        }
        BibEntryOutput::Select(key) => {
            if let Some(entry) = model.bibliography.get(&key) {
                let content = entry
                    .to_bibtex_string()
                    .unwrap_or_else(|e| format!("% Error generating BibTeX: {}", e));
                model
                    .details_dialog
                    .emit(DetailsDialogMsg::Open(key, content));
            }
        }
    }
}

// ----------------------------------------------------------------------------
// 3. Editing Logic
// ----------------------------------------------------------------------------

pub fn finish_edit(
    model: &mut AppModel,
    old_key: String,
    content: String,
    _sender: ComponentSender<AppModel>,
) {
    let content = core::normalize_month_macros(&content);
    let parsed = Bibliography::parse(&content);

    match parsed {
        Ok(bib) => {
            // ✅ FIX: Get reference, then CLONE to own it.
            if let Some(entry_ref) = bib.iter().next() {
                model.push_snapshot();

                // Clone it so we can mutate it (strip spans)
                let mut new_entry = entry_ref.clone();

                // ✅ CRITICAL: Strip spans to prevent reading garbage from original file
                for chunks in new_entry.fields.values_mut() {
                    for chunk in chunks {
                        chunk.span = 0..0;
                    }
                }

                model.bibliography.remove(&old_key);

                let final_key = ensure_unique(&new_entry.key, &model.bibliography);
                new_entry.key = final_key.clone();

                model.bibliography.insert(new_entry);
                refresh_ui_list(model);

                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus(format!("Saved entry: {}", final_key)));
                model.is_dirty = true;
            } else {
                model
                    .alert
                    .emit(AlertMsg::Show("Error: No valid entry found.".into()));
            }
        }
        Err(e) => {
            model
                .alert
                .emit(AlertMsg::Show(format!("Parse Error:\n{}", e)));
        }
    }
}

// ----------------------------------------------------------------------------
// 4. Batch Operations
// ----------------------------------------------------------------------------

pub fn regenerate_keys(model: &mut AppModel, _sender: ComponentSender<AppModel>) {
    model.push_snapshot();

    let mut new_bib = Bibliography::new();
    let config = &model.key_config;
    let mut count = 0;

    for entry in model.bibliography.iter() {
        let mut new_entry = entry.clone();
        let new_key = crate::core::keygen::generate_key(&new_entry, config);
        let unique_key = ensure_unique(&new_key, &new_bib);
        new_entry.key = unique_key;
        new_bib.insert(new_entry);
        count += 1;
    }

    model.bibliography = new_bib;
    refresh_ui_list(model);
    model.is_dirty = true;
    model.sidebar.emit(SidebarMsg::SetStatus(format!(
        "Regenerated {} keys.",
        count
    )));
}

pub fn abbreviate_all_entries(model: &mut AppModel) {
    model.push_snapshot();
    let mut count = 0;
    let keys: Vec<String> = model.bibliography.iter().map(|e| e.key.clone()).collect();

    for key in keys {
        if let Some(entry) = model.bibliography.get_mut(&key) {
            let mut changed = false;

            // Removed `mut` warning here
            let check_field =
                |field_name: &str, fields: &mut BTreeMap<String, Vec<Spanned<Chunk>>>| -> bool {
                    if let Some(chunk_val) = fields.get(field_name) {
                        let current_text = core::bib_to_string(chunk_val);
                        let abbrev = abbreviator::abbreviate_journal(&current_text);
                        if !abbrev.is_empty() && abbrev != current_text {
                            fields.insert(field_name.into(), make_normal_chunk(&abbrev));
                            return true;
                        }
                    }
                    false
                };

            if check_field("journal", &mut entry.fields) {
                changed = true;
            }
            if check_field("journaltitle", &mut entry.fields) {
                changed = true;
            }

            if changed {
                count += 1;
            }
        }
    }

    if count > 0 {
        refresh_ui_list(model);
        model.is_dirty = true;
        model.sidebar.emit(SidebarMsg::SetStatus(format!(
            "Abbreviated {} journals.",
            count
        )));
    } else {
        model.undo_stack.pop_back();
        model.sidebar.emit(SidebarMsg::SetStatus(
            "No journals found to abbreviate.".to_string(),
        ));
    }
}

pub fn unabbreviate_all_entries(model: &mut AppModel) {
    model.push_snapshot();
    let mut count = 0;
    let keys: Vec<String> = model.bibliography.iter().map(|e| e.key.clone()).collect();

    for key in keys {
        if let Some(entry) = model.bibliography.get_mut(&key) {
            let mut changed = false;

            // Removed `mut` warning here
            let check_field =
                |field_name: &str, fields: &mut BTreeMap<String, Vec<Spanned<Chunk>>>| -> bool {
                    if let Some(chunk_val) = fields.get(field_name) {
                        let current_text = core::bib_to_string(chunk_val);
                        if let Some(full) = abbreviator::unabbreviate_journal(&current_text) {
                            if full != current_text {
                                fields.insert(field_name.into(), make_normal_chunk(&full));
                                return true;
                            }
                        }
                    }
                    false
                };

            if check_field("journal", &mut entry.fields) {
                changed = true;
            }
            if check_field("journaltitle", &mut entry.fields) {
                changed = true;
            }

            if changed {
                count += 1;
            }
        }
    }

    if count > 0 {
        refresh_ui_list(model);
        model.is_dirty = true;
        model.sidebar.emit(SidebarMsg::SetStatus(format!(
            "Expanded {} journals.",
            count
        )));
    } else {
        model.undo_stack.pop_back();
        model.sidebar.emit(SidebarMsg::SetStatus(
            "No abbreviations found to expand.".to_string(),
        ));
    }
}

/// Reformat All: Re-processes every entry with current settings.
///
/// This applies the current preferences to all entries:
///   - Normalizes chunks (cleans legacy escapes, re-splits Normal/Math,
///     applies chemical formula style with conversion between styles)
///   - Re-abbreviates or un-abbreviates journals based on config
///   - Strips all spans to 0..0 so the formatter uses the clean path
///
/// NOTE: Does NOT re-run sanitize_entry_fields — that is an import-time
/// operation for HTML cleanup. Already-loaded entries have clean chunk data.
/// Running sanitize again would flatten Math chunks via bib_to_string,
/// destroying the Normal/Math boundaries that normalize_field_chunks
/// carefully establishes.
///
/// The actual reformatting (indent, field order, Unicode mode) happens
/// at save time via the formatter, but this ensures the data is clean.
pub fn reformat_all_entries(model: &mut AppModel) {
    model.push_snapshot();

    let keys: Vec<String> = model.bibliography.iter().map(|e| e.key.clone()).collect();
    let chem_style = model.key_config.chem_formula_style.clone();
    let abbreviate = model.key_config.abbreviate_journals;
    let mut count = 0;

    for key in keys {
        if let Some(entry) = model.bibliography.get_mut(&key) {
            // 1. Strip all spans to force clean formatter path on save
            for chunks in entry.fields.values_mut() {
                for chunk in chunks.iter_mut() {
                    chunk.span = 0..0;
                }
            }

            // 2. Normalize chunks: flatten Normal/Math, clean legacy escapes,
            //    re-split into proper Normal/Math chunks, apply chem style.
            //    Verbatim chunks are preserved as-is.
            let field_keys: Vec<String> = entry.fields.keys().cloned().collect();
            for field_key in field_keys {
                if let Some(chunks) = entry.fields.get(&field_key) {
                    let normalized = normalize_field_chunks(chunks, &chem_style);
                    entry.fields.insert(field_key, normalized);
                }
            }

            // 3. Journal abbreviation/unabbreviation
            if abbreviate {
                for field_name in &["journal", "journaltitle"] {
                    if let Some(chunk_val) = entry.fields.get(*field_name) {
                        let original = core::bib_to_string(chunk_val);
                        let abbr = abbreviator::abbreviate_journal(&original);
                        if !abbr.is_empty() && abbr != original {
                            entry
                                .fields
                                .insert((*field_name).into(), make_normal_chunk(&abbr));
                        }
                    }
                }
            }

            count += 1;
        }
    }

    refresh_ui_list(model);
    model.is_dirty = true;
    model.sidebar.emit(SidebarMsg::SetStatus(format!(
        "Reformatted {} entries.",
        count
    )));
}

#[cfg(test)]
mod sanitize_tests {
    use super::*;
    use biblatex::Bibliography;

    /// Parse a one-entry BibTeX string, run sanitize_entry_fields, and return
    /// the flattened text of `field`.
    fn sanitized_field(src: &str, field: &str, chem: ChemFormulaStyle) -> String {
        let bib = Bibliography::parse(src).expect("parse");
        let mut entry = bib.iter().next().expect("one entry").clone();
        sanitize_entry_fields(&mut entry, &chem);
        core::bib_to_string(entry.fields.get(field).expect("field present"))
    }

    // Gap 1: entities in a TAG-LESS title must be decoded.
    #[test]
    fn tagless_title_entities_are_decoded() {
        let src = r#"@article{k, title = {Superconductivity in &beta;-pyrochlore, 20&ndash;30 K, &#8722;5 meV}}"#;
        let got = sanitized_field(src, "title", ChemFormulaStyle::None);
        assert_eq!(got, "Superconductivity in β-pyrochlore, 20–30 K, −5 meV");
    }

    // Gap 2: entities in author names must be decoded (entity-only path).
    #[test]
    fn author_entities_are_decoded() {
        let src = r#"@article{k, author = {M&uuml;ller, Kurt and Kr&auml;mer, Ann}}"#;
        let got = sanitized_field(src, "author", ChemFormulaStyle::None);
        assert_eq!(got, "Müller, Kurt and Krämer, Ann");
    }

    // Editor field uses the same entity-only path.
    #[test]
    fn editor_entities_are_decoded() {
        let src = r#"@book{k, editor = {D'Angelo, Jos&eacute;}}"#;
        let got = sanitized_field(src, "editor", ChemFormulaStyle::None);
        assert_eq!(got, "D'Angelo, José");
    }

    // Regression: the tags-present path still converts <i> and decodes entities.
    #[test]
    fn tagged_title_still_converts_and_decodes() {
        let src = r#"@article{k, title = {Higgs <i>boson</i> at &beta;-decay}}"#;
        let got = sanitized_field(src, "title", ChemFormulaStyle::None);
        assert_eq!(got, "Higgs \\textit{boson} at β-decay");
    }

    // Guard: a URL/Verbatim field with a bare & must NOT be touched.
    #[test]
    fn url_ampersand_is_left_untouched() {
        let src = r#"@article{k, title = {Plain}, url = {https://x.org/q?a=1&b=2}}"#;
        let got = sanitized_field(src, "url", ChemFormulaStyle::None);
        assert_eq!(got, "https://x.org/q?a=1&b=2");
    }

    // Guard: a clean title with no tags and no entities is unchanged (and no
    // spurious rewrite that could reinterpret it).
    #[test]
    fn plain_title_is_unchanged() {
        let src = r#"@article{k, title = {A simple clean title}}"#;
        let got = sanitized_field(src, "title", ChemFormulaStyle::None);
        assert_eq!(got, "A simple clean title");
    }
}
