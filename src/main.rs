// src/main.rs
mod api;
mod app;
mod core;
mod menu;
mod ui;

use app::AppModel;
use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("org.mkbib.rs");
    app.run::<AppModel>(());
}
