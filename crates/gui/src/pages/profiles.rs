//! Profile selection page

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::actions::{default_profiles, ProfileDef};
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ProfilesPage {
        pub selected_profile: RefCell<Option<String>>,
        pub profile_rows: RefCell<Vec<(String, adw::ActionRow)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfilesPage {
        const NAME: &'static str = "NixosToolkitProfilesPage";
        type Type = super::ProfilesPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for ProfilesPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for ProfilesPage {}
    impl BoxImpl for ProfilesPage {}
}

glib::wrapper! {
    pub struct ProfilesPage(ObjectSubclass<imp::ProfilesPage>)
        @extends gtk::Box, gtk::Widget;
}

impl ProfilesPage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("orientation", gtk::Orientation::Vertical)
            .property("spacing", 0)
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Wrap everything in a scrolled window
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_start(24)
            .margin_end(24)
            .margin_top(24)
            .margin_bottom(24)
            .build();

        // Title
        let title = gtk::Label::builder()
            .label("Desktop Environment")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Select a desktop environment profile. You can only have one active at a time.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Profiles group
        let profiles_group = adw::PreferencesGroup::builder()
            .title("Available Profiles")
            .build();

        // Add profile rows
        let profiles = default_profiles();
        let mut profile_rows = Vec::new();

        for profile in &profiles {
            let row = self.create_profile_row(profile);
            profiles_group.add(&row);
            profile_rows.push((profile.id.clone(), row));
        }

        *imp.profile_rows.borrow_mut() = profile_rows;
        content.append(&profiles_group);

        // Preview section
        let preview_group = adw::PreferencesGroup::builder()
            .title("Preview")
            .description("Nix configuration that will be generated")
            .build();

        let preview_scroll = gtk::ScrolledWindow::builder()
            .height_request(200)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let preview_view = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .wrap_mode(gtk::WrapMode::Word)
            .build();
        preview_view.add_css_class("card");
        preview_view.buffer().set_text("# Select a profile to see preview");
        preview_scroll.set_child(Some(&preview_view));

        preview_group.add(&preview_scroll);
        content.append(&preview_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn create_profile_row(&self, profile: &ProfileDef) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(&profile.name)
            .subtitle(&profile.description)
            .activatable(true)
            .build();

        // Add icon
        row.add_prefix(&gtk::Image::from_icon_name(&profile.icon));

        // Add radio button (check mark style)
        let check = gtk::CheckButton::builder()
            .css_classes(["selection-mode"])
            .build();
        row.add_suffix(&check);
        row.set_activatable_widget(Some(&check));

        // Store profile ID
        let profile_id = profile.id.clone();

        // Connect selection
        check.connect_toggled(glib::clone!(@weak self as page => move |btn| {
            if btn.is_active() {
                page.select_profile(&profile_id);
            }
        }));

        row
    }

    fn select_profile(&self, profile_id: &str) {
        let imp = self.imp();

        // Update internal state
        *imp.selected_profile.borrow_mut() = Some(profile_id.to_string());

        // Update all rows to show correct selection state
        for (id, row) in imp.profile_rows.borrow().iter() {
            if let Some(check) = row.activatable_widget().and_then(|w| w.downcast::<gtk::CheckButton>().ok()) {
                check.set_active(id == profile_id);
            }
        }

        // Update main window state
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            window.update_app_state(|state| {
                state.select_profile(profile_id);
            });
        }

        tracing::info!("Selected profile: {}", profile_id);
    }

    pub fn get_selected_profile(&self) -> Option<String> {
        self.imp().selected_profile.borrow().clone()
    }

    /// Sync UI state from saved AppState (called after state is loaded)
    pub fn sync_from_state(&self, selected_profile: Option<&str>) {
        let imp = self.imp();

        if let Some(profile_id) = selected_profile {
            // Update internal state
            *imp.selected_profile.borrow_mut() = Some(profile_id.to_string());

            // Update all rows to show correct selection state
            for (id, row) in imp.profile_rows.borrow().iter() {
                if let Some(check) = row.activatable_widget().and_then(|w| w.downcast::<gtk::CheckButton>().ok()) {
                    check.set_active(id == profile_id);
                }
            }

            tracing::info!("Restored selected profile: {}", profile_id);
        }
    }
}

impl Default for ProfilesPage {
    fn default() -> Self {
        Self::new()
    }
}
