// src/app/model.rs

use biblatex::Bibliography;
use relm4::factory::FactoryVecDeque;
use relm4::Controller;
use relm4_components::open_dialog::OpenDialog;
use relm4_components::save_dialog::SaveDialog;
use std::path::PathBuf;

use super::alert::AlertModel;
use crate::core::keygen::KeyGenConfig;
use crate::ui;
use crate::ui::details_dialog::DetailsDialogModel;
use crate::ui::duplicate_dialog::DuplicateDialogModel;
use crate::ui::preferences::PreferencesModel;
use crate::ui::row::BibEntryOutput;
use crate::ui::search_dialog::SearchDialogModel;
use crate::ui::sidebar::SidebarModel;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    Open,
    Quit,
}

#[derive(Clone)]
pub struct DocumentSnapshot {
    pub bibliography: Bibliography,
    pub revision: u64,
}

// --- State ---
pub struct AppModel {
    pub bibliography: Bibliography,
    pub entries: FactoryVecDeque<ui::row::BibEntry>,
    pub current_file_path: Option<PathBuf>,
    pub original_file_content: Option<String>,

    // Child Components (Sidebar now handles inputs & status)
    pub sidebar: Controller<SidebarModel>,
    pub open_dialog: Controller<OpenDialog>,
    pub save_dialog: Controller<SaveDialog>,
    pub alert: Controller<AlertModel>,
    pub preferences: Controller<PreferencesModel>,
    pub details_dialog: Controller<DetailsDialogModel>,
    pub search_dialog: Controller<SearchDialogModel>,
    pub duplicate_dialog: Controller<DuplicateDialogModel>,
    pub unsaved_dialog: Controller<crate::ui::unsaved_dialog::UnsavedDialogModel>,

    pub key_config: KeyGenConfig,
    pub is_dirty: bool,
    pub undo_stack: VecDeque<DocumentSnapshot>,
    pub redo_stack: VecDeque<DocumentSnapshot>,
    pub current_revision: u64,
    pub saved_revision: u64,
    pub next_revision: u64,
    pub pending_action: Option<PendingAction>,
}

// --- Messages ---
#[derive(Debug)]
pub enum AppMsg {
    // These now carry data directly from the Sidebar!
    FetchDoi(String),
    FetchSearch(String),
    ParseManualBib(String),
    ClearAll,

    TriggerOpen,
    TriggerSave,
    TriggerSaveAs,
    TriggerQuit,
    #[allow(dead_code)] // quit-without-saving path; handler wired in update.rs, not yet emitted
    ForceQuit,
    UnsavedDecision(crate::ui::unsaved_dialog::UnsavedDecision),
    ShowPreferences,
    AbbreviateAllJournals,
    UnabbreviateAllJournals,
    Undo,
    Redo,

    FetchSuccess(Bibliography),
    FetchError(String),
    SearchResultsLoaded(Vec<crate::api::SearchResultItem>),
    FetchSelectedDoi(String),

    HandleRowOutput(BibEntryOutput),
    FinishEditEntry(String, String),
    RegenerateAllKeys,
    ReformatAll,
    ScanDuplicates,
    UpdateKeyConfig(KeyGenConfig),
    AddBiblatexEntry(biblatex::Entry),
    DeleteEntry(String),

    OpenResponse(
        relm4_components::open_dialog::OpenDialogResponse<
            relm4_components::open_dialog::SingleSelection,
        >,
    ),
    SaveResponse(relm4_components::save_dialog::SaveDialogResponse),
}

impl AppModel {
    pub fn push_snapshot(&mut self) {
        // 1. Clear Redo stack (Standard logic: new action kills the future)
        self.redo_stack.clear();

        // 2. Limit Undo stack to 50 steps
        if self.undo_stack.len() >= 50 {
            self.undo_stack.pop_front();
        }

        // 3. Save current state
        self.undo_stack.push_back(DocumentSnapshot {
            bibliography: self.bibliography.clone(),
            revision: self.current_revision,
        });
    }

    pub fn mark_changed(&mut self) {
        self.current_revision = self.next_revision;
        self.next_revision = self.next_revision.saturating_add(1);
        self.sync_dirty();
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.current_revision;
        self.sync_dirty();
    }

    pub fn sync_dirty(&mut self) {
        self.is_dirty = self.current_revision != self.saved_revision;
    }
}
