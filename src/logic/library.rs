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

/// Normalizes a field's chunks: flattens to text, cleans up legacy LaTeX
/// escapes, re-splits into proper Normal/Math chunks, and wraps bare
/// math content in the user's chosen chemical formula style.
fn normalize_field_chunks(
    chunks: &[Spanned<Chunk>],
    chem_style: &ChemFormulaStyle,
) -> Vec<Spanned<Chunk>> {
    // 1. Flatten all chunks into a single string, re-inserting $...$ for Math
    let mut flat = String::new();
    for chunk in chunks {
        match &chunk.v {
            Chunk::Normal(t) => flat.push_str(t),
            Chunk::Verbatim(t) => flat.push_str(t),
            Chunk::Math(t) => {
                flat.push('$');
                flat.push_str(t);
                flat.push('$');
            }
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
                // Skip if already wrapped in a command
                if trimmed.starts_with("\\mathrm{")
                    || trimmed.starts_with("\\ce{")
                    || trimmed.starts_with("\\text{")
                {
                    // Already has a wrapper — re-wrap only if switching styles
                    if *chem_style == ChemFormulaStyle::Math
                        && trimmed.starts_with("\\ce{")
                        && trimmed.ends_with('}')
                    {
                        // \ce{...} → \mathrm{...}
                        let inner = &trimmed[4..trimmed.len() - 1];
                        *content = format!("\\mathrm{{{}}}", inner);
                    } else if *chem_style == ChemFormulaStyle::Mhchem
                        && trimmed.starts_with("\\mathrm{")
                        && trimmed.ends_with('}')
                    {
                        // \mathrm{...} → \ce{...}
                        let inner = &trimmed[8..trimmed.len() - 1];
                        *content = format!("\\ce{{{}}}", inner);
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

    // Quick checks: if it contains Greek letters or = sign, probably pure math
    if math.contains("\\alpha")
        || math.contains("\\beta")
        || math.contains("\\gamma")
        || math.contains("\\delta")
        || math.contains("\\sigma")
        || math.contains("\\pi")
        || math.contains("\\mu")
        || math.contains("\\lambda")
        || math.contains("\\theta")
        || math.contains("\\epsilon")
        || math.contains('=')
    {
        return false;
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

// Fixed Helper: Smart tag stripping
fn strip_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;
    let mut current_tag_name = String::new();

    for c in input.chars() {
        if c == '<' {
            inside_tag = true;
            current_tag_name.clear();
        } else if c == '>' {
            inside_tag = false;
            let lower_tag = current_tag_name.to_lowercase();
            if lower_tag.contains("math")
                || lower_tag == "br"
                || lower_tag == "p"
                || lower_tag == "div"
            {
                output.push(' ');
            }
        } else if inside_tag {
            if !c.is_whitespace() {
                current_tag_name.push(c);
            }
        } else {
            output.push(c);
        }
    }

    let decoded = output
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    decoded.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn sanitize_entry_fields(entry: &mut biblatex::Entry, chem_style: &ChemFormulaStyle) {
    let fields_to_clean = ["title", "abstract", "journal", "journaltitle"];

    for field in fields_to_clean {
        if let Some(chunks) = entry.fields.get(field) {
            let raw_text = core::bib_to_string(chunks);
            let mut text = raw_text.clone();

            if text.contains('<') && text.contains('>') {
                // Check if source contains math/formula markup BEFORE stripping
                let has_math_markup = has_formula_markup(&text);

                if has_math_markup && *chem_style != ChemFormulaStyle::None {
                    // Convert <sub>/<sup>/<math> to LaTeX FIRST, then strip remaining tags
                    text = convert_html_math_to_latex(&text, chem_style);
                    text = strip_tags(&text);
                } else {
                    // No formula markup (or feature disabled) — just strip tags normally
                    text = strip_tags(&text);
                }
            }

            // Only update the field if something changed
            if text != raw_text {
                // Use make_mixed_chunks so $...$ math is stored as Chunk::Math,
                // preventing double-escaping by to_bibtex_string() and encode_latex
                entry.fields.insert(field.into(), make_mixed_chunks(&text));
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
///   - Re-runs sanitize (HTML stripping, chemical formula conversion)
///   - Re-abbreviates or un-abbreviates journals based on config
///   - Strips all spans to 0..0 so the formatter uses the clean path
///   - Regenerates keys if requested
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

            // 2. Normalize chunks: flatten, clean legacy escapes inside math,
            //    re-split into proper Normal/Math chunks, apply chem style
            let field_keys: Vec<String> = entry.fields.keys().cloned().collect();
            for field_key in field_keys {
                if let Some(chunks) = entry.fields.get(&field_key) {
                    let normalized = normalize_field_chunks(chunks, &chem_style);
                    entry.fields.insert(field_key, normalized);
                }
            }

            // 3. Re-run sanitize (HTML cleanup, chemical formula conversion)
            let mut owned_entry = entry.clone();
            sanitize_entry_fields(&mut owned_entry, &chem_style);

            // 4. Journal abbreviation/unabbreviation
            if abbreviate {
                for field_name in &["journal", "journaltitle"] {
                    if let Some(chunk_val) = owned_entry.fields.get(*field_name) {
                        let original = core::bib_to_string(chunk_val);
                        let abbr = abbreviator::abbreviate_journal(&original);
                        if !abbr.is_empty() && abbr != original {
                            owned_entry
                                .fields
                                .insert((*field_name).into(), make_normal_chunk(&abbr));
                        }
                    }
                }
            }

            // 5. Write back (remove old, insert new to update in place)
            let updated_key = owned_entry.key.clone();
            model.bibliography.remove(&key);
            model.bibliography.insert(owned_entry);
            count += 1;

            // Preserve key if it changed (shouldn't, but defensive)
            if updated_key != key {
                // Key was modified during sanitize — already handled by insert
            }
        }
    }

    refresh_ui_list(model);
    model.is_dirty = true;
    model.sidebar.emit(SidebarMsg::SetStatus(format!(
        "Reformatted {} entries.",
        count
    )));
}
