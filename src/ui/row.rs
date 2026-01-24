// src/ui/row.rs
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone)]
pub struct BibEntry {
    pub key: String,
    pub title: String,
    pub kind: String,
    pub is_error: bool,
}

#[derive(Debug)]
pub enum BibEntryMsg {
    Delete,
}

#[relm4::factory(pub)]
impl FactoryComponent for BibEntry {
    type Init = BibEntry;
    type Input = BibEntryMsg;
    type Output = String; // Outputs the key to be deleted
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::ListBoxRow {
            // FIX: Use set_class_active instead of add_css_class with conditional strings
            set_class_active: ("error", self.is_error),

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_all: 8,

                // Icon
                gtk::Image {
                    set_icon_name: Some(if self.is_error { "dialog-error-symbolic" } else { "text-x-generic-symbolic" }),
                },

                // Main Info
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

                // Delete Button
                gtk::Button {
                    set_icon_name: "user-trash-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Delete Entry"),
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
                let _ = sender.output(self.key.clone());
            }
        }
    }
}
