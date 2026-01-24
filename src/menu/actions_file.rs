use crate::app::AppModel;
use crate::app::AppMsg;
use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib::clone;
use gtk4::prelude::*;
use relm4::ComponentSender;

pub fn init(root: &gtk::ApplicationWindow, sender: ComponentSender<AppModel>) {
    // --- OPEN ---
    let action_open = gio::SimpleAction::new("open", None);
    action_open.connect_activate(clone!(@strong sender => move |_, _| {
        sender.input(AppMsg::TriggerOpen);
    }));
    root.add_action(&action_open);

    // --- SAVE ---
    let action_save = gio::SimpleAction::new("save", None);
    action_save.connect_activate(clone!(@strong sender => move |_, _| {
        sender.input(AppMsg::TriggerSave);
    }));
    root.add_action(&action_save);

    // --- SAVE AS ---
    let action_save_as = gio::SimpleAction::new("save_as", None);
    action_save_as.connect_activate(clone!(@strong sender => move |_, _| {
        sender.input(AppMsg::TriggerSaveAs);
    }));
    root.add_action(&action_save_as);

    // --- QUIT ---
    let action_quit = gio::SimpleAction::new("quit", None);
    action_quit.connect_activate(move |_, _| {
        let app = relm4::main_application();
        app.quit();
    });
    root.add_action(&action_quit);
}
