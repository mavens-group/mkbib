// src/ui/duplicate_dialog.rs
use crate::logic::deduplicator::{DuplicateGroup, EntryInfo};
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct DuplicateDialogModel {
    pub groups: Vec<DuplicateGroup>,
    pub group_index: usize,
    pub candidate_index: usize,
    pub is_active: bool,
}

#[derive(Debug)]
pub enum DuplicateDialogMsg {
    LoadGroups(Vec<DuplicateGroup>),
    Resolve(Resolution),
    Close,
}

#[derive(Debug)]
pub enum Resolution {
    KeepOriginal,
    KeepCandidate,
    Ignore,
}

#[derive(Debug)]
pub enum DuplicateDialogOutput {
    DeleteEntry(String),
}

#[relm4::component(pub)]
impl Component for DuplicateDialogModel {
    type Init = ();
    type Input = DuplicateDialogMsg;
    type Output = DuplicateDialogOutput;
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_modal: true,
            set_default_width: 1000, // Wide enough for side-by-side
            set_default_height: 600,
            set_title: Some("Resolve Duplicates"),
            set_hide_on_close: true,
            #[watch] set_visible: model.is_active,

            connect_close_request[sender] => move |_| {
                sender.input(DuplicateDialogMsg::Close);
                gtk::glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 20,
                set_spacing: 12,

                // --- HEADER ---
                gtk::Label {
                    #[watch]
                    set_label: &model.status_label(),
                    set_css_classes: &["title-3"],
                },
                gtk::Label {
                    #[watch]
                    set_label: &format!("Match Confidence: {}", model.current_score_display()),
                    set_css_classes: &["title-3"],
                },
                gtk::Separator {},

                // --- MAIN CONTENT ROW ---
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_vexpand: true,
                    set_spacing: 0, // We handle spacing manually with margins/width_request

                    // === LEFT COLUMN (45%) ===
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true, // Expand to take available space
                        set_width_request: 100, // Minimum width prevents collapse
                        set_spacing: 12,
                        set_margin_end: 6, // Half of the gap

                        // LEFT FRAME
                        gtk::Frame {
                            set_label: Some("Original (Existing)"),
                            set_vexpand: true,
                            // CSS class to potentially add borders via style.css
                            set_css_classes: &["view"],

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 12,
                                set_spacing: 8,

                                // Title Label - The Troublemaker
                                gtk::Label {
                                    #[watch] set_label: &model.original_info().title,
                                    set_css_classes: &["heading"],
                                    set_halign: gtk::Align::Start,
                                    set_xalign: 0.0,

                                    // --- CRITICAL FIX START ---
                                    set_wrap: true,
                                    set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                    set_hexpand: true, // Fill the box
                                    set_width_chars: 1, // "I am happy being tiny"
                                    set_max_width_chars: 1, // "Don't ask for size"
                                    // --- CRITICAL FIX END ---
                                },

                                // Key
                                gtk::Label {
                                    #[watch] set_label: &model.original_info().key,
                                    set_css_classes: &["monospaced", "caption"],
                                    set_halign: gtk::Align::Start,
                                    set_selectable: true,
                                    set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                                    set_hexpand: true,
                                    set_width_chars: 1,
                                    set_max_width_chars: 1,
                                },

                                // Authors
                                gtk::Label {
                                    #[watch] set_label: &model.original_info().author,
                                    set_css_classes: &["body"],
                                    set_halign: gtk::Align::Start,
                                    set_xalign: 0.0,
                                    set_wrap: true,
                                    set_hexpand: true,
                                    set_width_chars: 1,
                                    set_max_width_chars: 1,
                                },

                                // Year
                                gtk::Label {
                                    #[watch] set_label: &model.original_info().year,
                                    set_css_classes: &["monospaced"],
                                    set_halign: gtk::Align::Start,
                                },
                            }
                        },

