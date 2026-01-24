use biblatex::{Bibliography, Entry};
use gtk4::gio;
use gtk4::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use relm4_components::open_dialog::{
  OpenDialog, OpenDialogMsg, OpenDialogResponse, OpenDialogSettings, SingleSelection,
};
use relm4_components::save_dialog::{
  SaveDialog, SaveDialogMsg, SaveDialogResponse, SaveDialogSettings,
};

// Modules
use crate::api;
use crate::core;
use crate::core::keygen::KeyGenConfig;
use crate::menu;
use crate::ui;
use crate::ui::preferences::{PreferencesModel, PreferencesMsg, PreferencesOutput};

// --- Model ---

pub struct AppModel {
  pub bibliography: Bibliography,
  pub entries: FactoryVecDeque<ui::row::BibEntry>,

  // State
  pub doi_input: String,
  pub search_input: String,
  pub is_loading: bool,
  pub status_msg: String,

  // Components
  pub open_dialog: Controller<OpenDialog>,
  pub save_dialog: Controller<SaveDialog>,
  pub preferences: Controller<PreferencesModel>,

  // Config
  pub key_config: KeyGenConfig,
}

#[derive(Debug)]
pub enum AppMsg {
  // Inputs
  UpdateDoi(String),
  UpdateSearch(String),

  // Actions / Triggers
  TriggerOpen,
  TriggerSave,
  TriggerSaveAs,
  ShowPreferences, // Used by actions_edit

  // Async Triggers
  FetchDoi,
  FetchSearch,

  // Async Results
  FetchSuccess(Bibliography),
  FetchError(String),

  // App Logic
  DeleteEntry(String),
  ClearAll,
  RegenerateAllKeys, // Used by actions_edit
  UpdateKeyConfig(KeyGenConfig),

  // File Responses
  OpenResponse(OpenDialogResponse<SingleSelection>),
  SaveResponse(SaveDialogResponse),

  // Core
  AddBiblatexEntry(Entry),
}

// --- Component ---

#[relm4::component(pub)]
impl Component for AppModel {
  type Init = ();
  type Input = AppMsg;
  type Output = ();
  type CommandOutput = ();

  view! {
      gtk::ApplicationWindow {
          set_title: Some("MkBib"),
          set_default_width: 1100,
          set_default_height: 750,

          gtk::Box {
              set_orientation: gtk::Orientation::Vertical,
              set_spacing: 0,

              // --- 1. MENU BAR ---
              #[local_ref]
              menu_bar -> gtk::PopoverMenuBar {},

              // --- 2. MAIN CONTENT (Horizontal Split) ---
              gtk::Box {
                  set_orientation: gtk::Orientation::Horizontal,
                  set_vexpand: true,

                  // --- SIDEBAR ---
                  gtk::Box {
                      set_width_request: 320,
                      set_orientation: gtk::Orientation::Vertical,
                      set_margin_all: 12,
                      set_spacing: 12,

                      gtk::Label {
                          set_label: "Library Sources",
                          add_css_class: "title-3",
                          set_halign: gtk::Align::Start
                      },

                      // DOI Import Section
                      gtk::Frame {
                          set_label: Some("Import via DOI"),
                          gtk::Box {
                              set_orientation: gtk::Orientation::Vertical,
                              set_spacing: 8,
                              set_margin_all: 8,

                              gtk::Entry {
                                  set_placeholder_text: Some("10.1038/..."),
                                  set_text: &model.doi_input,
                                  connect_changed[sender] => move |e| sender.input(AppMsg::UpdateDoi(e.text().into())),
                                  connect_activate[sender] => move |_| sender.input(AppMsg::FetchDoi),
                              },
                              gtk::Button {
                                  set_label: "Fetch BibTeX",
                                  connect_clicked[sender] => move |_| sender.input(AppMsg::FetchDoi),
                              }
                          }
                      },

                      // Search Section
                      gtk::Frame {
                          set_label: Some("Web Search (Crossref)"),
                          gtk::Box {
                              set_orientation: gtk::Orientation::Vertical,
                              set_spacing: 8,
                              set_margin_all: 8,

                              gtk::Entry {
                                  set_placeholder_text: Some("Title, Author, etc."),
                                  set_text: &model.search_input,
                                  connect_changed[sender] => move |e| sender.input(AppMsg::UpdateSearch(e.text().into())),
                                  connect_activate[sender] => move |_| sender.input(AppMsg::FetchSearch),
                              },
                              gtk::Button {
                                  set_label: "Search & Import",
                                  connect_clicked[sender] => move |_| sender.input(AppMsg::FetchSearch),
                              }
                          }
                      },

                      gtk::Spinner {
                           #[watch]
                           set_spinning: model.is_loading,
                      },

                      gtk::Separator { set_margin_top: 10, set_margin_bottom: 10 },

                      gtk::Button {
                          set_label: "Clear All Entries",
                          add_css_class: "destructive-action",
                          set_margin_top: 6,
                          connect_clicked[sender] => move |_| sender.input(AppMsg::ClearAll),
                      },

                      // Status Area
                      gtk::Label {
                          set_halign: gtk::Align::Start,
                          #[watch]
                          set_label: &model.status_msg,
                          set_wrap: true,
                          set_margin_top: 10,
                          add_css_class: "caption",
                      },
                  },

                  gtk::Separator { set_orientation: gtk::Orientation::Vertical },

                  // --- MAIN LIST ---
                  gtk::ScrolledWindow {
                      set_hexpand: true,
                      set_vexpand: true,

                      #[local_ref]
                      entries_list_box -> gtk::ListBox {
                          set_selection_mode: gtk::SelectionMode::None,
                          add_css_class: "boxed-list",
                          set_margin_all: 12,
                      }
                  }
              }
          }
      }
  }

  fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
    let key_config = core::config::load();

    // 1. Initialize Actions
    menu::actions_file::init(&root, sender.clone());
    menu::actions_edit::init(&root, sender.clone());

    // 2. Set Keyboard Shortcuts
    // FIX: Use main_application() because root.application() is often None during init
    let app = relm4::main_application();
    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.save", &["<Control>s"]);
    app.set_accels_for_action("win.save_as", &["<Control><Shift>s"]);
    app.set_accels_for_action("win.quit", &["<Control>q"]);
    app.set_accels_for_action("edit.preferences", &["<Control>comma"]);

    // 3. Build Menu Bar Visuals
    let menu_model = gio::Menu::new();

    let file_menu = gio::Menu::new();
    file_menu.append(Some("Open"), Some("win.open"));
    file_menu.append(Some("Save"), Some("win.save"));
    file_menu.append(Some("Save As..."), Some("win.save_as"));
    file_menu.append(Some("Quit"), Some("win.quit"));
    menu_model.append_submenu(Some("File"), &file_menu);

    let edit_menu = gio::Menu::new();
    edit_menu.append(Some("Preferences"), Some("edit.preferences"));
    edit_menu.append(Some("Regenerate Keys"), Some("edit.regenerate_keys"));
    menu_model.append_submenu(Some("Edit"), &edit_menu);

    let menu_bar = gtk::PopoverMenuBar::from_model(Some(&menu_model));

    // 4. Components
    let entries = FactoryVecDeque::builder()
      .launch(gtk::ListBox::default())
      .forward(sender.input_sender(), |output| AppMsg::DeleteEntry(output));

    let open_dialog = OpenDialog::builder()
      .launch(OpenDialogSettings {
        accept_label: "Open".into(),
        is_modal: true,
        ..Default::default()
      })
      .forward(sender.input_sender(), |resp| AppMsg::OpenResponse(resp));

    let save_dialog = SaveDialog::builder()
      .launch(SaveDialogSettings {
        cancel_label: "Cancel".into(),
        accept_label: "Save".into(),
        is_modal: true,
        ..Default::default()
      })
      .forward(sender.input_sender(), |resp| AppMsg::SaveResponse(resp));

    let preferences = PreferencesModel::builder()
      .transient_for(&root)
      .launch(key_config.clone())
      .forward(sender.input_sender(), |msg| match msg {
        PreferencesOutput::ConfigUpdated(cfg) => AppMsg::UpdateKeyConfig(cfg),
      });

    let model = AppModel {
      bibliography: Bibliography::new(),
      entries,
      doi_input: String::new(),
      search_input: String::new(),
      is_loading: false,
      status_msg: "Ready.".to_string(),
      open_dialog,
      save_dialog,
      preferences,
      key_config,
    };

    let entries_list_box = model.entries.widget();
    let widgets = view_output!();

