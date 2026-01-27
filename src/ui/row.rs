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

          // FIX 1: Use native activation instead of a custom GestureClick controller.
          // GTK automatically prevents this from firing if a child button is clicked.
          set_activatable: true,
          connect_activate[sender] => move |_| {
              sender.input(BibEntryMsg::Select);
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

                  // FIX 2: Ensure the button stops event propagation so the row doesn't 'feel' it.
                  // (Usually not strictly necessary if using connect_activate, but good practice).
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
