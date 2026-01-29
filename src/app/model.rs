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

// --- State ---
pub struct AppModel {
    pub bibliography: Bibliography,
    pub entries: FactoryVecDeque<ui::row::BibEntry>,
    pub current_file_path: Option<PathBuf>,

    // Child Components (Sidebar now handles inputs & status)
    pub sidebar: Controller<SidebarModel>,
    pub open_dialog: Controller<OpenDialog>,
    pub save_dialog: Controller<SaveDialog>,
    pub alert: Controller<AlertModel>,
    pub preferences: Controller<PreferencesModel>,
    pub details_dialog: Controller<DetailsDialogModel>,
    pub search_dialog: Controller<SearchDialogModel>,
    pub duplicate_dialog: Controller<DuplicateDialogModel>,

    pub key_config: KeyGenConfig,
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
    ShowPreferences,
    AbbreviateAllJournals,
    UnabbreviateAllJournals,

    FetchSuccess(Bibliography),
    FetchError(String),
    SearchResultsLoaded(Vec<crate::api::SearchResultItem>),
    FetchSelectedDoi(String),

    HandleRowOutput(BibEntryOutput),
    FinishEditEntry(String, String),
    RegenerateAllKeys,
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
