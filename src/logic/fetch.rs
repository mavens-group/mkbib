// src/logic/fetch.rs
//
use crate::api;
use crate::app::alert::AlertMsg;
use crate::app::{AppModel, AppMsg};
use crate::ui::search_dialog::SearchDialogMsg;
use crate::ui::sidebar::SidebarMsg; // Needed for updates
use relm4::{ComponentController, ComponentSender}; // Needed for .emit()

pub fn handle_fetch_doi(model: &mut AppModel, sender: ComponentSender<AppModel>, doi: String) {
  let doi = doi.trim().to_string();
  if doi.is_empty() {
    return;
  }

  // Talk to the Sidebar Component
  model.sidebar.emit(SidebarMsg::SetLoading(true));
  model
    .sidebar
    .emit(SidebarMsg::SetStatus(format!("Fetching DOI: {}...", doi)));

  let input = sender.input_sender().clone();
  sender.command(move |_out, _shutdown| async move {
    let result = match api::fetch_doi(&doi).await {
      Ok(bib) => AppMsg::FetchSuccess(bib),
      Err(e) => AppMsg::FetchError(e.to_string()),
    };
    input.send(result).expect("Failed to send async result");
  });
}

pub fn handle_fetch_search(model: &mut AppModel, sender: ComponentSender<AppModel>, query: String) {
  let query = query.trim().to_string();
  if query.is_empty() {
    return;
  }

  model.sidebar.emit(SidebarMsg::SetLoading(true));
  model.sidebar.emit(SidebarMsg::SetStatus(format!(
    "Searching Crossref: {}...",
    query
  )));

  let input = sender.input_sender().clone();
  sender.command(move |_out, _shutdown| async move {
    match api::search_crossref_suggestions(&query).await {
      Ok(items) => input.send(AppMsg::SearchResultsLoaded(items)).unwrap(),
      Err(e) => input.send(AppMsg::FetchError(e.to_string())).unwrap(),
    }
  });
}

pub fn handle_search_results(model: &mut AppModel, items: Vec<crate::api::SearchResultItem>) {
  model.sidebar.emit(SidebarMsg::SetLoading(false));
  if items.is_empty() {
    model
      .sidebar
      .emit(SidebarMsg::SetStatus("No results found.".to_string()));
    model
      .alert
      .emit(AlertMsg::ShowInfo("No results found on Crossref.".into()));
  } else {
    model.sidebar.emit(SidebarMsg::SetStatus(
      "Select an item to import.".to_string(),
    ));
    model
      .search_dialog
      .emit(SearchDialogMsg::ShowResults(items));
  }
}

pub fn handle_success(
  model: &mut AppModel,
  bib: biblatex::Bibliography,
  sender: ComponentSender<AppModel>,
) {
  model.sidebar.emit(SidebarMsg::SetLoading(false));
  let mut count = 0;
  for entry in bib.iter() {
    sender.input(AppMsg::AddBiblatexEntry(entry.clone()));
    count += 1;
  }
  if count > 0 {
    model
      .sidebar
      .emit(SidebarMsg::SetStatus(format!("Imported {} entry.", count)));
  } else {
    model
      .sidebar
      .emit(SidebarMsg::SetStatus("DOI found, but empty.".to_string()));
  }
}

pub fn handle_error(model: &mut AppModel, err: String) {
  model.sidebar.emit(SidebarMsg::SetLoading(false));
  model
    .sidebar
    .emit(SidebarMsg::SetStatus("Error occurred.".to_string()));
  model.alert.emit(AlertMsg::Show(err));
}