                        // LEFT BUTTON
                        gtk::Button {
                            set_label: "Keep This",
                            set_css_classes: &["suggested-action"],
                            set_height_request: 60,
                            connect_clicked => DuplicateDialogMsg::Resolve(Resolution::KeepOriginal),
                        },
                    },

                    // === MIDDLE COLUMN (10%) ===
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: false,
                        set_valign: gtk::Align::End, // Bottom align
                        set_width_request: 100, // Fixed width gap

                        gtk::Button {
                            set_label: "Skip",
                            set_margin_start: 10,
                            set_margin_end: 10,
                            set_height_request: 60, // Match other buttons height
                            connect_clicked => DuplicateDialogMsg::Resolve(Resolution::Ignore),
                        },
                    },

                    // === RIGHT COLUMN (45%) ===
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true, // Expand to take available space
                        set_width_request: 100,
                        set_spacing: 12,
                        set_margin_start: 6, // Half of the gap

                        // RIGHT FRAME
                        gtk::Frame {
                            set_label: Some("Duplicate (Found)"),
                            set_vexpand: true,
                            set_css_classes: &["view"],

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 12,
                                set_spacing: 8,

                                // Title Label
                                gtk::Label {
                                    #[watch] set_label: &model.candidate_info().title,
                                    set_css_classes: &["heading"],
                                    set_halign: gtk::Align::Start,
                                    set_xalign: 0.0,

                                    // --- CRITICAL FIX START ---
                                    set_wrap: true,
                                    set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                    set_hexpand: true,
                                    set_width_chars: 1,
                                    set_max_width_chars: 1,
                                    // --- CRITICAL FIX END ---
                                },

                                // Key
                                gtk::Label {
                                    #[watch] set_label: &model.candidate_info().key,
                                    set_css_classes: &["monospaced", "caption"],
                                    set_halign: gtk::Align::Start,
                                    set_selectable: true,
                                    set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                                    set_hexpand: true,
                                    set_width_chars: 1,
                                    set_max_width_chars: 1,
                                },

                                // Authors
                                gtk::Label {
                                    #[watch] set_label: &model.candidate_info().author,
                                    set_css_classes: &["body"],
                                    set_halign: gtk::Align::Start,
                                    set_xalign: 0.0,
                                    set_wrap: true,
                                    set_hexpand: true,
                                    set_width_chars: 1,
                                    set_max_width_chars: 1,
                                },

                                // Year
                                gtk::Label {
                                    #[watch] set_label: &model.candidate_info().year,
                                    set_css_classes: &["monospaced"],
                                    set_halign: gtk::Align::Start,
                                },
                            }
                        },

                        // RIGHT BUTTON
                        gtk::Button {
                            set_label: "Keep This",
                            set_css_classes: &["suggested-action"],
                            set_height_request: 60,
                            connect_clicked => DuplicateDialogMsg::Resolve(Resolution::KeepCandidate),
                        },
                    },
                }
            }
        }
    }

    // ... (Init and Update functions remain the same)
    fn init(_: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DuplicateDialogModel {
            groups: vec![],
            group_index: 0,
            candidate_index: 0,
            is_active: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DuplicateDialogMsg::LoadGroups(groups) => {
                self.groups = groups;
                self.group_index = 0;
                self.candidate_index = 0;
                self.is_active = !self.groups.is_empty();
            }
            DuplicateDialogMsg::Resolve(resolution) => {
                if self.groups.is_empty() {
                    return;
                }
                let current_group = &self.groups[self.group_index];
                if self.candidate_index >= current_group.candidates.len() {
                    self.next_group();
                    return;
                }
                let candidate = &current_group.candidates[self.candidate_index];

                match resolution {
                    Resolution::KeepOriginal => {
                        let _ = sender.output(DuplicateDialogOutput::DeleteEntry(
                            candidate.info.key.clone(),
                        ));
                        self.next_candidate_or_group();
                    }
                    Resolution::KeepCandidate => {
                        let _ = sender.output(DuplicateDialogOutput::DeleteEntry(
                            current_group.original.key.clone(),
                        ));
                        self.next_group();
                    }
                    Resolution::Ignore => {
                        self.next_candidate_or_group();
                    }
                }
            }
            DuplicateDialogMsg::Close => {
                self.is_active = false;
            }
        }
    }
}

impl DuplicateDialogModel {
    fn next_candidate_or_group(&mut self) {
        let current_group_len = self.groups[self.group_index].candidates.len();
        if self.candidate_index + 1 < current_group_len {
            self.candidate_index += 1;
        } else {
            self.next_group();
        }
    }

    fn next_group(&mut self) {
        if self.group_index + 1 < self.groups.len() {
            self.group_index += 1;
            self.candidate_index = 0;
        } else {
            self.is_active = false;
            self.groups.clear();
        }
    }

    fn status_label(&self) -> String {
        if self.groups.is_empty() {
            return "Done".to_string();
        }
        let total_groups = self.groups.len();
        let current_group = &self.groups[self.group_index];
        let total_candidates = current_group.candidates.len();

        if total_candidates > 1 {
            format!(
                "Group {} of {} (Conflict {} of {})",
                self.group_index + 1,
                total_groups,
                self.candidate_index + 1,
                total_candidates
            )
        } else {
            format!("Conflict {} of {}", self.group_index + 1, total_groups)
        }
    }

    fn original_info(&self) -> EntryInfo {
        self.groups
            .get(self.group_index)
            .map(|g| g.original.clone())
            .unwrap_or_else(empty_info)
    }

    fn candidate_info(&self) -> EntryInfo {
        self.groups
            .get(self.group_index)
            .and_then(|g| g.candidates.get(self.candidate_index))
            .map(|c| c.info.clone())
            .unwrap_or_else(empty_info)
    }

    fn current_score_display(&self) -> String {
        self.groups
            .get(self.group_index)
            .and_then(|g| g.candidates.get(self.candidate_index))
            .map(|c| format!("{:.1}%", c.similarity * 100.0))
            .unwrap_or_else(|| "0%".to_string())
    }
}

fn empty_info() -> EntryInfo {
    EntryInfo {
        key: "---".into(),
        title: "---".into(),
        author: "---".into(),
        year: "---".into(),
    }
}
