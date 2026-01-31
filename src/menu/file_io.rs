// src/menu/file_io.rs

use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::ui::row::BibEntry;
use crate::ui::sidebar::SidebarMsg;
use biblatex::Bibliography;
use relm4::{ComponentController, ComponentSender};
use relm4_components::open_dialog::OpenDialogResponse;
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};
use std::path::PathBuf;

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

            // Capture original content for the Merger (Step 1 - Coming next)
            // For now, we are still using the Rewrite strategy, but we prep the field.
            model.original_file_content = Some(content.clone());

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
        perform_safe_save(model, path);
    }
}

pub fn trigger_save(model: &mut AppModel) {
    if let Some(path) = &model.current_file_path {
        perform_safe_save(model, path.clone());
    } else {
        model
            .save_dialog
            .emit(SaveDialogMsg::SaveAs("library.bib".into()));
    }
}

/// ✅ THE DIAMOND STANDARD SAVE FUNCTION
fn perform_safe_save(model: &mut AppModel, path: PathBuf) {
    // 1. Generate Content
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

    // ✅ FIX: Use trim_end() to remove ALL trailing newlines first.
    // format!("{}\n", ...) ensures exactly ONE newline exists at the end.
    let final_output = format!("{}\n", output.trim_end());

    // 2. Create Backup
    if let Err(e) = crate::core::create_backup(&path) {
        println!("Backup warning: {}", e);
        // We log to console/stdout instead of blocking the user with an alert
        // because backups failing shouldn't stop the user from saving their work.
    }

    // 3. ATOMIC WRITE (Write to .tmp -> Rename to .bib)
    let tmp_path = path.with_extension("bib.tmp");

    match std::fs::write(&tmp_path, &final_output) {
        Ok(_) => {
            match std::fs::rename(&tmp_path, &path) {
                Ok(_) => {
                    model.current_file_path = Some(path.clone());
                    // Update internal state to match what is now on disk
                    model.original_file_content = Some(final_output);
                    model.is_dirty = false;
                    model.sidebar.emit(SidebarMsg::SetStatus(format!(
                        "Saved to {}",
                        path.display()
                    )));
                }
                Err(e) => model
                    .alert
                    .emit(AlertMsg::Show(format!("Rename failed: {}", e))),
            }
        }
        Err(e) => model
            .alert
            .emit(AlertMsg::Show(format!("Write failed: {}", e))),
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
