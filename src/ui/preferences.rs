// src/ui/preferences.rs

use crate::core::keygen::{KeyGenConfig, KeyPart};
use gtk4::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

// ----------------------------------------------------------------------------
// Component 1: KeyPartRow (Key Generator Fields)
// ----------------------------------------------------------------------------
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
        if let KeyPartRowMsg::Remove = msg {
            let _ = sender.output(self.index);
        }
    }
}

// ----------------------------------------------------------------------------
// Component 2: FieldRow (Field Ordering)
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct FieldRow {
    pub field_name: String,
    pub index: usize,
}

#[derive(Debug)]
pub enum FieldRowMsg {
    MoveUp,
    MoveDown,
}

#[relm4::factory(pub)]
impl FactoryComponent for FieldRow {
    type Init = (usize, String);
    type Input = FieldRowMsg;
    type Output = (usize, FieldRowMsg);
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::ListBoxRow {
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_all: 8,
                set_spacing: 10,

                gtk::Label {
                    set_label: &self.field_name,
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["monospace"],
                },

                gtk::Button {
                    set_icon_name: "go-up-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Move Up"),
                    connect_clicked => FieldRowMsg::MoveUp,
                },
                gtk::Button {
                    set_icon_name: "go-down-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Move Down"),
                    connect_clicked => FieldRowMsg::MoveDown,
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let (idx, name) = init;
        Self {
            field_name: name,
            index: idx,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        let _ = sender.output((self.index, msg));
    }
}

