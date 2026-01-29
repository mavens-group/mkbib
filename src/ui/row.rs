use crate::core;
use gtk4::prelude::*;
use relm4::prelude::*; // Add this if not present

#[derive(Debug, Clone)]
pub struct BibEntry {
    pub key: String,
    pub title: String,
    pub kind: String,
    pub is_error: bool,
}

impl BibEntry {
    // NEW HELPER
    pub fn from_entry(entry: &biblatex::Entry) -> Self {
        let title = entry
            .fields
            .get("title")
            .map(|t| core::bib_to_string(t))
            .unwrap_or_else(|| "Untitled".to_string());

        BibEntry {
            key: entry.key.clone(),
            title,
            kind: format!("{}", entry.entry_type),
            is_error: false,
        }
    }
}

#[derive(Debug)]
pub enum BibEntryMsg {
    Delete,
    Select,
}

#[derive(Debug)]
pub enum BibEntryOutput {
    Delete(String),
    Select(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for BibEntry {
    type Init = BibEntry;
    type Input = BibEntryMsg;
    type Output = BibEntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::ListBoxRow {
            set_class_active: ("error", self.is_error),

            // FIX: Use GestureClick instead of 'set_activatable'.
            // This forces the click to be detected even if the ListBox
            // selection mode or focus rules would otherwise ignore it.
            add_controller = gtk::GestureClick {
                set_button: 1, // Left click only
                connect_released[sender] => move |_, _, _, _| {
                    sender.input(BibEntryMsg::Select);
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_all: 8,

                gtk::Image {
                    set_icon_name: Some(if self.is_error { "dialog-error-symbolic" } else { "text-x-generic-symbolic" }),
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,

                    gtk::Label {
                        set_label: &self.title,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        add_css_class: "heading",
                    },

                    gtk::Label {
                        set_label: &format!("[{}] {}", self.kind, self.key),
                        set_halign: gtk::Align::Start,
                        add_css_class: "caption",
                    }
                },

                gtk::Button {
                    set_icon_name: "user-trash-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Delete Entry"),

                    // Ensure the button handles its own focus so it doesn't confusingly trigger the row click
                    // (though GTK buttons usually stop propagation anyway).
                    set_focusable: true,
                    set_can_focus: true,

                    connect_clicked => BibEntryMsg::Delete,
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        init
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            BibEntryMsg::Delete => {
                let _ = sender.output(BibEntryOutput::Delete(self.key.clone()));
            }
            BibEntryMsg::Select => {
                let _ = sender.output(BibEntryOutput::Select(self.key.clone()));
            }
        }
    }
}
