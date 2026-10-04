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
use std::{fs, io::Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAttempt {
    Saved,
    AwaitingPath,
    Failed,
}

pub fn handle_open_response(
    model: &mut AppModel,
    resp: OpenDialogResponse<relm4_components::open_dialog::SingleSelection>,
    _sender: ComponentSender<AppModel>,
) {
    if let OpenDialogResponse::Accept(path) = resp {
        if let Ok(content) = fs::read_to_string(&path) {
            model.sidebar.emit(SidebarMsg::SetStatus(format!(
                "Loading {}...",
                path.display()
            )));

            let parse_input = crate::core::normalize_month_macros(&content);
            match Bibliography::parse(&parse_input) {
                Ok(bib) => {
                    let count = bib.len();
                    model.bibliography = bib;
                    model.current_file_path = Some(path.clone());
                    model.original_file_content = Some(content);

                    model.undo_stack.clear();
                    model.redo_stack.clear();
                    model.current_revision = 0;
                    model.saved_revision = 0;
                    model.next_revision = 1;
                    model.sync_dirty();

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
        } else if let Err(error) = fs::read_to_string(&path) {
            model.alert.emit(AlertMsg::Show(format!(
                "Failed to read {}:\n{error}",
                path.display()
            )));
        }
    }
}

pub fn handle_save_response(model: &mut AppModel, resp: SaveDialogResponse) -> SaveAttempt {
    if let SaveDialogResponse::Accept(path) = resp {
        return save_with_feedback(model, path);
    }
    SaveAttempt::Failed
}

pub fn trigger_save(model: &mut AppModel) -> SaveAttempt {
    if let Some(path) = &model.current_file_path {
        save_with_feedback(model, path.clone())
    } else {
        model
            .save_dialog
            .emit(SaveDialogMsg::SaveAs("library.bib".into()));
        SaveAttempt::AwaitingPath
    }
}

fn save_with_feedback(model: &mut AppModel, path: PathBuf) -> SaveAttempt {
    match perform_safe_save(model, path) {
        Ok(()) => SaveAttempt::Saved,
        Err(e) => {
            model
                .alert
                .emit(AlertMsg::Show(format!("Failed to save file:\n{e}")));
            SaveAttempt::Failed
        }
    }
}

fn perform_safe_save(model: &mut AppModel, path: PathBuf) -> anyhow::Result<()> {
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

    let final_output = format!("{}\n", output.trim_end());

    if path.exists() {
        crate::core::create_backup(&path)
            .map_err(|e| anyhow::anyhow!("could not create backup: {e}"))?;
    }

    atomic_replace(&path, final_output.as_bytes())?;

    model.current_file_path = Some(path.clone());
    model.original_file_content = Some(final_output);
    model.mark_saved();
    model.sidebar.emit(SidebarMsg::SetStatus(format!(
        "Saved to {}",
        path.display()
    )));
    Ok(())
}

fn atomic_replace(path: &std::path::Path, contents: &[u8]) -> anyhow::Result<()> {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("the destination has no file name"))?;
    let mut temp_path = None;
    let mut temp_file = None;
    for nonce in 0..100u32 {
        let candidate = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            nonce
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                if let Ok(metadata) = fs::metadata(path) {
                    file.set_permissions(metadata.permissions()).map_err(|e| {
                        anyhow::anyhow!("could not preserve destination permissions: {e}")
                    })?;
                }
                temp_path = Some(candidate);
                temp_file = Some(file);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(anyhow::anyhow!("could not create temporary file: {e}")),
        }
    }

    let temp_path =
        temp_path.ok_or_else(|| anyhow::anyhow!("could not allocate a temporary file"))?;
    let write_result = (|| -> std::io::Result<()> {
        let mut file = temp_file.expect("temporary file exists when its path exists");
        file.write_all(contents)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(anyhow::anyhow!("atomic write failed: {e}"));
    }

    Ok(())
}

pub fn parse_manual(model: &mut AppModel, sender: ComponentSender<AppModel>, text: String) {
    if text.trim().is_empty() {
        return;
    }

    let text = crate::core::normalize_month_macros(&text);
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

#[cfg(test)]
mod tests {
    use super::atomic_replace;

    #[test]
    fn atomic_replace_creates_and_replaces_file_without_leaving_temp_files() {
        let directory =
            std::env::temp_dir().join(format!("mkbib-atomic-write-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create test directory");
        let destination = directory.join("library.bib");

        atomic_replace(&destination, b"first").expect("create destination");
        atomic_replace(&destination, b"second").expect("replace destination");

        assert_eq!(std::fs::read(&destination).unwrap(), b"second");
        let leftovers = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);

        std::fs::remove_dir_all(directory).expect("clean test directory");
    }
}
