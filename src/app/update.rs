use biblatex::Bibliography;
use relm4::prelude::*;
use relm4_components::open_dialog::{OpenDialogMsg, OpenDialogResponse};
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};
use std::collections::HashSet;

use super::alert::AlertMsg;
use super::model::{AppModel, AppMsg};
use crate::api;
use crate::core;
use crate::ui;
use crate::ui::preferences::PreferencesMsg;

pub fn handle_msg(model: &mut AppModel, msg: AppMsg, sender: ComponentSender<AppModel>) {
  match msg {
    // --- Inputs ---
    AppMsg::UpdateDoi(v) => model.doi_input = v,
    AppMsg::UpdateSearch(v) => model.search_input = v,
    // Update the manual input string
    AppMsg::UpdateManualBib(v) => model.manual_bib_input = v,

    // --- NEW: Parse Manual Entry ---
    AppMsg::ParseManualBib => {
      let text = model.manual_bib_input.trim().to_string();
      if text.is_empty() {
        return;
      }

      // Try to parse the pasted text
      match Bibliography::parse(&text) {
        Ok(bib) => {
          let mut count = 0;
          for entry in bib.iter() {
            sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
            count += 1;
          }
          // Clear the input box on success
          model.manual_bib_input = String::new();
          model.status_msg = format!("Added {} manual entries.", count);
        }
        Err(e) => {
          // Show error alert if parsing fails
          model.status_msg = "Error parsing manual entry.".into();
          model
            .alert
            .emit(AlertMsg::Show(format!("Invalid BibTeX format:\n{}", e)));
        }
      }
    }

    // --- Network Fetching ---
    AppMsg::FetchDoi => {
      let doi = model.doi_input.trim().to_string();
      if doi.is_empty() {
        return;
      }

      model.is_loading = true;
      model.status_msg = format!("Resolving DOI: {}...", doi);

      let sender = sender.clone();
      tokio::spawn(async move {
        match api::fetch_doi(&doi).await {
          Ok(bib) => sender.input(AppMsg::FetchSuccess(bib)),
          Err(e) => sender.input(AppMsg::FetchError(e.to_string())),
        }
      });
    }

    AppMsg::FetchSearch => {
      let query = model.search_input.trim().to_string();
      if query.is_empty() {
        return;
      }

      // 1. Smart Paste (Raw BibTeX) - kept for search bar compatibility
      if query.contains("@") {
        match Bibliography::parse(&query) {
          Ok(bib) => {
            sender.input(AppMsg::FetchSuccess(bib));
            return;
          }
          Err(_) => {}
        }
      }

      model.is_loading = true;
      model.status_msg = format!("Searching Crossref: {}...", query);

      let sender = sender.clone();
      tokio::spawn(async move {
        match api::search_crossref(&query).await {
          Ok(bib) => sender.input(AppMsg::FetchSuccess(bib)),
          Err(e) => sender.input(AppMsg::FetchError(e.to_string())),
        }
      });
    }

    // --- Async Results ---
    AppMsg::FetchSuccess(new_bib) => {
      model.is_loading = false;
      model.status_msg = format!("Imported {} entries.", new_bib.len());
      for entry in new_bib {
        sender.input(AppMsg::AddBiblatexEntry(entry));
      }
      model.doi_input.clear();
      model.search_input.clear();
    }

    AppMsg::FetchError(err) => {
      model.is_loading = false;
      model.status_msg = format!("Error: {}", err);
      model.alert.emit(AlertMsg::Show(err));
    }

    // --- Data Logic ---
    AppMsg::AddBiblatexEntry(mut entry) => {
      entry.key = core::keygen::generate_key(&entry, &model.key_config);
      model.bibliography.insert(entry.clone());

      model.entries.guard().push_front(ui::row::BibEntry {
        key: entry.key.clone(),
        title: entry
          .fields
          .get("title")
          .map(|t| core::bib_to_string(t))
          .unwrap_or_else(|| "Untitled".to_string()),
        kind: entry.entry_type.to_string(),
        is_error: false,
      });
    }

    AppMsg::DeleteEntry(key) => {
      model.bibliography.remove(&key);
      let mut guard = model.entries.guard();
      let index_opt = guard.iter().position(|e| e.key == key);
      if let Some(index) = index_opt {
        guard.remove(index);
      }
      model.status_msg = format!("Deleted entry {}", key);
    }

    AppMsg::ClearAll => {
      model.bibliography = Bibliography::new();
      model.entries.guard().clear();
      model.status_msg = "Cleared library.".to_string();
    }

    // --- Config & Regen ---
    AppMsg::UpdateKeyConfig(cfg) => {
      model.key_config = cfg.clone();
      core::config::save(&cfg);
      sender.input(AppMsg::RegenerateAllKeys);
    }

    AppMsg::RegenerateAllKeys => {
      let old_bib = model.bibliography.clone();
      model.bibliography = Bibliography::new();
      model.entries.guard().clear();
      for entry in old_bib.iter() {
        sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
      }
      model.status_msg = "Regenerated all keys.".to_string();
    }

    // --- Triggers / Menus ---
    AppMsg::TriggerOpen => model.open_dialog.emit(OpenDialogMsg::Open),
    AppMsg::TriggerSave => model
      .save_dialog
      .emit(SaveDialogMsg::SaveAs("bibliography.bib".into())),
    AppMsg::TriggerSaveAs => model
      .save_dialog
      .emit(SaveDialogMsg::SaveAs("bibliography.bib".into())),
    AppMsg::ShowPreferences => model.preferences.emit(PreferencesMsg::Show),

    // --- File Responses ---
    AppMsg::OpenResponse(OpenDialogResponse::Accept(path)) => {
      let content = std::fs::read_to_string(&path).unwrap_or_default();

      // 1. FAST PATH
      if let Ok(bib) = Bibliography::parse(&content) {
        for entry in bib.iter() {
          sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
        }
        model.status_msg = format!("Loaded {} entries.", bib.len());
        return;
      }

      // 2. SLOW PATH (Error Collection)
      let mut success_count = 0;
      let mut errors = Vec::new();
      let mut batch_keys = HashSet::new();

      let chunks: Vec<&str> = content.split("\n@").collect();

      for (i, chunk) in chunks.iter().enumerate() {
        let text = if i == 0 {
          format!("{}", chunk)
        } else {
          format!("@{}", chunk)
        };
        if text.trim().is_empty() {
          continue;
        }

        match Bibliography::parse(&text) {
          Ok(bib) => {
            for entry in bib.iter() {
              let key = entry.key.clone();
              if batch_keys.contains(&key) {
                errors.push(format!("Duplicate key in file: '{}'", key));
                continue;
              }
              if model.bibliography.get(&key).is_some() {
                errors.push(format!("Key already exists in library: '{}'", key));
                continue;
              }
              batch_keys.insert(key);
              sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
              success_count += 1;
            }
          }
          Err(e) => {
            let key_hint = text
              .lines()
              .next()
              .unwrap_or("?")
              .replace("@", "")
              .replace("{", "");
            let key_name = key_hint.split(',').next().unwrap_or("?").trim();
            errors.push(format!("Parse error at '{}': {}", key_name, e));
          }
        }
      }

      if errors.is_empty() {
        model.status_msg = format!("Loaded {} entries.", success_count);
      } else {
        model.status_msg = format!(
          "Loaded {} valid. Found {} errors.",
          success_count,
          errors.len()
        );
        let report = format!(
          "Loaded {} valid entries.\n\nErrors:\n- {}",
          success_count,
          errors.join("\n- ")
        );
        model.alert.emit(AlertMsg::Show(report));
      }
    }

    AppMsg::SaveResponse(SaveDialogResponse::Accept(path)) => {
      let output = model.bibliography.to_bibtex_string();
      if std::fs::write(&path, output).is_ok() {
        model.status_msg = "Library saved.".to_string();
      } else {
        let err_msg = "Failed to write file to disk.".to_string();
        model.status_msg = err_msg.clone();
        model.alert.emit(AlertMsg::Show(err_msg));
      }
    }

    _ => {}
  }
}
