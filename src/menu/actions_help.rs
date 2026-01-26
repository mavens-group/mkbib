// src/menu/actions_help.rs
use crate::app::AppModel;
use gtk4::gdk_pixbuf::PixbufLoader;
use gtk4::prelude::*;
use gtk4::{gio, glib};
use relm4::ComponentSender; // <--- This import was missing

pub fn init(root: &gtk4::ApplicationWindow, _sender: ComponentSender<AppModel>) {
    // --- ABOUT ---
    let action_about = gio::SimpleAction::new("about", None);

    action_about.connect_activate(glib::clone!(@weak root => move |_, _| {

        // --- LOGO LOADING LOGIC ---
        let svg_bytes = include_bytes!("../../assets/mkbib.svg");
        let loader = PixbufLoader::with_type("svg").expect("Failed to init SVG loader");

        // Set size to 128x128 for a crisp, large icon in the dialog
        loader.set_size(128, 128);

        let _ = loader.write(svg_bytes);
        let _ = loader.close();
        let texture = loader.pixbuf().map(|p| gtk4::gdk::Texture::for_pixbuf(&p));
        // --------------------------

        let mut builder = gtk4::AboutDialog::builder()
            .transient_for(&root)
            .modal(true)
            .program_name("MkBib")
            .version("0.1.0")
            .comments("A modern bibliography manager for")
            .website("https://github.com/mavens-group/mkbib")
            .authors(vec!["The Mavens Group".to_string()])
            .license_type(gtk4::License::Gpl30); // GPLv3 License

        // Inject logo if loaded successfully
        if let Some(tex) = &texture {
            builder = builder.logo(tex);
        }

        builder.build().present();
    }));

    root.add_action(&action_about);
}
