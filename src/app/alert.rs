// src/ui/alert.rs
use gtk4::glib;
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct AlertModel {
    pub hidden: bool,
    pub message: String,
    // --- NEW FIELDS ---
    pub header: String,
    pub is_error: bool,
}

#[derive(Debug)]
pub enum AlertMsg {
    Show(String),     // Defaults to Error (Red)
    ShowInfo(String), // New: Info (Blue/Standard)
    Close,
}

#[relm4::component(pub)]
impl SimpleComponent for AlertModel {
    type Init = ();
    type Input = AlertMsg;
    type Output = ();

    view! {
        dialog = gtk::Window {
            set_modal: true,
            // Dynamic Title
            #[watch]
            set_title: Some(if model.is_error { "Error" } else { "Information" }),
            set_default_width: 350,
            set_resizable: false,

            connect_close_request[sender] => move |_| {
                sender.input(AlertMsg::Close);
                glib::Propagation::Stop
            },

            #[watch]
            set_visible: !model.hidden,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 24,
                set_spacing: 16,

                // Header Area
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Image {
                        // Dynamic Icon
                        #[watch]
                        set_icon_name: Some(if model.is_error { "dialog-error-symbolic" } else { "dialog-information-symbolic" }),
                        set_pixel_size: 32,
                        // Dynamic Class (error = red, success/info = standard)
                        #[watch]
                        set_css_classes: if model.is_error { &["error"] } else { &["success"] },
                    },

                    gtk::Label {
                        // Dynamic Header Text
                        #[watch]
                        set_label: &model.header,
                        add_css_class: "title-3",
                    },
                },

                // Message Body
                gtk::Label {
                    #[watch]
                    set_label: &model.message,
                    set_wrap: true,
                    set_max_width_chars: 40,
                    set_halign: gtk::Align::Start,
                },

                // Footer
                gtk::Box {
                    set_halign: gtk::Align::End,
                    gtk::Button {
                        set_label: "OK",
                        add_css_class: "suggested-action",
                        connect_clicked[sender] => move |_| {
                            sender.input(AlertMsg::Close);
                        }
                    }
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AlertModel {
            hidden: true,
            message: String::new(),
            header: String::new(),
            is_error: true,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AlertMsg::Show(text) => {
                self.message = text;
                self.header = "An error occurred".to_string();
                self.is_error = true;
                self.hidden = false;
            }
            // New Handler for Success/Info
            AlertMsg::ShowInfo(text) => {
                self.message = text;
                self.header = "Information".to_string();
                self.is_error = false;
                self.hidden = false;
            }
            AlertMsg::Close => {
                self.hidden = true;
            }
        }
    }
}
