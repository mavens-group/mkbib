use biblatex::Bibliography;
use relm4::prelude::*;
use relm4_components::open_dialog::{OpenDialogMsg, OpenDialogResponse};
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};
use std::collections::{HashMap, HashSet};

use super::alert::AlertMsg;
use super::model::{AppModel, AppMsg};
use crate::api;
use crate::core;
use crate::ui;
use crate::ui::details_dialog::DetailsDialogMsg;
use crate::ui::preferences::PreferencesMsg;
use crate::ui::row::BibEntryOutput;
use crate::ui::search_dialog::SearchDialogMsg; // <--- NEW

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

      // Spawn command to get suggestions list
      sender.command(move |_out, _shutdown| async move {
        match api::search_crossref_suggestions(&query).await {
          Ok(items) => input.send(AppMsg::SearchResultsLoaded(items)).unwrap(),
          Err(e) => input.send(AppMsg::FetchError(e.to_string())).unwrap(),
        }
      });
    }

    // New: Results arrived, show dialog
    AppMsg::SearchResultsLoaded(items) => {
      model.is_loading = false;
      if items.is_empty() {
        model.status_msg = "No results found.".to_string();
        model
          .alert
          .emit(AlertMsg::ShowInfo("No results found on Crossref.".into()));
      } else {
        model.status_msg = format!("Found {} results.", items.len());
        model
          .search_dialog
          .emit(SearchDialogMsg::ShowResults(items));
      }
    }

    // New: User selected a result from dialog
    AppMsg::FetchSelectedDoi(doi) => {
      model.is_loading = true;
      model.status_msg = format!("Importing DOI: {}...", doi);

      let input = sender.input_sender().clone();
      sender.command(move |_out, _shutdown| async move {
        let result = match api::fetch_doi(&doi).await {
          Ok(bib) => AppMsg::FetchSuccess(bib),
          Err(e) => AppMsg::FetchError(e.to_string()),
        };
        input.send(result).expect("Failed to send async result");
      });
    }

    AppMsg::FetchSuccess(bib) => {
      model.is_loading = false;
      let count = bib.iter().count();
      model.status_msg = format!("Fetched {} entries.", count);
      for entry in bib.iter() {
        sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
      }
    }

    AppMsg::FetchError(err) => {
      model.is_loading = false;
      model.status_msg = "Fetch failed.".to_string();
      model
        .alert
        .emit(AlertMsg::Show(format!("Network Error:\n{}", err)));
    }

    // --- Core Logic ---
    AppMsg::AddBiblatexEntry(mut entry) => {
      let base_key = core::keygen::generate_key(&entry, &model.key_config);
      let unique_key = ensure_unique(&base_key, &model.bibliography);
      entry.key = unique_key;

      model.bibliography.insert(entry.clone());
      model.entries.guard().push_back(to_ui_entry(&entry));
    }

    // --- Row Actions ---
    AppMsg::HandleRowOutput(output) => match output {
      BibEntryOutput::Delete(key) => {
        model.bibliography.remove(&key);
        model.entries.guard().clear();
        for entry in model.bibliography.iter() {
          model.entries.guard().push_back(to_ui_entry(entry));
        }
        model.status_msg = format!("Deleted entry: {}", key);
      }
      BibEntryOutput::Select(key) => {
        if let Some(entry) = model.bibliography.get(&key) {
          let bib_string = entry
            .to_bibtex_string()
            .unwrap_or_else(|e| format!("% Error: {}", e));
          model
            .details_dialog
            .sender()
            .send(DetailsDialogMsg::Open(key, bib_string))
            .expect("Failed to open dialog");
        }
      }
    },

    AppMsg::FinishEditEntry(old_key, new_source) => match Bibliography::parse(&new_source) {
      Ok(bib) => {
        if let Some(new_entry) = bib.iter().next() {
          model.bibliography.remove(&old_key);
          model.bibliography.insert(new_entry.clone());
          model.entries.guard().clear();
          for entry in model.bibliography.iter() {
            model.entries.guard().push_back(to_ui_entry(entry));
          }
          model.status_msg = format!("Updated: {}", new_entry.key);
        } else {
          model
            .alert
            .emit(AlertMsg::Show("No entry found in text.".into()));
        }
      }
      Err(e) => {
        model
          .alert
          .emit(AlertMsg::Show(format!("Parse Error:\n{}", e)));
      }
    },

    AppMsg::ClearAll => {
      model.bibliography = Bibliography::new();
      model.entries.guard().clear();
      model.status_msg = "Library cleared.".to_string();
    }

    AppMsg::RegenerateAllKeys => {
      let all_entries: Vec<biblatex::Entry> = model.bibliography.iter().cloned().collect();
      model.bibliography = Bibliography::new();
      model.entries.guard().clear();
      let mut count = 0;
      for entry in all_entries {
        sender.input(AppMsg::AddBiblatexEntry(entry));
        count += 1;
      }
      model.status_msg = format!("Regenerated keys for {} entries.", count);
    }

    AppMsg::UpdateKeyConfig(cfg) => {
      model.key_config = cfg;
      core::config::save(&model.key_config);
      model.status_msg = "Configuration saved.".to_string();
    }

    // --- Dialog Responses ---
    AppMsg::OpenResponse(resp) => {
      if let OpenDialogResponse::Accept(path) = resp {
        match std::fs::read_to_string(&path) {
          Ok(content) => {
            let mut success_count = 0;
            let mut errors = Vec::new();
            match Bibliography::parse(&content) {
              Ok(bib) => {
                let mut batch_keys = HashSet::new();
                for entry in bib.iter() {
                  let key = entry.key.clone();
                  if batch_keys.contains(&key) {
                    continue;
                  }
                  batch_keys.insert(key);
                  sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
                  success_count += 1;
                }
              }
              Err(e) => errors.push(format!("Full Parse Error: {}", e)),
            }
            if errors.is_empty() {
              model.status_msg = format!("Loaded {} entries.", success_count);
            } else {
              model
                .alert
                .emit(AlertMsg::Show(format!("Errors:\n{:?}", errors)));
            }
          }
          Err(e) => model
            .alert
            .emit(AlertMsg::Show(format!("File Error:\n{}", e))),
        }
      }
    }

    AppMsg::SaveResponse(resp) => {
      if let SaveDialogResponse::Accept(path) = resp {
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
