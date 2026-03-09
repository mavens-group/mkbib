// src/logic/merger.rs

use crate::core::keygen::KeyGenConfig;
use biblatex::{Bibliography, Chunk, Entry};
use std::collections::{HashMap, HashSet};

struct EntrySpan {
    key: String,
    start: usize,
    end: usize,
}

pub fn merge_bibliography_into_source(
    original: &str,
    bib: &Bibliography,
    config: &KeyGenConfig,
) -> String {
    let spans = scan_entry_spans(original);

    if spans.is_empty() && !bib.is_empty() {
        return generate_clean_bibliography(bib, config);
    }

    let mut sorted_spans = spans.values().collect::<Vec<_>>();
    sorted_spans.sort_by_key(|s| s.start);

    let mut output = String::with_capacity(original.len());
    let mut last_pos = 0;
    let mut processed_keys = HashSet::new();

    let mut bib_lookup = HashMap::new();
    for entry in bib.iter() {
        bib_lookup.insert(entry.key.to_lowercase(), entry);
    }

    for span in sorted_spans {
        // 1. GAP: Preserve exact spacing/comments
        output.push_str(&original[last_pos..span.start]);

        let key_lower = span.key.to_lowercase();

        if let Some(curr_entry) = bib_lookup.get(&key_lower) {
            let original_text_slice = &original[span.start..span.end];

            // Check Preservation: If equivalent, keep the RAW original block
            let should_preserve = if let Ok(parsed_bib) = Bibliography::parse(original_text_slice) {
                if let Some(orig_entry) = parsed_bib.iter().next() {
                    entries_are_equivalent(orig_entry, curr_entry)
                } else {
                    false
                }
            } else {
                false
            };

            if should_preserve {
                output.push_str(original_text_slice);
            } else {
                // ⚠️ CHANGED ENTRY: Reformat using HYBRID source
                // This passes 'original' so the formatter can look up `{DNA}`
                let serialized =
                    crate::logic::formatter::format_entry(curr_entry, config, Some(original));
                output.push_str(serialized.trim());
            }

            processed_keys.insert(key_lower);
        } else {
            // Deleted entry: do not write anything
        }

        last_pos = span.end;
    }

    output.push_str(&original[last_pos..]);

    // New Entries
    for entry in bib.iter() {
        if !processed_keys.contains(&entry.key.to_lowercase()) {
            // New entries have no original source, pass None
            let serialized = crate::logic::formatter::format_entry(entry, config, None);

            if !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(serialized.trim());
        }
    }

    output
}

fn generate_clean_bibliography(bib: &Bibliography, config: &KeyGenConfig) -> String {
    let mut out = String::new();
    for entry in bib.iter() {
        out.push_str(crate::logic::formatter::format_entry(entry, config, None).trim());
        out.push('\n');
    }
    out
}

fn entries_are_equivalent(a: &Entry, b: &Entry) -> bool {
    if a.entry_type != b.entry_type {
        return false;
    }
    if a.key != b.key {
        return false;
    }
    if a.fields.len() != b.fields.len() {
        return false;
    }
    for (k, v_a) in &a.fields {
        if let Some(v_b) = b.fields.get(k) {
            if chunks_to_string(v_a) != chunks_to_string(v_b) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn chunks_to_string(chunks: &[biblatex::Spanned<Chunk>]) -> String {
    let mut s = String::new();
    for chunk in chunks {
        match &chunk.v {
            Chunk::Normal(t) => s.push_str(t),
            Chunk::Verbatim(t) => s.push_str(t),
            Chunk::Math(t) => s.push_str(t),
        }
    }
    s
}

fn scan_entry_spans(text: &str) -> HashMap<String, EntrySpan> {
    let mut spans = HashMap::new();
    let mut chars_iter = text.char_indices().peekable();
    while let Some((idx, c)) = chars_iter.next() {
        if c == '@' {
            let start = idx;
            // 1. Identify Type
            let type_start = if let Some((i, _)) = chars_iter.peek() {
                *i
            } else {
                continue;
            };
            while let Some((_, c)) = chars_iter.peek() {
                if c.is_whitespace() || *c == '{' || *c == '(' {
                    break;
                }
                chars_iter.next();
            }
            let type_end = if let Some((i, _)) = chars_iter.peek() {
                *i
            } else {
                text.len()
            };
            let entry_type = &text[type_start..type_end];

            if entry_type.eq_ignore_ascii_case("preamble")
                || entry_type.eq_ignore_ascii_case("string")
                || entry_type.eq_ignore_ascii_case("comment")
            {
                continue;
            }

            // 2. Delim
            while let Some((_, c)) = chars_iter.peek() {
                if !c.is_whitespace() {
                    break;
                }
                chars_iter.next();
            }

            let (open_delim, close_delim) = if let Some((_, c)) = chars_iter.peek() {
                if *c == '{' {
                    chars_iter.next();
                    ('{', '}')
                } else if *c == '(' {
                    chars_iter.next();
                    ('(', ')')
                } else {
                    continue;
                }
            } else {
                continue;
            };

            // 3. Key
            while let Some((_, c)) = chars_iter.peek() {
                if !c.is_whitespace() {
                    break;
                }
                chars_iter.next();
            }
            let key_start_opt = chars_iter.peek().map(|(i, _)| *i);
            let mut key_end = 0;
            if let Some(key_start) = key_start_opt {
                while let Some((i, c)) = chars_iter.peek() {
                    if *c == ',' || *c == close_delim || c.is_whitespace() {
                        key_end = *i;
                        break;
                    }
                    chars_iter.next();
                }
                let key = text[key_start..key_end].trim().to_string();

                // 4. End
                let mut depth = 1;
                let mut end_pos = 0;
                while let Some((i, c)) = chars_iter.next() {
                    if c == open_delim {
                        depth += 1;
                    } else if c == close_delim {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = i + c.len_utf8();
                            break;
                        }
                    }
                }
                if depth == 0 && !key.is_empty() {
                    spans.insert(
                        key.clone(),
                        EntrySpan {
                            key,
                            start,
                            end: end_pos,
                        },
                    );
                }
            }
        }
    }
    spans
}
