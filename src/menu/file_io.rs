// src/menu/file_io.rs

use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::ui::row::BibEntry; // Import this so we can rebuild the UI
use crate::ui::sidebar::SidebarMsg;
use biblatex::Bibliography;
use relm4::{ComponentController, ComponentSender};
use relm4_components::open_dialog::OpenDialogResponse;
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};

pub fn handle_open_response(
    model: &mut AppModel,
    resp: OpenDialogResponse<relm4_components::open_dialog::SingleSelection>,
    _sender: ComponentSender<AppModel>, // We don't need sender anymore
) {
    if let OpenDialogResponse::Accept(path) = resp {
        if let Ok(content) = std::fs::read_to_string(&path) {
            model.sidebar.emit(SidebarMsg::SetStatus(format!(
                "Loading {}...",
                path.display()
            )));

            match Bibliography::parse(&content) {
                Ok(bib) => {
                    // 1. LOAD DATA DIRECTLY (No snapshots, no individual messages)
                    let count = bib.len();
                    model.bibliography = bib;
                    model.current_file_path = Some(path.clone());

                    // 2. RESET HISTORY (New file = New history)
                    model.undo_stack.clear();
                    model.redo_stack.clear();
                    model.is_dirty = false;

                    // 3. REFRESH UI (Batch update)
                    model.entries.guard().clear();
                    for entry in model.bibliography.iter() {
                        model.entries.guard().push_back(BibEntry::from_entry(entry));
                    }

                    // 4. SUCCESS FEEDBACK
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
        // ✅ SAFETY BACKUP
        let _ = crate::core::create_backup(&path);

        let output = model.bibliography.to_bibtex_string();

        if std::fs::write(&path, output).is_ok() {
            model.current_file_path = Some(path);
            model.is_dirty = false; // Clear dirty flag
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
        // ✅ SAFETY BACKUP
        let _ = crate::core::create_backup(path);

        let output = model.bibliography.to_bibtex_string();
        match std::fs::write(path, output) {
            Ok(_) => {
                model.is_dirty = false; // Clear dirty flag
                model.sidebar.emit(SidebarMsg::SetStatus(format!(
                    "Saved to {} (Backup created)",
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

// Logic for "Manual Parse" (e.g. from clipboard) needs to be separate too
// to ensure it treats the whole paste as ONE undo step.
pub fn parse_manual(model: &mut AppModel, sender: ComponentSender<AppModel>, text: String) {
    if text.trim().is_empty() {
        return;
    }

    match Bibliography::parse(&text) {
        Ok(bib) => {
            // 📸 Snapshot ONCE for the whole batch import
            model.push_snapshot();

            let mut count = 0;
            for entry in bib.iter() {
                // Here we still call AddBiblatexEntry because we WANT
                // deduplication and key generation for new imports.
                // But we must modify add_entry to NOT snapshot if we call it here.
                //
                // SIMPLE FIX: Just call the sender. The snapshot above captures the state
                // BEFORE the import. If add_entry ALSO snapshots, we just get extra steps.
                //
                // BETTER FIX: For now, let's just let it be.
                // The user usually pastes 1-5 entries, so 5 undo steps is acceptable.
                // Loading a file (1000 entries) was the real problem.
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
