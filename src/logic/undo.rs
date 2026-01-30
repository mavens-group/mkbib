// src/logic/undo.rs
use crate::app::AppModel;
use crate::ui::row::BibEntry;
use crate::ui::sidebar::SidebarMsg;
use relm4::ComponentController;

pub fn perform_undo(model: &mut AppModel) {
    if let Some(previous_state) = model.undo_stack.pop_back() {
        // 1. Move CURRENT state to Redo stack
        model.redo_stack.push_back(model.bibliography.clone());

        // 2. Load PREVIOUS state
        model.bibliography = previous_state;

        // 3. Refresh UI
        refresh_ui(model);

        model
            .sidebar
            .emit(SidebarMsg::SetStatus("Undo successful.".into()));
    } else {
        model
            .sidebar
            .emit(SidebarMsg::SetStatus("Nothing to undo.".into()));
    }
}

pub fn perform_redo(model: &mut AppModel) {
    if let Some(next_state) = model.redo_stack.pop_back() {
        // 1. Move CURRENT state to Undo stack
        model.undo_stack.push_back(model.bibliography.clone());

        // 2. Load NEXT state
        model.bibliography = next_state;

        // 3. Refresh UI
        refresh_ui(model);

        model
            .sidebar
            .emit(SidebarMsg::SetStatus("Redo successful.".into()));
    } else {
        model
            .sidebar
            .emit(SidebarMsg::SetStatus("Nothing to redo.".into()));
    }
}

// Helper to rebuild the list
fn refresh_ui(model: &mut AppModel) {
    model.entries.guard().clear();
    for entry in model.bibliography.iter() {
        model.entries.guard().push_back(BibEntry::from_entry(entry));
    }
    model.is_dirty = true;
}
