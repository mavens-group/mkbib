use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub struct DetailsDialogModel {
    pub is_active: bool,
    pub original_key: String,
    pub content: String,
    // Fix: Add a flag to track if the update is from the user typing
    pub is_internal_update: bool,
}

#[derive(Debug)]
pub enum DetailsDialogMsg {
    Open(String, String), // (Key, BibTeX Content)
    UpdateContent(String),
    Save,
    Close,
}

#[derive(Debug)]
pub enum DetailsDialogOutput {
    Saved(String, String), // (Original Key, New Content)
}

#[relm4::component(pub)]
impl Component for DetailsDialogModel {
    type Init = ();
    type Input = DetailsDialogMsg;
    type Output = DetailsDialogOutput;
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_modal: true,
            set_default_width: 650,
            set_default_height: 500,
            set_title: Some("Edit Entry"),
            set_hide_on_close: true,
            #[watch] set_visible: model.is_active,

            connect_close_request[sender] => move |_| {
                sender.input(DetailsDialogMsg::Close);
                gtk::glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 12,
                set_spacing: 12,

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Automatic,

                    #[name = "details_view"]
                    gtk::TextView {
                        set_monospace: true,
                        set_editable: true,
                        set_left_margin: 8,
                        set_right_margin: 8,
                        set_top_margin: 8,
                        set_bottom_margin: 8,
                        set_wrap_mode: gtk::WrapMode::WordChar,

                        #[watch]
                        set_visible: {
                            // FIX: Only write to the buffer if the update came from outside (Msg::Open)
                            // If it's an internal update (user typing), the buffer is already correct.
                            if !model.is_internal_update {
                                details_view.buffer().set_text(&model.content);
                            }
                            true
                        }
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::End,
                    set_spacing: 12,

                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => DetailsDialogMsg::Close,
                    },
                    gtk::Button {
                        set_label: "Save Changes",
                        add_css_class: "suggested-action",
                        connect_clicked => DetailsDialogMsg::Save,
                    }
                }
            }
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DetailsDialogModel {
            is_active: false,
            original_key: String::new(),
            content: String::new(),
            is_internal_update: true, // Default to true to prevent accidental overwrites on init
        };
        let widgets = view_output!();

        // Connect Buffer Change Signal
        let buffer = widgets.details_view.buffer();
        buffer.connect_changed(move |buff| {
            let text = buff
                .text(&buff.start_iter(), &buff.end_iter(), true)
                .to_string();
            sender.input(DetailsDialogMsg::UpdateContent(text));
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DetailsDialogMsg::Open(key, text) => {
                self.original_key = key;
                self.content = text;
                self.is_active = true;
                // Important: This is an EXTERNAL update. We want the View to reflect this.
                self.is_internal_update = false;
            }
            DetailsDialogMsg::UpdateContent(text) => {
                self.content = text;
                // Important: This is an INTERNAL update. Do NOT push back to View.
                self.is_internal_update = true;
            }
            DetailsDialogMsg::Save => {
                let _ = sender.output(DetailsDialogOutput::Saved(
                    self.original_key.clone(),
                    self.content.clone(),
                ));
                self.is_active = false;
            }
            DetailsDialogMsg::Close => {
                self.is_active = false;
            }
        }
    }
}
