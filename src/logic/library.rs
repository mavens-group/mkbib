// src/logic/library.rs

use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::core::{self, normalize};
use crate::ui::details_dialog::DetailsDialogMsg;
use crate::ui::row::{BibEntry, BibEntryOutput};
use crate::ui::sidebar::SidebarMsg;
use biblatex::Bibliography;
use relm4::{ComponentController, ComponentSender};
use std::collections::HashMap;

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

pub fn add_entry(model: &mut AppModel, mut entry: biblatex::Entry) {
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

            // FIX: Borrow checker E0597 - Force evaluation inside the block
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

pub fn scan_duplicates(model: &mut AppModel) {
    let mut fingerprints: HashMap<String, Vec<String>> = HashMap::new();
    for entry in model.bibliography.iter() {
        let title = entry
            .fields
            .get("title")
            .map(|t| core::bib_to_string(t))
            .unwrap_or_default();
        let year = entry
            .fields
            .get("year")
            .map(|t| core::bib_to_string(t))
            .unwrap_or_default();
        let author = entry
            .fields
            .get("author")
            .map(|t| core::bib_to_string(t))
            .unwrap_or_default();

        let fp = format!(
            "{}|{}|{}",
            normalize(&title),
            normalize(&year),
            normalize(&author)
        );
        fingerprints.entry(fp).or_default().push(entry.key.clone());
    }

    let mut report = String::new();
    let mut duplicate_count = 0;
    for (_fp, keys) in fingerprints {
        if keys.len() > 1 {
            duplicate_count += 1;
            report.push_str(&format!("• Group {}:\n", duplicate_count));
            for key in keys {
                report.push_str(&format!("   - {}\n", key));
            }
            report.push('\n');
        }
    }

    if duplicate_count == 0 {
        model
            .sidebar
            .emit(SidebarMsg::SetStatus("Library clean.".to_string()));
        model.alert.emit(AlertMsg::ShowInfo(
            "Great news!\n\nNo duplicate entries found.".into(),
        ));
    } else {
        model.sidebar.emit(SidebarMsg::SetStatus(format!(
            "Found {} duplicate groups.",
            duplicate_count
        )));
        model.alert.emit(AlertMsg::ShowInfo(format!(
            "Found duplicates:\n\n{}",
            report
        )));
    }
}

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

                // FIX: Borrow checker E0597 - Force evaluation inside the block
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