    ComponentParts { model, widgets }
  }

  fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match msg {
      // --- Inputs ---
      AppMsg::UpdateDoi(v) => self.doi_input = v,
      AppMsg::UpdateSearch(v) => self.search_input = v,

      // --- Network Fetching ---
      AppMsg::FetchDoi => {
        let doi = self.doi_input.trim().to_string();
        if doi.is_empty() {
          return;
        }

        self.is_loading = true;
        self.status_msg = format!("Resolving DOI: {}...", doi);

        let sender = sender.clone();
        tokio::spawn(async move {
          match api::fetch_doi(&doi).await {
            Ok(bib) => sender.input(AppMsg::FetchSuccess(bib)),
            Err(e) => sender.input(AppMsg::FetchError(e.to_string())),
          }
        });
      }

      AppMsg::FetchSearch => {
        let query = self.search_input.trim().to_string();
        if query.is_empty() {
          return;
        }

        self.is_loading = true;
        self.status_msg = format!("Searching Crossref: {}...", query);

        let sender = sender.clone();
        tokio::spawn(async move {
          match api::search_crossref(&query).await {
            Ok(bib) => sender.input(AppMsg::FetchSuccess(bib)),
            Err(e) => sender.input(AppMsg::FetchError(e.to_string())),
          }
        });
      }

      // --- Async Results ---
      AppMsg::FetchSuccess(new_bib) => {
        self.is_loading = false;
        self.status_msg = format!("Imported {} entries.", new_bib.len());
        for entry in new_bib {
          sender.input(AppMsg::AddBiblatexEntry(entry));
        }
        self.doi_input.clear();
        self.search_input.clear();
      }

      AppMsg::FetchError(err) => {
        self.is_loading = false;
        self.status_msg = format!("Error: {}", err);
      }

      // --- Data Logic ---
      AppMsg::AddBiblatexEntry(mut entry) => {
        // Generate Key
        entry.key = core::keygen::generate_key(&entry, &self.key_config);

        self.bibliography.insert(entry.clone());

        self.entries.guard().push_front(ui::row::BibEntry {
          key: entry.key.clone(),
          title: entry
            .fields
            .get("title")
            .map(|t| core::bib_to_string(t))
            .unwrap_or_else(|| "Untitled".to_string()),
          kind: entry.entry_type.to_string(),
          is_error: false,
        });
      }

      AppMsg::DeleteEntry(key) => {
        self.bibliography.remove(&key);
        let mut guard = self.entries.guard();
        let index_opt = guard.iter().position(|e| e.key == key);
        if let Some(index) = index_opt {
          guard.remove(index);
        }
        self.status_msg = format!("Deleted entry {}", key);
      }

      AppMsg::ClearAll => {
        self.bibliography = Bibliography::new();
        self.entries.guard().clear();
        self.status_msg = "Cleared library.".to_string();
      }

      // --- Config & Regen ---
      AppMsg::UpdateKeyConfig(cfg) => {
        self.key_config = cfg.clone();
        core::config::save(&cfg);
        sender.input(AppMsg::RegenerateAllKeys);
      }

      AppMsg::RegenerateAllKeys => {
        let old_bib = self.bibliography.clone();
        self.bibliography = Bibliography::new();
        self.entries.guard().clear();
        for entry in old_bib.iter() {
          sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
        }
        self.status_msg = "Regenerated all keys.".to_string();
      }

      // --- Triggers / Menus ---
      AppMsg::TriggerOpen => self.open_dialog.emit(OpenDialogMsg::Open),
      AppMsg::TriggerSave => self
        .save_dialog
        .emit(SaveDialogMsg::SaveAs("bibliography.bib".into())),
      AppMsg::TriggerSaveAs => self
        .save_dialog
        .emit(SaveDialogMsg::SaveAs("bibliography.bib".into())),
      AppMsg::ShowPreferences => self.preferences.emit(PreferencesMsg::Show),

      // --- File Responses ---
      AppMsg::OpenResponse(OpenDialogResponse::Accept(path)) => {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if let Ok(bib) = Bibliography::parse(&content) {
          for entry in bib.iter() {
            sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
          }
          self.status_msg = format!("Loaded {} entries.", bib.len());
        } else {
          self.status_msg = "Failed to parse file.".to_string();
        }
      }
      AppMsg::SaveResponse(SaveDialogResponse::Accept(path)) => {
        let output = self.bibliography.to_bibtex_string();
        if std::fs::write(&path, output).is_ok() {
          self.status_msg = "Library saved.".to_string();
        } else {
          self.status_msg = "Failed to save file.".to_string();
        }
      }
      _ => {} // Ignore Cancel responses
    }
  }
}
