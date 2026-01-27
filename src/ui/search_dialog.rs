use crate::api::SearchResultItem;
use gtk4::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

// -----------------------------------------------------------------------------
// ROW ITEM (The Individual Result)
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct SearchResultRow {
    pub data: SearchResultItem,
}

#[derive(Debug)]
pub enum SearchResultRowMsg {
    Import,
}

#[relm4::factory(pub)]
impl FactoryComponent for SearchResultRow {
    type Init = SearchResultItem;
    type Input = SearchResultRowMsg;
    type Output = String; // Sends the DOI back to the parent
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::ListBoxRow {
            set_activatable: true,
            set_selectable: false, // Don't hold "selection" state, just click

            // 1. Allow clicking the whole row
            connect_activate => SearchResultRowMsg::Import,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_all: 8,
                set_spacing: 12,

                // Text Info
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true, // Push button to the right
                    set_spacing: 4,

                    gtk::Label {
                        set_label: &self.data.title,
                        set_halign: gtk::Align::Start,
                        add_css_class: "heading",
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                    },
                    gtk::Label {
                        set_label: &format!("{} ({})", self.data.author, self.data.year),
                        set_halign: gtk::Align::Start,
                        add_css_class: "caption",
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                    }
                },

                // 2. Explicit Import Button (Failsafe)
                gtk::Button {
                    set_label: "Import",
                    set_valign: gtk::Align::Center,
                    add_css_class: "suggested-action",
                    connect_clicked => SearchResultRowMsg::Import,
                }
            }
        }
    }

    fn init_model(data: Self::Init, _idx: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { data }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            SearchResultRowMsg::Import => {
                // Send the DOI up to the Dialog
                sender.output(self.data.doi.clone());
            }
        }
    }
}

// -----------------------------------------------------------------------------
// DIALOG MODEL (The Window)
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct SearchDialogModel {
    pub is_visible: bool,
    pub results: FactoryVecDeque<SearchResultRow>,
}

#[derive(Debug)]
pub enum SearchDialogMsg {
    ShowResults(Vec<SearchResultItem>),
    SelectDoi(String), // Internal message received from Row
    Close,
}

#[derive(Debug)]
pub enum SearchDialogOutput {
    FetchDoi(String), // Output to the Main App
}

#[relm4::component(pub)]
impl Component for SearchDialogModel {
    type Init = ();
    type Input = SearchDialogMsg;
    type Output = SearchDialogOutput;
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_modal: true,
            set_title: Some("Select a Paper"),
            set_default_width: 650,
            set_default_height: 500,
            set_hide_on_close: true, // Don't destroy, just hide
            #[watch] set_visible: model.is_visible,

            connect_close_request[sender] => move |_| {
                sender.input(SearchDialogMsg::Close);
                gtk::glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[local_ref]
                    results_list -> gtk::ListBox {
                        // 3. Selection Mode None + Single Click Activate
                        // This ensures clicking doesn't just "highlight" the row
                        set_selection_mode: gtk::SelectionMode::None,
                        set_activate_on_single_click: true,
                        add_css_class: "boxed-list",
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::End,
                    set_margin_all: 12,

                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => SearchDialogMsg::Close,
                    }
                }
            }
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Initialize the list of results
        let results = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            // Forward the Row's output (DOI String) to the Dialog's Input
            .forward(sender.input_sender(), |doi| SearchDialogMsg::SelectDoi(doi));

        let model = SearchDialogModel {
            is_visible: false,
            results,
        };

        // Bind the factory widget to the local_ref variable name used in view!
        let results_list = model.results.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SearchDialogMsg::ShowResults(items) => {
                // Populate the list
                self.results.guard().clear();
                for item in items {
                    self.results.guard().push_back(item);
                }
                self.is_visible = true;
            }
            SearchDialogMsg::SelectDoi(doi) => {
                // 1. Send the DOI to the main App
                let _ = sender.output(SearchDialogOutput::FetchDoi(doi));
                // 2. Hide the dialog
                self.is_visible = false;
            }
            SearchDialogMsg::Close => {
                self.is_visible = false;
            }
        }
    }
}
