// src/menu/actions_help.rs
//
use crate::app::AppModel;
use gtk4::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use libadwaita::prelude::AdwDialogExt;
use relm4::ComponentSender;

pub fn init(root: &gtk4::ApplicationWindow, _sender: ComponentSender<AppModel>) {
    // --- ABOUT ---
    let action_about = gio::SimpleAction::new("about", None);

    action_about.connect_activate(glib::clone!(@weak root => move |_, _| {

        // --- HYBRID ICON LOGIC ---
        // AdwAboutDialog takes an *icon name* (application-icon), not a paintable.
        // 1. If the system has the icon installed (e.g. via RPM/Deb), it resolves
        //    directly from the theme.
        // 2. Otherwise (cargo run / AppImage / portable) point the icon theme at
        //    the bundled assets dir so the same name still resolves.
        let icon_name = "org.mavensgroup.mkbib";
        let display = gtk4::gdk::Display::default().expect("No default display");
        let icon_theme = gtk4::IconTheme::for_display(&display);

        if !icon_theme.has_icon(icon_name) {
            icon_theme.add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"));
        }

        let dialog = adw::AboutDialog::builder()
            .application_name("MkBib")
            .application_icon(icon_name)
            .developer_name("The Mavens Group")
            .version(env!("CARGO_PKG_VERSION"))
            .comments("A modern bibliography manager")
            .website("https://github.com/mavens-group/mkbib")
            .issue_url("https://github.com/mavens-group/mkbib/issues")
            .developers(vec!["The Mavens Group".to_string()])
            .license_type(gtk4::License::Gpl30)
            .build();

        dialog.present(&root);
    }));

    root.add_action(&action_about);
}
