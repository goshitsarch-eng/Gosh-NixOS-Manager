//! System settings page (hostname, DNS, user groups)

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::actions::{default_system_actions, SystemActionType};
use gtk::glib;
use std::cell::RefCell;
use std::collections::HashMap;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct SystemPage {
        pub style_combo: RefCell<Option<adw::ComboRow>>,
        pub hostname_entry: RefCell<Option<adw::EntryRow>>,
        pub current_hostname: RefCell<Option<String>>,
        pub dns_entry: RefCell<Option<adw::EntryRow>>,
        pub group_switches: RefCell<HashMap<String, adw::SwitchRow>>,
        pub username_entry: RefCell<Option<adw::EntryRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SystemPage {
        const NAME: &'static str = "NixosToolkitSystemPage";
        type Type = super::SystemPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for SystemPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for SystemPage {}
    impl BoxImpl for SystemPage {}
}

glib::wrapper! {
    pub struct SystemPage(ObjectSubclass<imp::SystemPage>)
        @extends gtk::Box, gtk::Widget;
}

impl SystemPage {
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
            .label("System Settings")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Configure system-level settings. These will be applied through NixOS configuration.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Appearance group
        let appearance_group = adw::PreferencesGroup::builder()
            .title("Appearance")
            .build();

        let style_model = gtk::StringList::new(&["System", "Light", "Dark"]);
        let style_combo = adw::ComboRow::builder()
            .title("Style")
            .subtitle("Choose application color scheme")
            .model(&style_model)
            .build();
        style_combo.add_prefix(&gtk::Image::from_icon_name("weather-clear-symbolic"));

        // Load saved preference and set selection
        let prefs = crate::preferences::UserPreferences::load();
        style_combo.set_selected(match prefs.color_scheme {
            crate::preferences::ColorSchemePreference::System => 0,
            crate::preferences::ColorSchemePreference::Light => 1,
            crate::preferences::ColorSchemePreference::Dark => 2,
        });

        // Connect to selection changes for instant theme application
        style_combo.connect_selected_notify(|combo| {
            let scheme = match combo.selected() {
                1 => crate::preferences::ColorSchemePreference::Light,
                2 => crate::preferences::ColorSchemePreference::Dark,
                _ => crate::preferences::ColorSchemePreference::System,
            };

            // Apply immediately
            adw::StyleManager::default().set_color_scheme(scheme.to_adw_color_scheme());

            // Save preference
            let mut prefs = crate::preferences::UserPreferences::load();
            prefs.color_scheme = scheme;
            if let Err(e) = prefs.save() {
                tracing::warn!("Failed to save theme preference: {}", e);
            }
        });

        appearance_group.add(&style_combo);
        *imp.style_combo.borrow_mut() = Some(style_combo);
        content.append(&appearance_group);

        // Hostname group
        let hostname_group = adw::PreferencesGroup::builder()
            .title("Network Identity")
            .build();

        // Get current hostname
        let current_hostname = std::fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "nixos".to_string());

        *imp.current_hostname.borrow_mut() = Some(current_hostname.clone());

        let hostname_entry = adw::EntryRow::builder()
            .title("Hostname")
            .text(&current_hostname)
            .show_apply_button(true)
            .build();

        // Add icon
        hostname_entry.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));

        // Connect apply button
        hostname_entry.connect_apply(glib::clone!(@weak self as page => move |entry| {
            let new_hostname = entry.text().to_string();
            page.set_hostname(&new_hostname);
        }));

        // Also update state when text changes (not just on apply button click)
        hostname_entry.connect_changed(glib::clone!(@weak self as page => move |entry| {
            let new_hostname = entry.text().to_string();
            page.set_hostname(&new_hostname);
        }));

        hostname_group.add(&hostname_entry);
        *imp.hostname_entry.borrow_mut() = Some(hostname_entry);

        content.append(&hostname_group);

        // Info banner about hostname changes
        let info_row = adw::ActionRow::builder()
            .title("Note")
            .subtitle("Hostname changes require a system rebuild and may require a reboot to take full effect.")
            .build();
        info_row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        hostname_group.add(&info_row);

        // DNS Configuration group
        let dns_group = adw::PreferencesGroup::builder()
            .title("DNS Configuration")
            .description("Set custom DNS resolvers for your system")
            .build();

        let dns_entry = adw::EntryRow::builder()
            .title("DNS Servers")
            .text("")
            .show_apply_button(true)
            .build();
        dns_entry.add_prefix(&gtk::Image::from_icon_name("network-server-symbolic"));

        // Add helper text
        let dns_help = adw::ActionRow::builder()
            .title("Format")
            .subtitle("Enter DNS servers separated by commas (e.g., 1.1.1.1, 8.8.8.8)")
            .build();
        dns_help.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));

        dns_entry.connect_apply(glib::clone!(@weak self as page => move |entry| {
            let dns_text = entry.text().to_string();
            page.set_dns_servers(&dns_text);
        }));

        // Also update state when text changes
        dns_entry.connect_changed(glib::clone!(@weak self as page => move |entry| {
            let dns_text = entry.text().to_string();
            page.set_dns_servers(&dns_text);
        }));

        dns_group.add(&dns_entry);
        dns_group.add(&dns_help);
        *imp.dns_entry.borrow_mut() = Some(dns_entry);

        content.append(&dns_group);

        // User Groups section
        let groups_group = adw::PreferencesGroup::builder()
            .title("User Group Membership")
            .description("Add your user to system groups for specific functionality")
            .build();

        // Username entry
        let current_user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        let username_entry = adw::EntryRow::builder()
            .title("Username")
            .text(&current_user)
            .show_apply_button(true)
            .build();
        username_entry.add_prefix(&gtk::Image::from_icon_name("avatar-default-symbolic"));

        username_entry.connect_apply(glib::clone!(@weak self as page => move |entry| {
            let username = entry.text().to_string();
            page.set_username(&username);
        }));

        // Also update state when text changes
        username_entry.connect_changed(glib::clone!(@weak self as page => move |entry| {
            let username = entry.text().to_string();
            page.set_username(&username);
        }));

        groups_group.add(&username_entry);
        *imp.username_entry.borrow_mut() = Some(username_entry);

        // Add group switches from system actions
        let system_actions = default_system_actions();
        for action in &system_actions {
            if let SystemActionType::UserGroup { group } = &action.action_type {
                let switch_row = adw::SwitchRow::builder()
                    .title(&action.name)
                    .subtitle(&action.description)
                    .active(false)
                    .build();
                switch_row.add_prefix(&gtk::Image::from_icon_name(&action.icon));

                let group_name = group.clone();
                switch_row.connect_active_notify(glib::clone!(@weak self as page => move |switch| {
                    page.toggle_user_group(&group_name, switch.is_active());
                }));

                groups_group.add(&switch_row);
                imp.group_switches.borrow_mut().insert(group.clone(), switch_row);
            }
        }

        // Info banner about group changes
        let group_info = adw::ActionRow::builder()
            .title("Note")
            .subtitle("Group changes require a system rebuild. You may need to log out and back in for changes to take effect.")
            .build();
        group_info.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        groups_group.add(&group_info);

        content.append(&groups_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn set_hostname(&self, hostname: &str) {
        let imp = self.imp();

        // Validate hostname
        if hostname.is_empty() {
            tracing::warn!("Empty hostname not allowed");
            return;
        }

        if !hostname.chars().all(|c| c.is_alphanumeric() || c == '-') {
            tracing::warn!("Invalid hostname characters");
            return;
        }

        // Check if actually changed
        let current = imp.current_hostname.borrow();
        if current.as_ref() == Some(&hostname.to_string()) {
            tracing::info!("Hostname unchanged");
            return;
        }

        // Update main window state
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            window.update_app_state(|state| {
                state.set_hostname(hostname);
            });
        }

        tracing::info!("Hostname will be changed to: {}", hostname);
    }

    pub fn get_hostname(&self) -> Option<String> {
        self.imp()
            .hostname_entry
            .borrow()
            .as_ref()
            .map(|e| e.text().to_string())
    }

    fn set_dns_servers(&self, dns_text: &str) {
        // Parse comma-separated DNS servers
        let servers: Vec<String> = dns_text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Validate each server (basic IP format check)
        for server in &servers {
            if !Self::is_valid_ip(server) {
                tracing::warn!("Invalid DNS server format: {}", server);
                return;
            }
        }

        // Update main window state
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            window.update_app_state(|state| {
                state.set_dns_servers(servers.clone());
            });
        }

        tracing::info!("DNS servers set to: {:?}", servers);
    }

    fn is_valid_ip(ip: &str) -> bool {
        // Basic validation for IPv4 addresses
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        parts.iter().all(|part| {
            part.parse::<u8>().is_ok()
        })
    }

    fn set_username(&self, username: &str) {
        if username.is_empty() {
            tracing::warn!("Empty username not allowed");
            return;
        }

        // Basic username validation
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            tracing::warn!("Invalid username characters");
            return;
        }

        // Update main window state
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            window.update_app_state(|state| {
                state.set_username(username);
            });
        }

        tracing::info!("Username set to: {}", username);
    }

    fn toggle_user_group(&self, group: &str, active: bool) {
        // Update main window state
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            window.update_app_state(|state| {
                if active {
                    state.add_user_group(group);
                } else {
                    state.remove_user_group(group);
                }
            });
        }

        tracing::info!("User group {} set to: {}", group, active);
    }

    /// Get current DNS servers as comma-separated string
    pub fn get_dns_servers(&self) -> Option<String> {
        self.imp()
            .dns_entry
            .borrow()
            .as_ref()
            .map(|e| e.text().to_string())
    }

    /// Get username for group membership
    pub fn get_username(&self) -> Option<String> {
        self.imp()
            .username_entry
            .borrow()
            .as_ref()
            .map(|e| e.text().to_string())
    }

    /// Check if a group switch is active
    pub fn is_group_enabled(&self, group: &str) -> bool {
        self.imp()
            .group_switches
            .borrow()
            .get(group)
            .map(|s| s.is_active())
            .unwrap_or(false)
    }

    /// Sync UI from loaded state
    pub fn sync_from_state(
        &self,
        hostname: Option<&str>,
        dns_servers: &[String],
        username: Option<&str>,
        user_groups: &std::collections::HashSet<String>,
    ) {
        let imp = self.imp();

        // Restore hostname
        if let Some(h) = hostname {
            if let Some(entry) = imp.hostname_entry.borrow().as_ref() {
                entry.set_text(h);
            }
        }

        // Restore DNS servers
        if !dns_servers.is_empty() {
            if let Some(entry) = imp.dns_entry.borrow().as_ref() {
                entry.set_text(&dns_servers.join(", "));
            }
        }

        // Restore username
        if let Some(u) = username {
            if let Some(entry) = imp.username_entry.borrow().as_ref() {
                entry.set_text(u);
            }
        }

        // Restore user group switches
        for (group, switch) in imp.group_switches.borrow().iter() {
            switch.set_active(user_groups.contains(group));
        }

        tracing::info!(
            "SystemPage synced from state: hostname={:?}, dns={:?}, username={:?}, groups={:?}",
            hostname,
            dns_servers,
            username,
            user_groups
        );
    }
}

impl Default for SystemPage {
    fn default() -> Self {
        Self::new()
    }
}
