// relm4's view! macro generates unused_assignments for #[local_ref] and #[name] bindings
#![allow(unused_assignments)]

use biblatex::Bibliography;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::FileFilter;
use libadwaita as adw;
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
// use crate::ui;
use crate::ui::details_dialog::{DetailsDialogModel, DetailsDialogOutput};
use crate::ui::duplicate_dialog::{DuplicateDialogModel, DuplicateDialogOutput}; // <--- FIX 1: ADD IMPORT
use crate::ui::preferences::{PreferencesModel, PreferencesOutput};
use crate::ui::row::BibEntryOutput;
use crate::ui::search_dialog::{SearchDialogModel, SearchDialogOutput};
use crate::ui::sidebar::{SidebarModel, SidebarOutput};
use crate::ui::unsaved_dialog::UnsavedDialogModel;

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
                connect_close_request[sender] => move |_| {
                    sender.input(AppMsg::TriggerQuit);
                    gtk::glib::Propagation::Stop
                },

                #[wrap(Some)]
                set_titlebar = &adw::HeaderBar {
                    pack_start = &gtk::Button {
                        set_icon_name: "document-open-symbolic",
                        set_tooltip_text: Some("Open (Ctrl+O)"),
                        set_action_name: Some("win.open"),
                    },
                    pack_start = &gtk::Button {
                        set_icon_name: "document-save-symbolic",
                        set_tooltip_text: Some("Save (Ctrl+S)"),
                        set_action_name: Some("win.save"),
                    },
                    pack_end = &gtk::MenuButton {
                        set_icon_name: "open-menu-symbolic",
                        set_tooltip_text: Some("Main Menu"),
                        set_menu_model: Some(&primary_menu),
                        set_primary: true,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 0,

                    gtk::Paned {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_position: 330,
                        set_shrink_start_child: false,
                        set_shrink_end_child: false,
                        set_resize_start_child: true,
                        set_resize_end_child: true,
                        set_vexpand: true,
                        set_hexpand: true,

                        #[wrap(Some)]
                        set_start_child = &gtk::ScrolledWindow {
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vscrollbar_policy: gtk::PolicyType::Automatic,

                            #[local_ref]
                            sidebar_widget -> gtk::Box {},
                        },

                        #[wrap(Some)]
                        set_end_child = &gtk::ScrolledWindow {
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
                },
                // KEYBOARD CONTROLLER for undo/redo
                add_controller = gtk::EventControllerKey {
                  connect_key_pressed[sender] => move |_controller, keyval, _keycode, state| {
                    let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                    let is_shift = state.contains(gtk::gdk::ModifierType::SHIFT_MASK);

                    // Ctrl + Z -> UNDO
                    if is_ctrl && keyval == gtk::gdk::Key::z && !is_shift {
                      sender.input(AppMsg::Undo);
                      return gtk::glib::Propagation::Stop;
                    }

                    // Ctrl + Shift + Z  OR  Ctrl + Y -> REDO
                    if (is_ctrl && is_shift && keyval == gtk::gdk::Key::z) ||
                      (is_ctrl && keyval == gtk::gdk::Key::y) {
                        sender.input(AppMsg::Redo);
                        return gtk::glib::Propagation::Stop;
                    }

                    gtk::glib::Propagation::Proceed
                  }
                }        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // FIX 2: Load config FIRST so 'key_config' variable exists
        let key_config = core::config::load().unwrap_or_else(|error| {
            eprintln!("Failed to load preferences; using defaults: {error}");
            Default::default()
        });

        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("../style.css"));
        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&root),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

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

        // Primary (hamburger) menu, GNOME HIG style
        let primary_menu = gio::Menu::new();

        let file_section = gio::Menu::new();
        file_section.append(Some("Save As…"), Some("win.save_as"));
        primary_menu.append_section(None, &file_section);

        let tools_section = gio::Menu::new();
        tools_section.append(Some("Regenerate Keys"), Some("edit.regenerate_keys"));
        tools_section.append(Some("Reformat All Entries"), Some("edit.reformat_all"));
        tools_section.append(Some("Scan for Duplicates"), Some("edit.scan_duplicates"));
        tools_section.append(
            Some("Abbreviate Journal Titles"),
            Some("edit.abbreviate_journals"),
        );
        tools_section.append(
            Some("Un-abbreviate Journal Titles"),
            Some("edit.unabbreviate_journals"),
        );
        primary_menu.append_section(None, &tools_section);

        let app_section = gio::Menu::new();
        app_section.append(Some("Preferences"), Some("edit.preferences"));
        app_section.append(Some("About MkBib"), Some("win.about"));
        primary_menu.append_section(None, &app_section);

        let entries = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output: BibEntryOutput| {
                AppMsg::HandleRowOutput(output)
            });

        let sidebar = SidebarModel::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                SidebarOutput::FetchDoi(doi) => AppMsg::FetchDoi(doi),
                SidebarOutput::SearchCrossref(q) => AppMsg::FetchSearch(q),
                SidebarOutput::ParseManual(txt) => AppMsg::ParseManualBib(txt),
                SidebarOutput::ClearAll => AppMsg::ClearAll,
            });

        let sidebar_widget = sidebar.widget().clone();

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
            .forward(sender.input_sender(), AppMsg::OpenResponse);
        open_dialog.widget().set_transient_for(Some(&root));

        let save_dialog = SaveDialog::builder()
            .launch(SaveDialogSettings {
                cancel_label: "Cancel".into(),
                accept_label: "Save".into(),
                is_modal: true,
                ..Default::default()
            })
            .forward(sender.input_sender(), AppMsg::SaveResponse);
        save_dialog.widget().set_transient_for(Some(&root));

        let preferences = PreferencesModel::builder()
            .transient_for(&root)
            .launch(key_config.clone()) // Now 'key_config' exists!
            .forward(sender.input_sender(), |msg| match msg {
                PreferencesOutput::ConfigUpdated(cfg) => AppMsg::UpdateKeyConfig(cfg),
            });

        let details_dialog = DetailsDialogModel::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                DetailsDialogOutput::Saved(old_key, text) => AppMsg::FinishEditEntry(old_key, text),
            });

        let search_dialog = SearchDialogModel::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                SearchDialogOutput::FetchDoi(doi) => AppMsg::FetchSelectedDoi(doi),
            });

        // FIX 3: Initialize DuplicateDialog
        let duplicate_dialog = DuplicateDialogModel::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                DuplicateDialogOutput::DeleteEntry(key) => AppMsg::DeleteEntry(key),
            });

        let unsaved_dialog = UnsavedDialogModel::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), AppMsg::UnsavedDecision);

        let alert = AlertModel::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let model = AppModel {
            bibliography: Bibliography::new(),
            entries,
            original_file_content: None,
            current_file_path: None,
            sidebar,
            open_dialog,
            save_dialog,
            preferences,
            alert,
            details_dialog,
            search_dialog,
            duplicate_dialog,
            unsaved_dialog,
            key_config,
            is_dirty: false,
            undo_stack: std::collections::VecDeque::new(),
            redo_stack: std::collections::VecDeque::new(),
            current_revision: 0,
            saved_revision: 0,
            next_revision: 1,
            pending_action: None,
        };

        let entries_list_box = model.entries.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        update::handle_msg(self, msg, sender);
    }
}
