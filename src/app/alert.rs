use gtk4::glib;
use gtk4::prelude::*;
use relm4::prelude::*; // <--- Import glib

pub struct AlertModel {
    pub hidden: bool,
    pub message: String,
}

#[derive(Debug)]
pub enum AlertMsg {
    Show(String),
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
            set_title: Some("Error"),
            set_default_width: 350,
            set_resizable: false,

            // Fix: Return Propagation::Stop to prevent the window from being destroyed
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

                // Header
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Image {
                        set_icon_name: Some("dialog-error-symbolic"),
                        set_pixel_size: 32,
                        add_css_class: "error",
                    },

                    gtk::Label {
                        set_label: "An error occurred",
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

                // Footer / Button
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
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AlertMsg::Show(text) => {
                self.message = text;
                self.hidden = false;
            }
            AlertMsg::Close => {
                self.hidden = true;
            }
        }
    }
}
