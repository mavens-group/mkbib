// src/ui/preferences.rs
use crate::core::keygen::{KeyGenConfig, KeyPart};
use gtk4::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

// --- KeyPartRow (Unchanged) ---
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
            label: format!("{}. {}", idx + 1, part.label()),
            index: idx,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            KeyPartRowMsg::Remove => {
                let _ = sender.output(self.index);
            }
        }
    }
}

// --- Main Preferences Model ---

pub struct PreferencesModel {
    pub config: KeyGenConfig,
    pub parts_list: FactoryVecDeque<KeyPartRow>,
}

#[derive(Debug)]
pub enum PreferencesMsg {
    Show,
    Close,
    AddPart(KeyPart),
    RemovePart(usize),
    SetSeparator(String),
    Save,
}

#[derive(Debug)]
pub enum PreferencesOutput {
    ConfigUpdated(KeyGenConfig),
}

#[relm4::component(pub)]
impl Component for PreferencesModel {
    type Init = KeyGenConfig;
    type Input = PreferencesMsg;
    type Output = PreferencesOutput;
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_title: Some("Preferences"),
            set_modal: true,
            set_default_width: 400,
            set_default_height: 550,
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
                    set_label: "BibTeX Key Generator",
                    add_css_class: "title-4",
                    set_halign: gtk::Align::Start,
                },

                // --- SEPARATOR SELECTION (Using DropDown) ---
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Label { set_label: "Separator:" },

                    // FIX: Replaced ComboBoxText with DropDown
                    gtk::DropDown {
                        set_model: Some(&gtk::StringList::new(&[
                            "None (e.g. AuthorYear)",
                            "Underscore (Author_Year)",
                            "Hyphen (Author-Year)"
                        ])),

                        // Set initial selection based on config
                        set_selected: match model.config.separator.as_str() {
                            "_" => 1,
                            "-" => 2,
                            _ => 0,
                        },

                        connect_selected_item_notify[sender] => move |dd| {
                            let sep = match dd.selected() {
                                1 => "_".to_string(),
                                2 => "-".to_string(),
                                _ => "".to_string(),
                            };
                            sender.input(PreferencesMsg::SetSeparator(sep));
                        }
                    }
                },

                gtk::Label {
                    set_label: "Field Order:",
                    set_halign: gtk::Align::Start,
                    set_margin_top: 8,
                },

                gtk::Frame {
                    gtk::ScrolledWindow {
                        set_height_request: 250,
                        set_vexpand: true,

                        #[local_ref]
                        parts_listbox -> gtk::ListBox {
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                        }
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_top: 8,

                    gtk::MenuButton {
                        set_label: "Add Field...",
                        set_icon_name: "list-add-symbolic",
                        set_direction: gtk::ArrowType::Up,

                        #[wrap(Some)]
                        set_popover = &gtk::Popover {
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 4,
                                set_margin_all: 4,

                                gtk::Button { set_label: "Author (Last Name)", connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::AuthorLastName)) },
                                gtk::Button { set_label: "Year (Full: 2024)", connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::Year)) },
                                gtk::Button { set_label: "Year (Short: 24)", connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::ShortYear)) },
                                gtk::Button { set_label: "Title (1st Word)", connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::TitleFirstWord)) },
                                gtk::Button { set_label: "Journal (1st Word)", connect_clicked[sender] => move |_| sender.input(PreferencesMsg::AddPart(KeyPart::JournalFirstWord)) },
                            }
                        }
                    },

                    gtk::Box { set_hexpand: true },

                    gtk::Button {
                        set_label: "Apply & Save",
                        add_css_class: "suggested-action",
                        connect_clicked[sender] => move |_| sender.input(PreferencesMsg::Save),
                    }
                }
            }
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut parts_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |idx| PreferencesMsg::RemovePart(idx));

        for (i, part) in config.parts.iter().enumerate() {
            parts_list.guard().push_back((i, part.clone()));
        }

        let model = PreferencesModel { config, parts_list };

        // FIX: Ensure this name matches the local_ref
        let parts_listbox = model.parts_list.widget();

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            PreferencesMsg::Show => {
                root.set_visible(true);
                root.present();
            }
            PreferencesMsg::Close => {
                root.set_visible(false);
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

            PreferencesMsg::Save => {
                root.set_visible(false);
                let _ = sender.output(PreferencesOutput::ConfigUpdated(self.config.clone()));
            }
        }
    }
}
