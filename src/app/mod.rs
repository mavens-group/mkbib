// src/app/mod.rs

#![allow(unused_assignments)]

use biblatex::Bibliography;
use gtk4::gio;
use gtk4::prelude::*;
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
use crate::ui::sidebar::{SidebarModel, SidebarOutput};

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

                    #[local_ref]
                    sidebar_widget -> gtk::Box {},

                    gtk::Separator { set_orientation: gtk::Orientation::Vertical },

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

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
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
            .forward(sender.input_sender(), |resp| AppMsg::OpenResponse(resp));
        open_dialog.widget().set_transient_for(Some(&root));

        let save_dialog = SaveDialog::builder()
            .launch(SaveDialogSettings {
                cancel_label: "Cancel".into(),
                accept_label: "Save".into(),
                is_modal: true,
                ..Default::default()
            })
            .forward(sender.input_sender(), |resp| AppMsg::SaveResponse(resp));
        save_dialog.widget().set_transient_for(Some(&root));

        let preferences = PreferencesModel::builder()
            .transient_for(&root)
            .launch(key_config.clone())
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

        let alert = AlertModel::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let model = AppModel {
            bibliography: Bibliography::new(),
            entries,
            current_file_path: None,
            // REMOVED old fields (doi_input, etc)
            sidebar,
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

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        update::handle_msg(self, msg, sender);
    }
}
