// src/app/update.rs

use biblatex::Bibliography;
use relm4::prelude::*;
use relm4_components::open_dialog::OpenDialogMsg;
use relm4_components::save_dialog::SaveDialogMsg;

use super::alert::AlertMsg; // Import AlertMsg
use super::model::{AppModel, AppMsg};
use crate::core;
use crate::logic::{deduplicator, fetch, library}; // Import deduplicator
use crate::menu::file_io;
use crate::ui::duplicate_dialog::DuplicateDialogMsg; // Import DialogMsg
use crate::ui::preferences::PreferencesMsg;
use crate::ui::sidebar::SidebarMsg;

pub fn handle_msg(model: &mut AppModel, msg: AppMsg, sender: ComponentSender<AppModel>) {
    match msg {
        // --- Sidebar Actions ---
        AppMsg::FetchDoi(doi) => fetch::handle_fetch_doi(model, sender, doi),
        AppMsg::FetchSearch(query) => fetch::handle_fetch_search(model, sender, query),
        AppMsg::ParseManualBib(text) => file_io::parse_manual(model, sender, text),

        AppMsg::ClearAll => {
            model.bibliography = Bibliography::new();
            model.entries.guard().clear();
            model
                .sidebar
                .emit(SidebarMsg::SetStatus("Library cleared.".into()));
        }

        // --- Standard Logic ---
        AppMsg::TriggerOpen => model.open_dialog.emit(OpenDialogMsg::Open),
        AppMsg::TriggerSave => file_io::trigger_save(model),
        AppMsg::TriggerSaveAs => model
            .save_dialog
            .emit(SaveDialogMsg::SaveAs("library.bib".into())),

        AppMsg::OpenResponse(resp) => file_io::handle_open_response(model, resp, sender),
        AppMsg::SaveResponse(resp) => file_io::handle_save_response(model, resp),

        AppMsg::FetchSuccess(bib) => fetch::handle_success(model, bib, sender),
        AppMsg::FetchError(err) => fetch::handle_error(model, err),
        AppMsg::SearchResultsLoaded(items) => fetch::handle_search_results(model, items),

        AppMsg::FetchSelectedDoi(doi) => {
            sender.input(AppMsg::FetchDoi(doi));
        }

        // --- Library Management ---
        AppMsg::AddBiblatexEntry(entry) => library::add_entry(model, entry),
        AppMsg::HandleRowOutput(output) => library::handle_row_output(model, output),

        // NEW: Updated Duplicate Logic
        AppMsg::ScanDuplicates => {
            let duplicates = deduplicator::find_duplicates(&model.bibliography);

            if duplicates.is_empty() {
                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus("Library clean.".into()));
                model.alert.emit(AlertMsg::ShowInfo(
                    "Library Clean.\nNo duplicates found.".into(),
                ));
            } else {
                model.sidebar.emit(SidebarMsg::SetStatus(format!(
                    "Reviewing {} duplicate groups...",
                    duplicates.len()
                )));
                // Open the review dialog
                model
                    .duplicate_dialog
                    .emit(DuplicateDialogMsg::LoadGroups(duplicates));
            }
        }

        // NEW: Handle Deletion from the Duplicate Dialog
        AppMsg::DeleteEntry(key) => {
            // 1. Remove from Data
            model.bibliography.remove(&key);

            // 2. Remove from UI (Find index first to satisfy borrow checker)
            let index_opt = model.entries.iter().position(|e| e.key == key);
            if let Some(index) = index_opt {
                model.entries.guard().remove(index);
            }

            model
                .sidebar
                .emit(SidebarMsg::SetStatus(format!("Deleted entry: {}", key)));
        }

        AppMsg::RegenerateAllKeys => library::regenerate_keys(model, sender),
        AppMsg::AbbreviateAllJournals => library::abbreviate_all_entries(model),
        AppMsg::UnabbreviateAllJournals => library::unabbreviate_all_entries(model),

        AppMsg::FinishEditEntry(key, content) => library::finish_edit(model, key, content, sender),

        // --- Preferences ---
        AppMsg::ShowPreferences => model.preferences.emit(PreferencesMsg::Show),
        AppMsg::UpdateKeyConfig(config) => {
            model.key_config = config;
            core::config::save(&model.key_config);
            model
                .sidebar
                .emit(SidebarMsg::SetStatus("Preferences saved.".into()));
        }
    }
}
