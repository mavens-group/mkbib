// src/app/update.rs

use biblatex::Bibliography;
use gtk4::gio::prelude::ApplicationExt;
use relm4::prelude::*;
use relm4_components::open_dialog::OpenDialogMsg;
use relm4_components::save_dialog::SaveDialogMsg;

use super::alert::AlertMsg; // Import AlertMsg
use super::model::{AppModel, AppMsg, PendingAction};
use crate::core;
use crate::logic::{deduplicator, fetch, library}; // Import deduplicator
use crate::menu::file_io;
use crate::ui::duplicate_dialog::DuplicateDialogMsg; // Import DialogMsg
use crate::ui::preferences::PreferencesMsg;
use crate::ui::sidebar::SidebarMsg;
use crate::ui::unsaved_dialog::{UnsavedDecision, UnsavedDialogMsg};

fn continue_pending(model: &mut AppModel) {
    match model.pending_action.take() {
        Some(PendingAction::Open) => model.open_dialog.emit(OpenDialogMsg::Open),
        Some(PendingAction::Quit) => relm4::main_application().quit(),
        None => {}
    }
}

fn request_destructive_action(model: &mut AppModel, action: PendingAction) {
    if model.is_dirty {
        model.pending_action = Some(action);
        model.unsaved_dialog.emit(UnsavedDialogMsg::Show);
    } else {
        model.pending_action = Some(action);
        continue_pending(model);
    }
}

pub fn handle_msg(model: &mut AppModel, msg: AppMsg, sender: ComponentSender<AppModel>) {
    match msg {
        // --- Sidebar Actions ---
        AppMsg::FetchDoi(doi) => fetch::handle_fetch_doi(model, sender, doi),
        AppMsg::FetchSearch(query) => fetch::handle_fetch_search(model, sender, query),
        AppMsg::ParseManualBib(text) => file_io::parse_manual(model, sender, text),

        AppMsg::ClearAll => {
            if model.bibliography.is_empty() {
                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus("Library is already empty.".into()));
                return;
            }
            model.push_snapshot();
            model.bibliography = Bibliography::new();
            model.entries.guard().clear();
            model.mark_changed();
            model
                .sidebar
                .emit(SidebarMsg::SetStatus("Library cleared.".into()));
        }

        // --- Standard Logic ---
        AppMsg::TriggerOpen => request_destructive_action(model, PendingAction::Open),
        AppMsg::TriggerSave => {
            file_io::trigger_save(model);
        }
        AppMsg::TriggerSaveAs => model
            .save_dialog
            .emit(SaveDialogMsg::SaveAs("library.bib".into())),

        AppMsg::TriggerQuit => request_destructive_action(model, PendingAction::Quit),
        AppMsg::ForceQuit => {
            let app = relm4::main_application();
            app.quit();
        }

        AppMsg::OpenResponse(resp) => file_io::handle_open_response(model, resp, sender),
        AppMsg::SaveResponse(resp) => match file_io::handle_save_response(model, resp) {
            file_io::SaveAttempt::Saved => continue_pending(model),
            file_io::SaveAttempt::AwaitingPath => {}
            file_io::SaveAttempt::Failed => model.pending_action = None,
        },
        AppMsg::UnsavedDecision(decision) => match decision {
            UnsavedDecision::Save => match file_io::trigger_save(model) {
                file_io::SaveAttempt::Saved => continue_pending(model),
                file_io::SaveAttempt::AwaitingPath => {}
                file_io::SaveAttempt::Failed => model.pending_action = None,
            },
            UnsavedDecision::Discard => continue_pending(model),
            UnsavedDecision::Cancel => model.pending_action = None,
        },

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
            if model.bibliography.get(&key).is_none() {
                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus(format!("Entry not found: {key}")));
                return;
            }
            model.push_snapshot();
            // 1. Remove from Data
            model.bibliography.remove(&key);

            // 2. Remove from UI (Find index first to satisfy borrow checker)
            let index_opt = model.entries.iter().position(|e| e.key == key);
            if let Some(index) = index_opt {
                model.entries.guard().remove(index);
            }
            model.mark_changed();

            model
                .sidebar
                .emit(SidebarMsg::SetStatus(format!("Deleted entry: {}", key)));
        }

        AppMsg::RegenerateAllKeys => library::regenerate_keys(model, sender),
        AppMsg::ReformatAll => library::reformat_all_entries(model),
        AppMsg::AbbreviateAllJournals => library::abbreviate_all_entries(model),
        AppMsg::UnabbreviateAllJournals => library::unabbreviate_all_entries(model),

        AppMsg::FinishEditEntry(key, content) => library::finish_edit(model, key, content, sender),

        // --- Preferences ---
        AppMsg::ShowPreferences => model.preferences.emit(PreferencesMsg::Show),
        AppMsg::UpdateKeyConfig(config) => match core::config::save(&config) {
            Ok(()) => {
                model.key_config = config;
                model
                    .sidebar
                    .emit(SidebarMsg::SetStatus("Preferences saved.".into()));
            }
            Err(error) => model.alert.emit(AlertMsg::Show(format!(
                "Failed to save preferences:\n{error}"
            ))),
        },
        AppMsg::Undo => {
            crate::logic::undo::perform_undo(model);
        }
        AppMsg::Redo => {
            crate::logic::undo::perform_redo(model);
        }
    }
}
