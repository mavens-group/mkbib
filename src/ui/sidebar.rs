use gtk4::prelude::*;
use relm4::prelude::*;

// 1. State
pub struct SidebarModel {
    pub doi_input: String,
    pub search_input: String,
    pub manual_input: String,
    pub is_loading: bool,
    pub status_msg: String,
}

// 2. Messages
#[derive(Debug)]
pub enum SidebarMsg {
    TriggerFetchDoi(String),
    TriggerSearch(String),
    TriggerParseManual(String),
    TriggerClear,
    SetLoading(bool),
    SetStatus(String),
}

// 3. Output
#[derive(Debug)]
pub enum SidebarOutput {
    FetchDoi(String),
    SearchCrossref(String),
    ParseManual(String),
    ClearAll,
}

#[relm4::component(pub)]
impl SimpleComponent for SidebarModel {
    type Init = ();
    type Input = SidebarMsg;
    type Output = SidebarOutput;

    view! {
        gtk::Box {
            set_width_request: 320,
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 12,
            set_spacing: 12,

            gtk::Label {
                set_label: "Library Sources",
                set_css_classes: &["title-3"],
                set_halign: gtk::Align::Start,
            },

            // --- DOI Section ---
            gtk::Frame {
                set_label: Some("Import via DOI"),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_all: 8,

                    // FIX: Use #[name] attribute to create the variable 'doi_entry'
                    #[name = "doi_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("10.1038/..."),

                        // We watch model.doi_input so "Clear" works,
                        // but we rely on 'connect_activate' to read the value, avoiding loops.
                        #[watch]
                        set_text: &model.doi_input,

                        connect_activate[sender] => move |entry| {
                            sender.input(SidebarMsg::TriggerFetchDoi(entry.text().into()));
                        },
                    },

                    gtk::Button {
                        set_label: "Fetch BibTeX",
                        // Capture 'doi_entry' variable here
                        connect_clicked[sender, doi_entry] => move |_| {
                            sender.input(SidebarMsg::TriggerFetchDoi(doi_entry.text().into()));
                        }
                    }
                }
            },

            // --- Search Section ---
            gtk::Frame {
                set_label: Some("Web Search (Crossref)"),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_all: 8,

                    // FIX: Use #[name]
                    #[name = "search_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("Title, Author..."),

                        #[watch]
                        set_text: &model.search_input,

                        connect_activate[sender] => move |entry| {
                            sender.input(SidebarMsg::TriggerSearch(entry.text().into()));
                        },
                    },

                    gtk::Button {
                        set_label: "Search & Import",
                        connect_clicked[sender, search_entry] => move |_| {
                            sender.input(SidebarMsg::TriggerSearch(search_entry.text().into()));
                        }
                    }
                }
            },

            // --- Manual Entry Section ---
            gtk::Frame {
                set_label: Some("Manual Entry"),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_all: 8,

                    gtk::ScrolledWindow {
                        set_height_request: 120,
                        set_has_frame: true,

                        // FIX: Use #[name]
                        #[name = "manual_view"]
                        gtk::TextView {
                            set_wrap_mode: gtk::WrapMode::WordChar,
                            set_top_margin: 8,
                            set_bottom_margin: 8,
                            set_left_margin: 8,
                            set_right_margin: 8,

                            #[watch]
                            set_buffer: Some(&gtk::TextBuffer::builder().text(&model.manual_input).build()),
                        }
                    },

                    gtk::Button {
                        set_label: "Add Entry",
                        connect_clicked[sender, manual_view] => move |_| {
                            let buffer = manual_view.buffer();
                            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                            sender.input(SidebarMsg::TriggerParseManual(text.into()));
                        }
                    }
                }
            },

            // --- Status Section ---
            gtk::Spinner {
                #[watch]
                set_spinning: model.is_loading,
            },

            gtk::Separator { set_margin_top: 10, set_margin_bottom: 10 },

            gtk::Button {
                set_label: "Clear All Entries",
                set_css_classes: &["destructive-action"],
                connect_clicked[sender] => move |_| sender.input(SidebarMsg::TriggerClear),
            },

            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &model.status_msg,
                set_wrap: true,
                set_margin_top: 10,
                set_css_classes: &["caption"],
            }
        }
    }

    fn init(
        _: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SidebarModel {
            doi_input: String::new(),
            search_input: String::new(),
            manual_input: String::new(),
            is_loading: false,
            status_msg: "Ready.".to_string(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SidebarMsg::SetLoading(b) => self.is_loading = b,
            SidebarMsg::SetStatus(s) => self.status_msg = s,

            SidebarMsg::TriggerFetchDoi(text) => {
                if !text.is_empty() {
                    self.is_loading = true;
                    self.status_msg = "Requesting...".into();
                    // We update the model so "Clear" works later, but we DON'T rely on binding for the data
                    self.doi_input = text.clone();
                    sender.output(SidebarOutput::FetchDoi(text)).unwrap();
                }
            }
            SidebarMsg::TriggerSearch(text) => {
                if !text.is_empty() {
                    self.is_loading = true;
                    self.status_msg = "Searching...".into();
                    self.search_input = text.clone();
                    sender.output(SidebarOutput::SearchCrossref(text)).unwrap();
                }
            }
            SidebarMsg::TriggerParseManual(text) => {
                if !text.is_empty() {
                    self.manual_input = text.clone();
                    sender.output(SidebarOutput::ParseManual(text)).unwrap();
                }
            }
            SidebarMsg::TriggerClear => {
                self.doi_input.clear();
                self.search_input.clear();
                self.manual_input.clear();
                self.status_msg = "Library cleared.".into();
                sender.output(SidebarOutput::ClearAll).unwrap();
            }
        }
    }
}
