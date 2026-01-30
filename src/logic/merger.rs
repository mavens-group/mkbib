// src/logic/merger.rs

use crate::core::keygen::KeyGenConfig;
use biblatex::Bibliography;
use std::collections::{HashMap, HashSet};

struct EntrySpan {
    key: String,
    start: usize, // Byte index
    end: usize,   // Byte index
}

pub fn merge_bibliography_into_source(
    original: &str,
    bib: &Bibliography,
    config: &KeyGenConfig,
) -> String {
    let spans = scan_entry_spans(original);

    // Fallback if parsing fails (should rarely happen with scan_entry_spans)
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
        // Write text BEFORE the entry (preserves existing newlines exactly)
        output.push_str(&original[last_pos..span.start]);

        let key_lower = span.key.to_lowercase();

        if let Some(entry) = bib_lookup.get(&key_lower) {
            // Write formatted entry (Tight, no extra newlines)
            let serialized = crate::logic::formatter::format_entry(entry, config);
            output.push_str(&serialized);
            processed_keys.insert(key_lower);
        } else {
            // Entry Deleted: we skip writing the original span (deleting it)
        }

        last_pos = span.end;
    }

    // Write tail
    output.push_str(&original[last_pos..]);

    // Append NEW entries (e.g. created via UI)
    // ✅ STRICT FIX: No extra newlines before/after
    for entry in bib.iter() {
        if !processed_keys.contains(&entry.key.to_lowercase()) {
            let serialized = crate::logic::formatter::format_entry(entry, config);

            // Only add a newline if the file doesn't end with one
            if !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&serialized);
            // We do NOT add a trailing newline here, to satisfy your request.
        }
    }

    output
}

fn generate_clean_bibliography(bib: &Bibliography, config: &KeyGenConfig) -> String {
    let mut out = String::new();
    for entry in bib.iter() {
        out.push_str(&crate::logic::formatter::format_entry(entry, config));
        // Minimal separation for clean file generation
        out.push('\n');
    }
    out
}

/// Robust scanner using char_indices for correct Byte Offsets
fn scan_entry_spans(text: &str) -> HashMap<String, EntrySpan> {
    let mut spans = HashMap::new();
    let mut chars_iter = text.char_indices().peekable();

    while let Some((idx, c)) = chars_iter.next() {
        if c == '@' {
            let start = idx;

            // 1. Skip Type
            while let Some((_, c)) = chars_iter.peek() {
                if c.is_whitespace() || *c == '{' || *c == '(' {
                    break;
                }
                chars_iter.next();
            }

            while let Some((_, c)) = chars_iter.peek() {
                if !c.is_whitespace() {
                    break;
                }
                chars_iter.next();
            }

            // 2. Delimiter
            let mut open_delim = '{';
            let mut close_delim = '}';

            if let Some((_, c)) = chars_iter.peek() {
                if *c == '{' {
                    open_delim = '{';
                    close_delim = '}';
                    chars_iter.next();
                } else if *c == '(' {
                    open_delim = '(';
                    close_delim = ')';
                    chars_iter.next();
                } else {
                    continue;
                }
            } else {
                continue;
            }

            // 3. Parse Key
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

                // 4. Find Matching End
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
