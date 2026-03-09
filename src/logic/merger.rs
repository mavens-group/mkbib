// src/logic/merger.rs

use crate::core::keygen::KeyGenConfig;
use biblatex::Bibliography;
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

    let mut output = String::with_capacity(original.len());
    let mut last_pos = 0;
    let mut processed_keys = HashSet::new();

    let mut bib_lookup = HashMap::new();
    for entry in bib.iter() {
        bib_lookup.insert(entry.key.to_lowercase(), entry);
    }

    for span in &spans {
        // 1. GAP: Preserve exact spacing/comments/@string/@preamble blocks
        output.push_str(&original[last_pos..span.start]);

        let key_lower = span.key.to_lowercase();

        if let Some(curr_entry) = bib_lookup.get(&key_lower) {
            // Always reformat through the clean formatter
            let serialized = crate::logic::formatter::format_entry(curr_entry, config);
            output.push_str(serialized.trim());

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
            let serialized = crate::logic::formatter::format_entry(entry, config);

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
        out.push_str(crate::logic::formatter::format_entry(entry, config).trim());
        out.push('\n');
    }
    out
}

/// FIX #3: Returns Vec instead of HashMap so duplicate keys don't collide.
/// FIX #4: @preamble/@string/@comment bodies are consumed before continue.
fn scan_entry_spans(text: &str) -> Vec<EntrySpan> {
    let mut spans = Vec::new();
    let mut chars_iter = text.char_indices().peekable();

    while let Some((idx, c)) = chars_iter.next() {
        if c == '@' {
            let start = idx;

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

            // FIX #4: Consume brace-delimited body of non-entry blocks
            if entry_type.eq_ignore_ascii_case("preamble")
                || entry_type.eq_ignore_ascii_case("string")
                || entry_type.eq_ignore_ascii_case("comment")
            {
                while let Some((_, c)) = chars_iter.peek() {
                    if !c.is_whitespace() {
                        break;
                    }
                    chars_iter.next();
                }
                if let Some((_, c)) = chars_iter.peek() {
                    let (open, close) = if *c == '{' {
                        ('{', '}')
                    } else if *c == '(' {
                        ('(', ')')
                    } else {
                        continue;
                    };
                    chars_iter.next();
                    let mut depth = 1;
                    while let Some((_, ch)) = chars_iter.next() {
                        if ch == open {
                            depth += 1;
                        } else if ch == close {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }
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
                    spans.push(EntrySpan {
                        key,
                        start,
                        end: end_pos,
                    });
                }
            }
        }
    }

    spans
}
