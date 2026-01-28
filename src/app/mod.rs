// src/app/mod.rs
use biblatex::Bibliography;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*; // Import prelude to get NativeDialogExt for set_transient_for
use gtk4::FileFilter;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use relm4_components::open_dialog::{OpenDialog, OpenDialogSettings};
use relm4_components::save_dialog::{SaveDialog, SaveDialogSettings};

pub mod alert;
pub mod model;
pub mod update;

pub use model::{AppModel, AppMsg};

use self::alert::AlertModel;
use crate::core;
use crate::menu;
use crate::ui::details_dialog::{DetailsDialogModel, DetailsDialogOutput};
use crate::ui::preferences::{PreferencesModel, PreferencesOutput};
use crate::ui::row::BibEntryOutput;
use crate::ui::search_dialog::{SearchDialogModel, SearchDialogOutput};

#[relm4::component(pub)]
impl Component for AppModel {
  type Init = ();
  type Input = AppMsg;
  type Output = ();
  type CommandOutput = ();

  view! {
      gtk::ApplicationWindow {
          set_title: Some("MkBib"),
          set_icon_name: Some("mkbib"),
          set_default_width: 1100,
          set_default_height: 750,

          gtk::Box {
              set_orientation: gtk::Orientation::Vertical,
              set_spacing: 0,

              #[local_ref]
              menu_bar -> gtk::PopoverMenuBar {},

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

                      // 1. DOI Import
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

                      // 2. Web Search
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

                      // 3. Manual Bib Entry
                      gtk::Frame {
                          set_label: Some("Manual Entry"),
                          gtk::Box {
                              set_orientation: gtk::Orientation::Vertical,
                              set_spacing: 8,
                              set_margin_all: 8,

                              gtk::ScrolledWindow {
                                  set_height_request: 120,
                                  set_has_frame: true,
                                  set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),

                                  #[name = "manual_text_view"]
                                  gtk::TextView {
                                      set_wrap_mode: gtk::WrapMode::WordChar,
                                      set_top_margin: 8,
                                      set_bottom_margin: 8,
                                      set_left_margin: 8,
                                      set_right_margin: 8,

                                      #[watch]
                                      set_visible: {
                                          let buffer = manual_text_view.buffer();
                                          let current = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                                          if model.manual_bib_input != current {
                                              buffer.set_text(&model.manual_bib_input);
                                          }
                                          true
                                      }
                                  },
                              },

                              gtk::Button {
                                  set_label: "Add Entry",
                                  connect_clicked[sender] => move |_| sender.input(AppMsg::ParseManualBib),
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
                          set_activate_on_single_click: true,
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

    menu::actions_file::init(&root, sender.clone());
    menu::actions_edit::init(&root, sender.clone());
    menu::actions_help::init(&root, sender.clone());

    let app = relm4::main_application();
    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.save", &["<Control>s"]);
    app.set_accels_for_action("win.save_as", &["<Control><Shift>s"]);
    app.set_accels_for_action("win.quit", &["<Control>q"]);
    app.set_accels_for_action("edit.preferences", &["<Control>comma"]);
    app.set_accels_for_action("win.about", &["F1"]);

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
    edit_menu.append(Some("Scan for Duplicates"), Some("edit.scan_duplicates"));
    menu_model.append_submenu(Some("Edit"), &edit_menu);

    let help_menu = gio::Menu::new();
    help_menu.append(Some("About MkBib"), Some("win.about"));
    menu_model.append_submenu(Some("Help"), &help_menu);

    let menu_bar = gtk::PopoverMenuBar::from_model(Some(&menu_model));

    // --- Entries Factory ---
    let entries = FactoryVecDeque::builder()
      .launch(gtk::ListBox::default())
      .forward(sender.input_sender(), |output: BibEntryOutput| {
        AppMsg::HandleRowOutput(output)
      });

    // --- Open Dialog with Filter ---
    // FIX: Removed .transient_for(&root) from builder
    let open_dialog = OpenDialog::builder()
      .launch(OpenDialogSettings {
        accept_label: "Open".into(),
        is_modal: true,
        filters: vec![{
          let f = FileFilter::new();
          f.set_name(Some("BibTeX Files (*.bib)"));
          f.add_pattern("*.bib");
          f
        }],
        ..Default::default()
      })
      .forward(sender.input_sender(), |resp| AppMsg::OpenResponse(resp));

    // FIX: Manually set transient parent after creation
    open_dialog.widget().set_transient_for(Some(&root));

    let save_dialog = SaveDialog::builder()
      .launch(SaveDialogSettings {
        cancel_label: "Cancel".into(),
        accept_label: "Save".into(),
        is_modal: true,
        ..Default::default()
      })
      .forward(sender.input_sender(), |resp| AppMsg::SaveResponse(resp));

    // FIX: Manually set transient parent for Save Dialog too
    save_dialog.widget().set_transient_for(Some(&root));

    let preferences = PreferencesModel::builder()
      .transient_for(&root)
      .launch(key_config.clone())
      .forward(sender.input_sender(), |msg| match msg {
        PreferencesOutput::ConfigUpdated(cfg) => AppMsg::UpdateKeyConfig(cfg),
      });

    // --- Details Dialog ---
    let details_dialog = DetailsDialogModel::builder()
      .transient_for(&root)
      .launch(())
      .forward(sender.input_sender(), |output| match output {
        DetailsDialogOutput::Saved(old_key, text) => AppMsg::FinishEditEntry(old_key, text),
      });

    // --- Search Dialog ---
    let search_dialog = SearchDialogModel::builder()
      .transient_for(&root)
      .launch(())
      .forward(sender.input_sender(), |output| match output {
        SearchDialogOutput::FetchDoi(doi) => AppMsg::FetchSelectedDoi(doi),
      });

    let alert = AlertModel::builder()
      .transient_for(&root)
      .launch(())
      .detach();

    let model = AppModel {
      bibliography: Bibliography::new(),
      entries,
      doi_input: String::new(),
      search_input: String::new(),
      manual_bib_input: String::new(),
      is_loading: false,
      status_msg: "Ready.".to_string(),
      open_dialog,
      save_dialog,
      preferences,
      alert,
      details_dialog,
      search_dialog,
      key_config,
    };

    let entries_list_box = model.entries.widget();
    let widgets = view_output!();

    let buffer = widgets.manual_text_view.buffer();
    buffer.connect_changed(glib::clone!(@strong sender => move |buff| {
        let text = buff.text(&buff.start_iter(), &buff.end_iter(), true).to_string();
        sender.input(AppMsg::UpdateManualBib(text));
    }));

    ComponentParts { model, widgets }
  }

  fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    update::handle_msg(self, msg, sender);
  }
}
