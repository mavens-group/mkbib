// src/logic/library.rs
// #![allow(unused_assignments)]

use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::core;
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
    let mut i = 1;
    loop {
        let candidate = format!("{}_{}", base_key, i);
        if bib.get(&candidate).is_none() {
            return candidate;
        }
        i += 1;
    }
}

fn make_normal_chunk(s: &str) -> Vec<Spanned<Chunk>> {
    vec![Spanned {
        v: Chunk::Normal(s.to_string()),
        span: 0..0, // Dummy span
    }]
}

// ✅ FIXED HELPER: Smartly handles spacing based on tag type
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

            // Analyze the tag we just finished
            let lower_tag = current_tag_name.to_lowercase();

            // HEURISTIC: Only add spaces for "block" or "math container" tags.
            // "math" covers <math>, <mml:math>, </math> etc.
            // "br", "p", "div" are standard separators.
            if lower_tag.contains("math")
                || lower_tag == "br"
                || lower_tag == "p"
                || lower_tag == "div"
            {
                output.push(' ');
            }
            // Inline tags (mi, mn, mo, b, i, sup, sub) are stripped silently (no space added).
        } else if inside_tag {
            // Record tag name (stop at space to ignore attributes like display="inline")
            if !c.is_whitespace() {
                current_tag_name.push(c);
            }
        } else {
            output.push(c);
        }
    }

    // Decode entities
    let decoded = output
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    // ✅ SQUASH: Clean up any double spaces created by the logic above
    decoded.split_whitespace().collect::<Vec<&str>>().join(" ")
}

// ✅ NEW HELPER: Cleans specific fields in an entry
fn sanitize_entry_fields(entry: &mut biblatex::Entry) {
    let fields_to_clean = ["title", "abstract", "journal", "journaltitle"];

    for field in fields_to_clean {
        if let Some(chunks) = entry.fields.get(field) {
            let raw_text = core::bib_to_string(chunks);
            // Check if it looks like it has tags
            if raw_text.contains('<') && raw_text.contains('>') {
                let clean_text = strip_tags(&raw_text);
                entry
                    .fields
                    .insert(field.into(), make_normal_chunk(&clean_text));
            }
        }
    }
}

// Helper to refresh UI without repeating code
fn refresh_ui_list(model: &mut AppModel) {
    model.entries.guard().clear();
    for entry in model.bibliography.iter() {
        model.entries.guard().push_back(BibEntry::from_entry(entry));
    }
}

// ----------------------------------------------------------------------------
// 2. Core Actions (Add, Delete, etc.)
// ----------------------------------------------------------------------------

pub fn add_entry(model: &mut AppModel, mut entry: biblatex::Entry) {
    // 1. SAVE STATE
    model.push_snapshot();

    // 2. Logic: Sanitize Input (Fixes MathML titles)
    sanitize_entry_fields(&mut entry);

    // 3. Logic: Abbreviate on add if configured
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
        // Handle journaltitle as well
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

    // 4. Generate Key
    if entry.key.is_empty() {
        entry.key = core::keygen::generate_key(&entry, &model.key_config);
    }

    // 5. Ensure Uniqueness
    let unique_key = ensure_unique(&entry.key, &model.bibliography);
    entry.key = unique_key.clone();

    // 6. Insert
    model.bibliography.insert(entry.clone());

    // 7. Update UI
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
            // 1. SAVE STATE
            model.push_snapshot();

            // 2. Remove from Data
            model.bibliography.remove(&key);

            // 3. Remove from UI (Find index first to avoid borrow error)
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
            // Selection doesn't change state, so no snapshot needed
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
    // 1. Parse the edited content
    let parsed = Bibliography::parse(&content);

    match parsed {
        Ok(bib) => {
            if let Some(mut new_entry) = bib.iter().next() {
                // ✅ FIX: Take a Snapshot BEFORE applying changes
                model.push_snapshot();

                // Note: We do NOT sanitize here because this is a Manual Edit.

                // 2. Remove old entry
                model.bibliography.remove(&old_key);

                // 3. Insert new entry (handle key change automatically)
                let final_key = ensure_unique(&new_entry.key, &model.bibliography);

                let mut entry_to_insert = new_entry.clone();
                entry_to_insert.key = final_key.clone();

                model.bibliography.insert(entry_to_insert);

                // 4. Update UI
                refresh_ui_list(model);

                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus(format!("Saved entry: {}", final_key)));
                model.is_dirty = true;
            } else {
                model.alert.emit(AlertMsg::Show(
                    "Error: No valid entry found in the text.".into(),
                ));
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
    // Iterate over KEYS to avoid borrowing issues while mutating
    let keys: Vec<String> = model.bibliography.iter().map(|e| e.key.clone()).collect();

    for key in keys {
        if let Some(entry) = model.bibliography.get_mut(&key) {
            let mut changed = false;

            let mut check_field =
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

            let mut check_field =
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
