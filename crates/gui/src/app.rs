//! Application setup and lifecycle management

use crate::window::MainWindow;
use crate::APP_ID;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct NixosToolkitApp;

    #[glib::object_subclass]
    impl ObjectSubclass for NixosToolkitApp {
        const NAME: &'static str = "NixosToolkitApp";
        type Type = super::NixosToolkitApp;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for NixosToolkitApp {}

    impl ApplicationImpl for NixosToolkitApp {
        fn activate(&self) {
            let app = self.obj();
            let window = MainWindow::new(&app);
            window.present();
        }

        fn startup(&self) {
            self.parent_startup();

            // Apply saved theme before any windows are created (prevents flash)
            let prefs = crate::preferences::UserPreferences::load();
            adw::StyleManager::default().set_color_scheme(prefs.color_scheme.to_adw_color_scheme());

            // Set up application actions
            let app = self.obj();
            app.setup_actions();
        }
    }

    impl GtkApplicationImpl for NixosToolkitApp {}
    impl AdwApplicationImpl for NixosToolkitApp {}
}

glib::wrapper! {
    pub struct NixosToolkitApp(ObjectSubclass<imp::NixosToolkitApp>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl NixosToolkitApp {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", APP_ID)
            .property("flags", gio::ApplicationFlags::FLAGS_NONE)
            .build()
    }

    fn setup_actions(&self) {
        // Quit action
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(|app: &Self, _, _| {
                app.quit();
            })
            .build();

        // About action
        let about_action = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about();
            })
            .build();

        self.add_action_entries([quit_action, about_action]);

        // Set accelerators
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("win.refresh", &["<Control>r", "F5"]);
    }

    fn show_about(&self) {
        let window = self.active_window();

        let about = adw::AboutWindow::builder()
            .application_name("NixOS Toolkit")
            .application_icon("preferences-system")
            .developer_name("NixOS Toolkit Contributors")
            .version(env!("CARGO_PKG_VERSION"))
            .website("https://github.com/goshitsarch-eng/Gosh-NixOS-Manager")
            .issue_url("https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/issues")
            .license_type(gtk::License::Gpl30)
            .comments("A declarative NixOS system management tool")
            .modal(true)
            .build();

        about.add_acknowledgement_section(
            Some("Built with"),
            &["GTK4", "libadwaita", "Rust", "Nix"],
        );

        if let Some(win) = window {
            about.set_transient_for(Some(&win));
        }
        about.present();
    }
}

impl Default for NixosToolkitApp {
    fn default() -> Self {
        Self::new()
    }
}
