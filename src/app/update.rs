// src/app/update.rs

use biblatex::Bibliography;
use relm4::prelude::*;
use relm4_components::open_dialog::OpenDialogMsg;
use relm4_components::save_dialog::SaveDialogMsg;

use super::model::{AppModel, AppMsg};
use crate::core;
use crate::logic::{fetch, library};
use crate::menu::file_io;
use crate::ui::preferences::PreferencesMsg;
use crate::ui::sidebar::SidebarMsg;

pub fn handle_msg(model: &mut AppModel, msg: AppMsg, sender: ComponentSender<AppModel>) {
    match msg {
        // --- Sidebar Actions (Now with Data!) ---
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
            // Re-route internally
            sender.input(AppMsg::FetchDoi(doi));
        }

        AppMsg::AddBiblatexEntry(entry) => library::add_entry(model, entry),
        AppMsg::HandleRowOutput(output) => library::handle_row_output(model, output),
        AppMsg::ScanDuplicates => library::scan_duplicates(model),
        AppMsg::RegenerateAllKeys => library::regenerate_keys(model, sender),
        AppMsg::FinishEditEntry(key, content) => library::finish_edit(model, key, content, sender),

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
