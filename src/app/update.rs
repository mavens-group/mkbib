// src/app/update.rs
use biblatex::Bibliography;
use relm4::prelude::*;
use relm4_components::open_dialog::{OpenDialogMsg, OpenDialogResponse};
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};
use std::collections::HashMap;

use super::alert::AlertMsg;
use super::model::{AppModel, AppMsg};
use crate::api;
use crate::core;
use crate::ui;
use crate::ui::details_dialog::DetailsDialogMsg;
use crate::ui::preferences::PreferencesMsg;
use crate::ui::row::BibEntryOutput;
use crate::ui::search_dialog::SearchDialogMsg;

// --- HELPERS ---

fn normalize(s: &str) -> String {
  s.chars()
    .filter(|c| c.is_alphanumeric())
    .map(|c| c.to_ascii_lowercase())
    .collect()
}

fn to_ui_entry(entry: &biblatex::Entry) -> ui::row::BibEntry {
  let title = entry
    .fields
    .get("title")
    .map(|t| core::bib_to_string(t))
    .unwrap_or_else(|| "Untitled".to_string());

  ui::row::BibEntry {
    key: entry.key.clone(),
    title,
    kind: format!("{}", entry.entry_type),
    is_error: false,
  }
}

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

// --- UPDATE HANDLER ---

