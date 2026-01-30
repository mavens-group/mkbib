// src/menu/file_io.rs

use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::ui::row::BibEntry;
use crate::ui::sidebar::SidebarMsg;
use biblatex::Bibliography;
use relm4::{ComponentController, ComponentSender};
use relm4_components::open_dialog::OpenDialogResponse;
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};

pub fn handle_open_response(
    model: &mut AppModel,
    resp: OpenDialogResponse<relm4_components::open_dialog::SingleSelection>,
    _sender: ComponentSender<AppModel>,
) {
    if let OpenDialogResponse::Accept(path) = resp {
        if let Ok(content) = std::fs::read_to_string(&path) {
            model.sidebar.emit(SidebarMsg::SetStatus(format!(
                "Loading {}...",
                path.display()
            )));

            // NOTE: We rely on the "rewrite" behavior (keeping original_file_content as None)
            // to avoid the duplication bugs you encountered with the merger.

            match Bibliography::parse(&content) {
                Ok(bib) => {
                    let count = bib.len();
                    model.bibliography = bib;
                    model.current_file_path = Some(path.clone());

                    model.undo_stack.clear();
                    model.redo_stack.clear();
                    model.is_dirty = false;

                    model.entries.guard().clear();
                    for entry in model.bibliography.iter() {
                        model.entries.guard().push_back(BibEntry::from_entry(entry));
                    }

                    model
                        .sidebar
                        .emit(SidebarMsg::SetStatus(format!("Loaded {} entries.", count)));
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

pub fn handle_save_response(model: &mut AppModel, resp: SaveDialogResponse) {
    if let SaveDialogResponse::Accept(path) = resp {
        let _ = crate::core::create_backup(&path);

        let output = if let Some(original) = &model.original_file_content {
            crate::logic::merger::merge_bibliography_into_source(
                original,
                &model.bibliography,
                &model.key_config,
            )
        } else {
            crate::logic::merger::merge_bibliography_into_source(
                "",
                &model.bibliography,
                &model.key_config,
            )
        };

        // ✅ FIX: Trim leading/trailing whitespace to remove the empty lines at the top.
        // We add one newline at the end for standard file formatting.
        let final_output = format!("{}\n", output.trim());

        if std::fs::write(&path, &final_output).is_ok() {
            model.current_file_path = Some(path);
            model.original_file_content = Some(final_output);
            model.is_dirty = false;
            model
                .sidebar
                .emit(SidebarMsg::SetStatus("Library saved.".to_string()));
        } else {
            model
                .alert
                .emit(AlertMsg::Show("Failed to write file.".into()));
        }
    }
}

pub fn trigger_save(model: &mut AppModel) {
    if let Some(path) = &model.current_file_path {
        let _ = crate::core::create_backup(path);

        let output = if let Some(original) = &model.original_file_content {
            crate::logic::merger::merge_bibliography_into_source(
                original,
                &model.bibliography,
                &model.key_config,
            )
        } else {
            crate::logic::merger::merge_bibliography_into_source(
                "",
                &model.bibliography,
                &model.key_config,
            )
        };

        // ✅ FIX: Trim whitespace here too.
        let final_output = format!("{}\n", output.trim());

        match std::fs::write(path, &final_output) {
            Ok(_) => {
                model.original_file_content = Some(final_output);
                model.is_dirty = false;
                model.sidebar.emit(SidebarMsg::SetStatus(format!(
                    "Saved to {}",
                    path.display()
                )));
            }
            Err(e) => model
                .alert
                .emit(AlertMsg::Show(format!("Failed to save:\n{}", e))),
        }
    } else {
        model
            .save_dialog
            .emit(SaveDialogMsg::SaveAs("library.bib".into()));
    }
}

pub fn parse_manual(model: &mut AppModel, sender: ComponentSender<AppModel>, text: String) {
    if text.trim().is_empty() {
        return;
    }

    match Bibliography::parse(&text) {
        Ok(bib) => {
            model.push_snapshot();

            let mut count = 0;
            for entry in bib.iter() {
                sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
                count += 1;
            }
            model.sidebar.emit(SidebarMsg::SetStatus(format!(
                "Added {} manual entries.",
                count
            )));
        }
        Err(e) => {
            model
                .sidebar
                .emit(SidebarMsg::SetStatus("Parse failed.".to_string()));
            model
                .alert
                .emit(AlertMsg::Show(format!("BibTeX Parse Error:\n{}", e)));
        }
    }
}
