// src/menu/actions_edit.rs
use crate::app::{AppModel, AppMsg};
use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib::clone;
use gtk4::prelude::*;
use relm4::ComponentSender;

pub fn init(root: &gtk::ApplicationWindow, sender: ComponentSender<AppModel>) {
    let group = gio::SimpleActionGroup::new();

    // Action: preferences
    let action_prefs = gio::SimpleAction::new("preferences", None);
    action_prefs.connect_activate(clone!(@strong sender => move |_, _| {
        sender.input(AppMsg::ShowPreferences);
    }));
    group.add_action(&action_prefs);

    // Action: regenerate_keys
    let action_regen = gio::SimpleAction::new("regenerate_keys", None);
    action_regen.connect_activate(clone!(@strong sender => move |_, _| {
        sender.input(AppMsg::RegenerateAllKeys);
    }));
    group.add_action(&action_regen);

    // --- Action: scan_duplicates ---
    let action_scan = gio::SimpleAction::new("scan_duplicates", None);
    action_scan.connect_activate(clone!(@strong sender => move |_, _| {
        sender.input(AppMsg::ScanDuplicates);
    }));
    group.add_action(&action_scan);
    // CRITICAL FIX: Use "edit" group to avoid overwriting "win" group from File actions
    root.insert_action_group("edit", Some(&group));
}
