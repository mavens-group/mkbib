use gtk4::glib;
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedDecision {
    Save,
    Discard,
    Cancel,
}

pub struct UnsavedDialogModel {
    visible: bool,
}

#[derive(Debug)]
pub enum UnsavedDialogMsg {
    Show,
    Choose(UnsavedDecision),
}

#[relm4::component(pub)]
impl SimpleComponent for UnsavedDialogModel {
    type Init = ();
    type Input = UnsavedDialogMsg;
    type Output = UnsavedDecision;

    view! {
        dialog = gtk::Window {
            set_title: Some("Unsaved Changes"),
            set_modal: true,
            set_resizable: false,
            set_default_width: 440,
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(UnsavedDialogMsg::Choose(UnsavedDecision::Cancel));
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 18,
                set_margin_all: 24,

                gtk::Label {
                    set_label: "Save changes before continuing?",
                    add_css_class: "title-3",
                    set_halign: gtk::Align::Start,
                },
                gtk::Label {
                    set_label: "Unsaved changes will be permanently lost if you discard them.",
                    set_wrap: true,
                    set_halign: gtk::Align::Start,
                },
                gtk::Box {
                    set_spacing: 8,
                    set_halign: gtk::Align::End,
                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => UnsavedDialogMsg::Choose(UnsavedDecision::Cancel),
                    },
                    gtk::Button {
                        set_label: "Discard",
                        add_css_class: "destructive-action",
                        connect_clicked => UnsavedDialogMsg::Choose(UnsavedDecision::Discard),
                    },
                    gtk::Button {
                        set_label: "Save",
                        add_css_class: "suggested-action",
                        connect_clicked => UnsavedDialogMsg::Choose(UnsavedDecision::Save),
                    },
                }
            }
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self { visible: false };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            UnsavedDialogMsg::Show => self.visible = true,
            UnsavedDialogMsg::Choose(decision) => {
                self.visible = false;
                let _ = sender.output(decision);
            }
        }
    }
}
