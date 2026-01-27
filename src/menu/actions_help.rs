// src/menu/actions_help.rs
//
use crate::app::AppModel;
use gtk4::gdk_pixbuf::PixbufLoader;
use gtk4::prelude::*;
use gtk4::{gio, glib};
use relm4::ComponentSender;

pub fn init(root: &gtk4::ApplicationWindow, _sender: ComponentSender<AppModel>) {
    // --- ABOUT ---
    let action_about = gio::SimpleAction::new("about", None);

    action_about.connect_activate(glib::clone!(@weak root => move |_, _| {

        let mut builder = gtk4::AboutDialog::builder()
            .transient_for(&root)
            .modal(true)
            .program_name("MkBib")
            .version(env!("CARGO_PKG_VERSION"))
            .comments("A modern bibliography manager")
            .website("https://github.com/mavens-group/mkbib")
            .authors(vec!["The Mavens Group".to_string()])
            .license_type(gtk4::License::Gpl30);

        // --- HYBRID ICON LOGIC ---
        // 1. Check if the system has the icon installed (e.g. via RPM/Deb)
        // We use the full App ID "org.mkbib.rs" to match the .desktop file convention.
        let display = gtk4::gdk::Display::default().expect("No default display");
        let icon_theme = gtk4::IconTheme::for_display(&display);

        if icon_theme.has_icon("org.mavensgroup.mkbib") {
            // CASE A: System icon found. Use it!
            // This allows the icon to be themed by the OS if applicable.
            builder = builder.logo_icon_name("org.mavensgroup.mkbib");
        } else {
            // CASE B: Icon missing from system. Use embedded fallback.
            // This runs during 'cargo run', AppImage, or portable builds.

            // NOTE: Ensure your file in assets/ is named "org.mkbib.rs.svg"
            let svg_bytes = include_bytes!("../../assets/org.mavensgroup.mkbib.svg");
            let loader = PixbufLoader::with_type("svg").expect("Failed to init SVG loader");

            // Set size to 128x128 for a crisp, large icon in the dialog
            loader.set_size(128, 128);

            // Write data and close
            let _ = loader.write(svg_bytes);
            let _ = loader.close();

            // Create texture
            let texture = loader.pixbuf().map(|p| gtk4::gdk::Texture::for_pixbuf(&p));

            if let Some(tex) = &texture {
                builder = builder.logo(tex);
            }
        }

        builder.build().present();
    }));

    root.add_action(&action_about);
}
