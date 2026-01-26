use biblatex::{Bibliography, Entry};
use relm4::factory::FactoryVecDeque;
use relm4::Controller;
use relm4_components::open_dialog::{OpenDialog, OpenDialogResponse, SingleSelection};
use relm4_components::save_dialog::{SaveDialog, SaveDialogResponse};

use super::alert::AlertModel;
use crate::core::keygen::KeyGenConfig;
use crate::ui;
use crate::ui::preferences::PreferencesModel;

// --- State ---

pub struct AppModel {
    pub bibliography: Bibliography,
    pub entries: FactoryVecDeque<ui::row::BibEntry>,

    // UI State
    pub doi_input: String,
    pub search_input: String,
    pub manual_bib_input: String, // <--- NEW FIELD

    pub is_loading: bool,
    pub status_msg: String,

    // Child Components
    pub open_dialog: Controller<OpenDialog>,
    pub save_dialog: Controller<SaveDialog>,
    pub alert: Controller<AlertModel>,
    pub preferences: Controller<PreferencesModel>,

    // Config
    pub key_config: KeyGenConfig,
}

// --- Messages ---

#[derive(Debug)]
pub enum AppMsg {
    // UI Inputs
    UpdateDoi(String),
    UpdateSearch(String),
    UpdateManualBib(String), // <--- NEW MSG

    // Triggers
    TriggerOpen,
    TriggerSave,
    TriggerSaveAs,
    ShowPreferences,
    ParseManualBib, // <--- NEW MSG

    // Async Triggers
    FetchDoi,
    FetchSearch,

    // Async Results
    FetchSuccess(Bibliography),
    FetchError(String),

    // Core Logic
    DeleteEntry(String),
    ClearAll,
    RegenerateAllKeys,
    UpdateKeyConfig(KeyGenConfig),
    AddBiblatexEntry(Entry),

    // Component Responses
    OpenResponse(OpenDialogResponse<SingleSelection>),
    SaveResponse(SaveDialogResponse),
}
