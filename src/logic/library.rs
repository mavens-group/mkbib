// src/logic/library.rs
#![allow(unused_assignments)]

use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::core;
use crate::logic::abbreviator;
// use crate::logic::deduplicator;
use crate::ui::details_dialog::DetailsDialogMsg;
// use crate::ui::duplicate_dialog::DuplicateDialogMsg;
use crate::ui::row::{BibEntry, BibEntryOutput};
use crate::ui::sidebar::SidebarMsg;
use biblatex::{Bibliography, Chunk, Spanned};
use relm4::{ComponentController, ComponentSender};
use std::collections::BTreeMap;

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

// FIX: Use 0..0 instead of private Span type
fn make_normal_chunk(text: &str) -> Vec<Spanned<Chunk>> {
    vec![Spanned::new(Chunk::Normal(text.to_string()), 0..0)]
}

pub fn add_entry(model: &mut AppModel, mut entry: biblatex::Entry) {
    model.push_snapshot();
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
        // Also check journaltitle
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

    model
        .sidebar
        .emit(SidebarMsg::SetStatus(format!("Added entry: {}", entry.key)));
}

pub fn handle_row_output(model: &mut AppModel, output: BibEntryOutput) {
    match output {
        BibEntryOutput::Delete(key) => {
            model.bibliography.remove(&key);

            let idx_opt = {
                let guard = model.entries.guard();
                let pos = guard.iter().position(|e| e.key == key);
                pos
            };

            if let Some(idx) = idx_opt {
                model.entries.guard().remove(idx);
            }

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

// pub fn scan_duplicates(model: &mut AppModel) {
// model
// .sidebar
// .emit(SidebarMsg::SetStatus("Scanning for duplicates...".into()));

// // 1. Delegate math to deduplicator (Returns structured DuplicateGroup objects)
// let duplicates = deduplicator::find_duplicates(&model.bibliography);

// // 2. UI Feedback
// if duplicates.is_empty() {
// model
// .sidebar
// .emit(SidebarMsg::SetStatus("Library clean.".to_string()));
// model.alert.emit(AlertMsg::ShowInfo(
// "Great news!\n\nNo duplicate entries found.".into(),
// ));
// } else {
// model.sidebar.emit(SidebarMsg::SetStatus(format!(
// "Reviewing {} duplicate groups...",
// duplicates.len()
// )));

// // 3. DIAMOND LOGIC: Open the interactive dialog instead of a text alert
// model
// .duplicate_dialog
// .emit(DuplicateDialogMsg::LoadGroups(duplicates));
// }
// }

pub fn regenerate_keys(model: &mut AppModel, sender: ComponentSender<AppModel>) {
    let old_entries: Vec<_> = model.bibliography.iter().map(|e| e.clone()).collect();
    model.bibliography = Bibliography::new();
    model.entries.guard().clear();

    let mut count = 0;
    for mut entry in old_entries {
        entry.key = core::keygen::generate_key(&entry, &model.key_config);
        sender.input(AppMsg::AddBiblatexEntry(entry));
        count += 1;
    }
    model.sidebar.emit(SidebarMsg::SetStatus(format!(
        "Regenerated keys for {} entries.",
        count
    )));
}

pub fn finish_edit(
    model: &mut AppModel,
    original_key: String,
    new_content: String,
    sender: ComponentSender<AppModel>,
) {
    match Bibliography::parse(&new_content) {
        Ok(mut bib) => {
            if let Some(new_entry) = bib.iter_mut().next() {
                model.bibliography.remove(&original_key);

                let idx_opt = {
                    let guard = model.entries.guard();
                    let pos = guard.iter().position(|e| e.key == original_key);
                    pos
                };

                if let Some(idx) = idx_opt {
                    model.entries.guard().remove(idx);
                }

                sender.input(AppMsg::AddBiblatexEntry(new_entry.clone()));
                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus("Entry updated.".to_string()));
            }
        }
        Err(e) => {
            model
                .alert
                .emit(AlertMsg::Show(format!("Invalid BibTeX:\n{}", e)));
        }
    }
}

pub fn abbreviate_all_entries(model: &mut AppModel) {
    let mut count = 0;

    for entry in model.bibliography.iter_mut() {
        let mut changed = false;

        let check_field =
            |field_name: &str, fields: &mut BTreeMap<String, Vec<Spanned<Chunk>>>| -> bool {
                if let Some(chunk_val) = fields.get(field_name) {
                    let original = core::bib_to_string(chunk_val);
                    let abbr = abbreviator::abbreviate_journal(&original);

                    if !abbr.is_empty() && abbr != original {
                        fields.insert(field_name.into(), make_normal_chunk(&abbr));
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

    if count > 0 {
        model.entries.guard().clear();
        for entry in model.bibliography.iter() {
            model.entries.guard().push_back(BibEntry::from_entry(entry));
        }

        model.sidebar.emit(SidebarMsg::SetStatus(format!(
            "Abbreviated {} journals.",
            count
        )));
    } else {
        model.sidebar.emit(SidebarMsg::SetStatus(
            "No journals needed abbreviation.".to_string(),
        ));
    }
}

pub fn unabbreviate_all_entries(model: &mut AppModel) {
    let mut count = 0;

    for entry in model.bibliography.iter_mut() {
        let mut changed = false;

        let check_field =
            |field_name: &str, fields: &mut BTreeMap<String, Vec<Spanned<Chunk>>>| -> bool {
                if let Some(chunk_val) = fields.get(field_name) {
                    let current_text = core::bib_to_string(chunk_val);
                    // Try to find the full title
                    if let Some(full_title) = abbreviator::unabbreviate_journal(&current_text) {
                        if full_title != current_text {
                            fields.insert(field_name.into(), make_normal_chunk(&full_title));
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

    if count > 0 {
        // Refresh UI List
        model.entries.guard().clear();
        for entry in model.bibliography.iter() {
            model.entries.guard().push_back(BibEntry::from_entry(entry));
        }
        model.sidebar.emit(SidebarMsg::SetStatus(format!(
            "Expanded {} journals to full titles.",
            count
        )));
    } else {
        model.sidebar.emit(SidebarMsg::SetStatus(
            "No known abbreviations found to expand.".to_string(),
        ));
    }
}