pub fn handle_msg(model: &mut AppModel, msg: AppMsg, sender: ComponentSender<AppModel>) {
  match msg {
    // --- Inputs ---
    AppMsg::UpdateDoi(v) => model.doi_input = v,
    AppMsg::UpdateSearch(v) => model.search_input = v,
    AppMsg::UpdateManualBib(v) => model.manual_bib_input = v,

    // --- Manual Parse ---
    AppMsg::ParseManualBib => {
      let text = model.manual_bib_input.trim().to_string();
      if text.is_empty() {
        return;
      }
      match Bibliography::parse(&text) {
        Ok(bib) => {
          let mut count = 0;
          for entry in bib.iter() {
            sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
            count += 1;
          }
          model.manual_bib_input = String::new();
          model.status_msg = format!("Added {} manual entries.", count);
        }
        Err(e) => {
          model.status_msg = "Failed to parse manual entry.".to_string();
          model
            .alert
            .emit(AlertMsg::Show(format!("BibTeX Parse Error:\n{}", e)));
        }
      }
    }

    // --- Dialog Triggers ---
    AppMsg::TriggerOpen => model.open_dialog.emit(OpenDialogMsg::Open),
    AppMsg::TriggerSave => model
      .save_dialog
      .emit(SaveDialogMsg::SaveAs("library.bib".into())),
    AppMsg::TriggerSaveAs => model
      .save_dialog
      .emit(SaveDialogMsg::SaveAs("library.bib".into())),
    AppMsg::ShowPreferences => model.preferences.emit(PreferencesMsg::Show),

    // --- Duplicate Scanner ---
    AppMsg::ScanDuplicates => {
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
        model.status_msg = "Library clean.".to_string();
        model.alert.emit(AlertMsg::ShowInfo(
          "Great news!\n\nNo duplicate entries found.".into(),
        ));
      } else {
        model.status_msg = format!("Found {} duplicate groups.", duplicate_count);
        model.alert.emit(AlertMsg::ShowInfo(format!(
          "Found duplicates:\n\n{}",
          report
        )));
      }
    }

    // --- Fetch Logic (Single DOI) ---
    AppMsg::FetchDoi => {
      let doi = model.doi_input.trim().to_string();
      if doi.is_empty() {
        return;
      }
      model.is_loading = true;
      model.status_msg = format!("Fetching DOI: {}...", doi);

      let input = sender.input_sender().clone();
      sender.command(move |_out, _shutdown| async move {
        let result = match api::fetch_doi(&doi).await {
          Ok(bib) => AppMsg::FetchSuccess(bib),
          Err(e) => AppMsg::FetchError(e.to_string()),
        };
        input.send(result).expect("Failed to send async result");
      });
    }

    // --- Search Logic (Suggestions) ---
    AppMsg::FetchSearch => {
      let query = model.search_input.trim().to_string();
      if query.is_empty() {
        return;
      }
      model.is_loading = true;
      model.status_msg = format!("Searching Crossref for: {}...", query);

      let input = sender.input_sender().clone();
      sender.command(move |_out, _shutdown| async move {
        match api::search_crossref_suggestions(&query).await {
          Ok(items) => input.send(AppMsg::SearchResultsLoaded(items)).unwrap(),
          Err(e) => input.send(AppMsg::FetchError(e.to_string())).unwrap(),
        }
      });
    }

    // --- Search Results Handling ---
    AppMsg::SearchResultsLoaded(items) => {
      model.is_loading = false;
      if items.is_empty() {
        model.status_msg = "No results found.".to_string();
        model
          .alert
          .emit(AlertMsg::ShowInfo("No results found on Crossref.".into()));
      } else {
        model.status_msg = "Select an item to import.".to_string();
        model
          .search_dialog
          .emit(SearchDialogMsg::ShowResults(items));
      }
    }

    // FIX: Handle DOI selected from Search Dialog
    AppMsg::FetchSelectedDoi(doi) => {
      model.doi_input = doi;
      sender.input(AppMsg::FetchDoi);
    }

    // --- Async Results ---
    AppMsg::FetchSuccess(bib) => {
      model.is_loading = false;
      let mut count = 0;
      for entry in bib.iter() {
        sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
        count += 1;
      }
      if count > 0 {
        model.status_msg = format!("Successfully imported {} entry.", count);
        model.doi_input = String::new();
      } else {
        model.status_msg = "No entries found in response.".to_string();
      }
    }

    AppMsg::FetchError(err) => {
      model.is_loading = false;
      model.status_msg = "Error occurred.".to_string();
      model.alert.emit(AlertMsg::Show(err));
    }

    // --- Core Entry Management ---
    AppMsg::AddBiblatexEntry(mut entry) => {
      if entry.key.is_empty() {
        let new_key = core::keygen::generate_key(&entry, &model.key_config);
        entry.key = new_key;
      }

      let unique_key = ensure_unique(&entry.key, &model.bibliography);
      entry.key = unique_key.clone();

      model.bibliography.insert(entry.clone());
      model.entries.guard().push_front(to_ui_entry(&entry));
    }

    AppMsg::HandleRowOutput(output) => match output {
      BibEntryOutput::Delete(key) => {
        model.bibliography.remove(&key);
        // FIX: Calculate index first to drop immutable borrow, then remove
        let mut guard = model.entries.guard();
        let idx_opt = guard.iter().position(|e| e.key == key);
        if let Some(idx) = idx_opt {
          guard.remove(idx);
        }
        model.status_msg = format!("Deleted entry: {}", key);
      }
      BibEntryOutput::Select(key) => {
        if let Some(entry) = model.bibliography.get(&key) {
          // FIX: Handle the Result from entry.to_bibtex_string()
          let content = entry
            .to_bibtex_string()
            .unwrap_or_else(|e| format!("% Error generating BibTeX: {}", e));

          model
            .details_dialog
            .emit(DetailsDialogMsg::Open(key, content));
        }
      }
    },

    AppMsg::FinishEditEntry(original_key, new_content) => {
      match Bibliography::parse(&new_content) {
        Ok(mut bib) => {
          if let Some(new_entry) = bib.iter_mut().next() {
            model.bibliography.remove(&original_key);

            // FIX: Calculate index first to drop immutable borrow, then remove
            let mut guard = model.entries.guard();
            let idx_opt = guard.iter().position(|e| e.key == original_key);
            if let Some(idx) = idx_opt {
              guard.remove(idx);
            }
            drop(guard);

            sender.input(AppMsg::AddBiblatexEntry(new_entry.clone()));
            model.status_msg = "Entry updated.".to_string();
          }
        }
        Err(e) => {
          model
            .alert
            .emit(AlertMsg::Show(format!("Invalid BibTeX:\n{}", e)));
        }
      }
    }

    AppMsg::ClearAll => {
      model.bibliography = Bibliography::new();
      model.entries.guard().clear();
      model.status_msg = "Library cleared.".to_string();
    }

    AppMsg::RegenerateAllKeys => {
      let old_entries: Vec<_> = model.bibliography.iter().map(|e| e.clone()).collect();

      model.bibliography = Bibliography::new();
      model.entries.guard().clear();

      let mut count = 0;
      for mut entry in old_entries {
        entry.key = core::keygen::generate_key(&entry, &model.key_config);
        sender.input(AppMsg::AddBiblatexEntry(entry));
        count += 1;
      }
      model.status_msg = format!("Regenerated keys for {} entries.", count);
    }

    AppMsg::UpdateKeyConfig(config) => {
      model.key_config = config;
      core::config::save(&model.key_config);
      model.status_msg = "Preferences saved.".to_string();
    }

    // --- File I/O Responses ---
    AppMsg::OpenResponse(resp) => {
      if let OpenDialogResponse::Accept(path) = resp {
        if let Ok(content) = std::fs::read_to_string(&path) {
          model.status_msg = format!("Loading {}...", path.display());

          match Bibliography::parse(&content) {
            Ok(bib) => {
              let mut count = 0;
              for entry in bib.iter() {
                sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
                count += 1;
              }
              model.status_msg = format!("Loaded {} entries from file.", count);
            }
            Err(e) => {
              model
                .alert
                .emit(AlertMsg::Show(format!("Parse Error:\n{}", e)));
            }
          }
        } else {
          model
            .alert
            .emit(AlertMsg::Show("Failed to read file.".into()));
        }
      }
    }

    AppMsg::SaveResponse(resp) => {
      if let SaveDialogResponse::Accept(path) = resp {
        // FIX: bibliography.to_bibtex_string() returns String (not Result)
        let output = model.bibliography.to_bibtex_string();
        if std::fs::write(&path, output).is_ok() {
          model.status_msg = "Library saved.".to_string();
        } else {
          model
            .alert
            .emit(AlertMsg::Show("Failed to write file.".into()));
        }
      }
    }
  }
}
