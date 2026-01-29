// src/menu/file_io.rs
//
use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::ui::sidebar::SidebarMsg; // Needed
use biblatex::Bibliography;
use relm4::{ComponentController, ComponentSender};
use relm4_components::open_dialog::OpenDialogResponse;
use relm4_components::save_dialog::{SaveDialogMsg, SaveDialogResponse};

pub fn handle_open_response(
    model: &mut AppModel,
    resp: OpenDialogResponse<relm4_components::open_dialog::SingleSelection>,
    sender: ComponentSender<AppModel>,
) {
    if let OpenDialogResponse::Accept(path) = resp {
        if let Ok(content) = std::fs::read_to_string(&path) {
            model.sidebar.emit(SidebarMsg::SetStatus(format!(
                "Loading {}...",
                path.display()
            )));

            match Bibliography::parse(&content) {
                Ok(bib) => {
                    model.current_file_path = Some(path.clone());
                    let mut count = 0;
                    for entry in bib.iter() {
                        sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
                        count += 1;
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
        let output = model.bibliography.to_bibtex_string(); // Result<String, Error> in newer, String in older.
                                                            // Assuming your version returns String based on previous logs:
        if std::fs::write(&path, output).is_ok() {
            model.current_file_path = Some(path);
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
        let output = model.bibliography.to_bibtex_string();
        match std::fs::write(path, output) {
            Ok(_) => model.sidebar.emit(SidebarMsg::SetStatus(format!(
                "Saved to {}",
                path.display()
            ))),
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

// NEW SIGNATURE: Accepts String
pub fn parse_manual(model: &mut AppModel, sender: ComponentSender<AppModel>, text: String) {
    if text.trim().is_empty() {
        return;
    }

    match Bibliography::parse(&text) {
        Ok(bib) => {
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
