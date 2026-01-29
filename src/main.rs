// src/main.rs
mod api;
mod app;
mod core;
mod logic; // NEW
mod menu;
mod ui;

use app::AppModel;
use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("org.mavensgroup.mkbib");
    app.run::<AppModel>(());
}