// ----------------------------------------------------------------------------
// Main Preferences Model
// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct PreferencesModel {
    pub config: KeyGenConfig,
    pub parts_list: FactoryVecDeque<KeyPartRow>,
    pub fields_list: FactoryVecDeque<FieldRow>,
    // ✅ FIX 1: Add visibility state
    pub is_visible: bool,
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
    SetIndentChar(char),
    SetIndentWidth(f64),
    MoveField(usize, FieldRowMsg),
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
            set_default_width: 600,
            set_default_height: 650,
            set_title: Some("Preferences"),

            // ✅ FIX 2: Bind visibility to the model
            #[watch]
            set_visible: model.is_visible,

            connect_close_request[sender] => move |_| {
                sender.input(PreferencesMsg::Close);
                gtk::glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                // --- TABS (Stack Switcher) ---
                gtk::StackSwitcher {
                    set_stack: Some(&main_stack),
                    set_halign: gtk::Align::Center,
                    set_margin_all: 6,
                },

                // --- CONTENT (Stack) ---
                #[name(main_stack)]
                gtk::Stack {
                    set_vexpand: true,
                    set_transition_type: gtk::StackTransitionType::Crossfade,

                    // --- TAB 1: General ---
                    add_titled[Some("general"), "General"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 12,
                        set_spacing: 12,

                        gtk::Label {
                            set_label: "Citation Key Generator",
                            set_css_classes: &["title-4"],
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            gtk::Label { set_label: "Separator:" },

                            // ✅ CHANGED: DropDown for Separator
                            gtk::DropDown {
                                set_model: Some(&gtk::StringList::new(&["None", "- (Hyphen)", "_ (Underscore)"])),

                                #[watch]
                                set_selected: match model.config.separator.as_str() {
                                    "-" => 1,
                                    "_" => 2,
                                    _ => 0,
                                },

                                connect_selected_notify[sender] => move |dd| {
                                    let sep = match dd.selected() {
                                        1 => "-",
                                        2 => "_",
                                        _ => "",
                                    };
                                    sender.input(PreferencesMsg::SetSeparator(sep.to_string()));
                                }
                            },
                        },

                        gtk::Label {
                            set_label: "Key Format Parts:",
                            set_halign: gtk::Align::Start,
                            add_css_class: "dim-label",
                        },

                        gtk::Frame {
                            #[local_ref]
                            parts_listbox -> gtk::ListBox {
                                set_selection_mode: gtk::SelectionMode::None,
                                add_css_class: "boxed-list",
                            }
                        },

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

                        gtk::Separator { set_margin_top: 10, set_margin_bottom: 10 },

                        gtk::Label {
                            set_label: "Import Settings",
                            set_css_classes: &["title-4"],
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            gtk::Label {
                                set_label: "Auto-abbreviate Journal Titles:",
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
                    },

                    // --- TAB 2: Formatting ---
                    add_titled[Some("formatting"), "Formatting"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 12,
                        set_spacing: 12,

                        gtk::Label {
                            set_label: "Indentation",
                            set_css_classes: &["title-4"],
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,

                            gtk::Label { set_label: "Style:" },

                            gtk::DropDown {
                                set_model: Some(&gtk::StringList::new(&["Spaces", "Tabs"])),
                                #[watch]
                                set_selected: if model.config.indent_char == '\t' { 1 } else { 0 },
                                connect_selected_notify[sender] => move |d| {
                                    let char = if d.selected() == 1 { '\t' } else { ' ' };
                                    sender.input(PreferencesMsg::SetIndentChar(char));
                                }
                            },

                            gtk::Label { set_label: "Width:" },

                            gtk::SpinButton {
                                set_range: (1.0, 8.0),
                                set_digits: 0,
                                set_increments: (1.0, 1.0),
                                #[watch]
                                set_value: model.config.indent_width as f64,
                                #[watch]
                                set_sensitive: model.config.indent_char == ' ',
                                connect_value_changed[sender] => move |btn| {
                                    sender.input(PreferencesMsg::SetIndentWidth(btn.value()));
                                }
                            }
                        },

                        gtk::Separator { set_margin_top: 10, set_margin_bottom: 10 },

                        gtk::Label {
                            set_label: "Field Ordering",
                            set_css_classes: &["title-4"],
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Label {
                            set_label: "Determines the order of fields when generating new entries.",
                            set_css_classes: &["caption"],
                            set_halign: gtk::Align::Start,
                        },

                        gtk::ScrolledWindow {
                            set_vexpand: true,
                            gtk::Frame {
                                #[local_ref]
                                fields_listbox -> gtk::ListBox {
                                    set_selection_mode: gtk::SelectionMode::None,
                                    add_css_class: "boxed-list",
                                }
                            }
                        },
                    },
                },

                // --- Bottom Actions ---
                gtk::Box {
                    set_halign: gtk::Align::End,
                    set_spacing: 12,
                    set_margin_all: 12,

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

        let fields_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |(idx, msg)| {
                PreferencesMsg::MoveField(idx, msg)
            });

        // ✅ FIX 3: Init with is_visible = false
        let mut model = PreferencesModel {
            config,
            parts_list,
            fields_list,
            is_visible: false,
        };

        // 1. Populate Key Parts
        for (i, part) in model.config.parts.iter().enumerate() {
            model.parts_list.guard().push_back((i, part.clone()));
        }

        // 2. Populate Field Order
        let mut field_order = model.config.field_order.clone();
        if field_order.is_empty() {
            field_order = vec![
                "author".into(),
                "title".into(),
                "year".into(),
                "date".into(),
                "journal".into(),
                "journaltitle".into(),
                "volume".into(),
                "doi".into(),
            ];
        }

        for (i, field) in field_order.iter().enumerate() {
            model.fields_list.guard().push_back((i, field.clone()));
        }

        let parts_listbox = model.parts_list.widget();
        let fields_listbox = model.fields_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            // ✅ FIX 4: Handle Showing/Closing
            PreferencesMsg::Show => self.is_visible = true,
            PreferencesMsg::Close => self.is_visible = false,

            PreferencesMsg::Save => {
                let _ = sender.output(PreferencesOutput::ConfigUpdated(self.config.clone()));
                self.is_visible = false; // Close on save
            }

            // --- Tab 1 ---
            PreferencesMsg::SetSeparator(s) => self.config.separator = s,
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
            PreferencesMsg::ToggleAbbreviate(state) => self.config.abbreviate_journals = state,

            // --- Tab 2 ---
            PreferencesMsg::SetIndentChar(c) => self.config.indent_char = c,
            PreferencesMsg::SetIndentWidth(w) => self.config.indent_width = w as u8,

            PreferencesMsg::MoveField(idx, move_msg) => {
                let mut guard = self.fields_list.guard();
                let len = guard.len();

                match move_msg {
                    FieldRowMsg::MoveUp if idx > 0 => {
                        guard.swap(idx, idx - 1);
                        self.config.field_order.swap(idx, idx - 1);

                        // ✅ FIX 5: Update the internal index of swapped items
                        // (Otherwise clicking them again sends the old index)
                        if let Some(item) = guard.get_mut(idx) {
                            item.index = idx;
                        }
                        if let Some(item) = guard.get_mut(idx - 1) {
                            item.index = idx - 1;
                        }
                    }
                    FieldRowMsg::MoveDown if idx < len - 1 => {
                        guard.swap(idx, idx + 1);
                        self.config.field_order.swap(idx, idx + 1);

                        // ✅ FIX 5 (Down)
                        if let Some(item) = guard.get_mut(idx) {
                            item.index = idx;
                        }
                        if let Some(item) = guard.get_mut(idx + 1) {
                            item.index = idx + 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
