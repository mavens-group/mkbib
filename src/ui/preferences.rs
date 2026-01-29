// src/ui/preferences.rs

use crate::core::keygen::{KeyGenConfig, KeyPart};
use gtk4::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

// --- KeyPartRow ---
#[derive(Debug)]
pub struct KeyPartRow {
    pub label: String,
    pub index: usize,
}

#[derive(Debug)]
pub enum KeyPartRowMsg {
    Remove,
}

#[relm4::factory(pub)]
impl FactoryComponent for KeyPartRow {
    type Init = (usize, KeyPart);
    type Input = KeyPartRowMsg;
    type Output = usize;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::ListBoxRow {
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_all: 8,

                gtk::Label {
                    set_label: &self.label,
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                },

                gtk::Button {
                    set_icon_name: "list-remove-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Remove field"),
                    connect_clicked => KeyPartRowMsg::Remove,
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let (idx, part) = init;
        Self {
            label: format!("{} - {}", idx + 1, part.label()),
            index: idx,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            KeyPartRowMsg::Remove => {
                // Send the index back to the parent
                let _ = sender.output(self.index);
            }
        }
    }
}

// --- PreferencesModel ---

#[derive(Debug)]
pub struct PreferencesModel {
    pub config: KeyGenConfig,
    pub parts_list: FactoryVecDeque<KeyPartRow>,
}

#[derive(Debug)]
pub enum PreferencesMsg {
    Show,
    Close,
    Save,
    SetSeparator(String),
    AddPart(KeyPart),
    RemovePart(usize),
    ToggleAbbreviate(bool),
}

#[derive(Debug)]
pub enum PreferencesOutput {
    ConfigUpdated(KeyGenConfig),
}

#[relm4::component(pub)]
impl SimpleComponent for PreferencesModel {
    type Init = KeyGenConfig;
    type Input = PreferencesMsg;
    type Output = PreferencesOutput;

    view! {
        gtk::Window {
            set_modal: true,
            set_default_width: 500,
            set_default_height: 600,
            set_title: Some("Preferences"),
            set_hide_on_close: true,

            connect_close_request[sender] => move |_| {
                sender.input(PreferencesMsg::Close);
                gtk::glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 12,
                set_spacing: 12,

                gtk::Label {
                    set_label: "Citation Key Generator",
                    set_css_classes: &["title-4"],
                    set_halign: gtk::Align::Start,
                },

                // --- Separator Settings ---
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Label { set_label: "Separator:" },
                    gtk::Entry {
                        set_placeholder_text: Some("e.g. - or _"),
                        set_text: &model.config.separator,
                        connect_changed[sender] => move |e| {
                            sender.input(PreferencesMsg::SetSeparator(e.text().into()));
                        }
                    }
                },

                gtk::Label {
                    set_label: "Key Format Parts (Drag to reorder - TODO)",
                    set_halign: gtk::Align::Start,
                    add_css_class: "dim-label",
                },

                // --- Key Parts List ---
                gtk::Frame {
                    #[local_ref]
                    parts_listbox -> gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        add_css_class: "boxed-list",
                    }
                },

                // --- Add Part Buttons ---
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "+ Author",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::AuthorLastName)),
                    },
                    gtk::Button {
                        set_label: "+ Year",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::Year)),
                    },
                    gtk::Button {
                        set_label: "+ Title",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::TitleFirstWord)),
                    },
                    gtk::Button {
                        set_label: "+ Journal",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::JournalFirstWord)),
                    },
                },

                gtk::Separator {
                    set_margin_top: 10,
                    set_margin_bottom: 10,
                },

                // --- Import Settings (NEW SECTION) ---
                gtk::Label {
                    set_label: "Import Settings",
                    set_css_classes: &["title-4"],
                    set_halign: gtk::Align::Start,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Label {
                        set_label: "Auto-abbreviate Journal Titles on Import",
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Switch {
                        #[watch]
                        set_active: model.config.abbreviate_journals,
                        connect_state_set[sender] => move |_, state| {
                            sender.input(PreferencesMsg::ToggleAbbreviate(state));
                            gtk::glib::Propagation::Stop
                        }
                    }
                },

                gtk::Box {
                    set_vexpand: true, // Spacer
                },

                // --- Actions ---
                gtk::Box {
                    set_halign: gtk::Align::End,
                    set_spacing: 12,

                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::Close),
                    },
                    gtk::Button {
                        set_label: "Save Configuration",
                        add_css_class: "suggested-action",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::Save),
                    },
                }
            }
        }
    }

    fn init(
        config: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let parts_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| {
                PreferencesMsg::RemovePart(output)
            });

        let mut model = PreferencesModel { config, parts_list };

        // Populate initial list
        for (i, part) in model.config.parts.iter().enumerate() {
            model.parts_list.guard().push_back((i, part.clone()));
        }

        let parts_listbox = model.parts_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PreferencesMsg::Show => {
                // Window visibility handled by view! watch
            }
            PreferencesMsg::Close => {
                // Handled by view! connect_close_request
            }

            PreferencesMsg::SetSeparator(s) => {
                self.config.separator = s;
            }

            PreferencesMsg::AddPart(part) => {
                self.config.parts.push(part.clone());
                let idx = self.config.parts.len() - 1;
                self.parts_list.guard().push_back((idx, part));
            }

            PreferencesMsg::RemovePart(index) => {
                if index < self.config.parts.len() {
                    self.config.parts.remove(index);
                    self.parts_list.guard().clear();
                    for (i, part) in self.config.parts.iter().enumerate() {
                        self.parts_list.guard().push_back((i, part.clone()));
                    }
                }
            }

            PreferencesMsg::ToggleAbbreviate(state) => {
                self.config.abbreviate_journals = state;
            }

            PreferencesMsg::Save => {
                let _ = sender.output(PreferencesOutput::ConfigUpdated(self.config.clone()));
            }
        }
    }
}
