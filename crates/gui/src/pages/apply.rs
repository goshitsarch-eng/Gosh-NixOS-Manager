//! Apply changes page with preview and log viewer

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::actions::{default_bundles, default_profiles};
use common::ipc::{HelperRequest, HelperResponse, RebuildType};
use common::nix::{generate_preview_full, NixGenOptions};
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

use crate::helper::HelperClient;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ApplyPage {
        pub preview_view: RefCell<Option<gtk::TextView>>,
        pub log_view: RefCell<Option<gtk::TextView>>,
        pub apply_button: RefCell<Option<gtk::Button>>,
        pub spinner: RefCell<Option<gtk::Spinner>>,
        pub status_label: RefCell<Option<gtk::Label>>,
        pub is_applying: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplyPage {
        const NAME: &'static str = "NixosToolkitApplyPage";
        type Type = super::ApplyPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for ApplyPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for ApplyPage {}
    impl BoxImpl for ApplyPage {}
}

glib::wrapper! {
    pub struct ApplyPage(ObjectSubclass<imp::ApplyPage>)
        @extends gtk::Box, gtk::Widget;
}

impl ApplyPage {
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
            .label("Apply Changes")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Review your configuration and apply changes to the system.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Preview section
        let preview_group = adw::PreferencesGroup::builder()
            .title("Configuration Preview")
            .description("Nix files that will be written")
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
        preview_view.buffer().set_text("# No changes to preview\n# Select a profile or bundles to see the configuration");
        preview_scroll.set_child(Some(&preview_view));
        *imp.preview_view.borrow_mut() = Some(preview_view);

        preview_group.add(&preview_scroll);
        content.append(&preview_group);

        // Refresh preview button
        let refresh_button = gtk::Button::builder()
            .label("Refresh Preview")
            .halign(gtk::Align::Start)
            .build();
        refresh_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.refresh_preview();
        }));
        content.append(&refresh_button);

        // Action buttons
        let button_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Start)
            .margin_top(12)
            .build();

        let apply_button = gtk::Button::builder()
            .label("Apply Changes")
            .css_classes(["suggested-action", "pill"])
            .build();
        apply_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.show_apply_dialog();
        }));
        *imp.apply_button.borrow_mut() = Some(apply_button.clone());

        let dry_run_button = gtk::Button::builder()
            .label("Dry Run")
            .css_classes(["pill"])
            .tooltip_text("Build configuration without activating")
            .build();
        dry_run_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.do_dry_run();
        }));

        // Spinner for progress indication
        let spinner = gtk::Spinner::builder()
            .spinning(false)
            .visible(false)
            .build();
        *imp.spinner.borrow_mut() = Some(spinner.clone());

        // Status label
        let status_label = gtk::Label::builder()
            .label("")
            .css_classes(["dim-label"])
            .visible(false)
            .build();
        *imp.status_label.borrow_mut() = Some(status_label.clone());

        button_box.append(&apply_button);
        button_box.append(&dry_run_button);
        button_box.append(&spinner);
        button_box.append(&status_label);
        content.append(&button_box);

        // Log section
        let log_group = adw::PreferencesGroup::builder()
            .title("Build Log")
            .description("Output from nixos-rebuild")
            .build();

        let log_scroll = gtk::ScrolledWindow::builder()
            .height_request(200)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let log_view = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .wrap_mode(gtk::WrapMode::Word)
            .build();
        log_view.add_css_class("card");
        log_view.buffer().set_text("# Build log will appear here when you apply changes");
        log_scroll.set_child(Some(&log_view));
        *imp.log_view.borrow_mut() = Some(log_view);

        log_group.add(&log_scroll);
        content.append(&log_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn refresh_preview(&self) {
        let imp = self.imp();

        // Get current state from main window
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            let state = window.get_app_state();
            let profiles = default_profiles();
            let bundles = default_bundles();

            // Find selected profile
            let profile = state
                .selected_profile
                .as_ref()
                .and_then(|id| profiles.iter().find(|p| &p.id == id));

            // Find enabled bundles
            let enabled_bundles: Vec<_> = bundles
                .iter()
                .filter(|b| state.enabled_bundles.contains(&b.id))
                .collect();

            // Convert bundle_packages from HashSet to Vec for IPC format
            let bundle_packages: std::collections::HashMap<String, Vec<String>> = state
                .bundle_packages
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
                .collect();

            // Build options including custom packages and bundle_packages
            let options = NixGenOptions {
                profile,
                bundles: enabled_bundles,
                bundle_packages,
                hostname: state.hostname.as_deref(),
                dns_servers: state.dns_servers.clone(),
                user_groups: state.user_groups.iter().cloned().collect(),
                username: state.username.as_deref(),
                bluetooth_enabled: state.bluetooth_enabled,
                custom_packages: state.custom_packages.iter().cloned().collect(),
                network_config: state.network_config.clone(),
                services_config: state.services_config.clone(),
            };

            // Generate preview
            let preview = generate_preview_full(&options);

            if let Some(ref view) = *imp.preview_view.borrow() {
                view.buffer().set_text(&preview);
            }
        }
    }

    fn show_apply_dialog(&self) {
        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            let state = window.get_app_state();

            // Check if state appears empty - warn user about potential data loss
            let has_no_profile = state.selected_profile.is_none();
            let has_no_bundles = state.enabled_bundles.is_empty();
            let has_no_packages = state.custom_packages.is_empty();

            let (heading, body, destructive) = if has_no_profile && has_no_bundles && has_no_packages {
                (
                    "Apply Empty Configuration?",
                    "WARNING: You have no profile, bundles, or packages selected.\n\n\
                     Applying this will remove ALL managed software from your system.\n\n\
                     If you previously had bundles or packages installed through this tool, \
                     they will be REMOVED.\n\n\
                     Are you sure you want to continue?",
                    true
                )
            } else if has_no_profile && has_no_bundles {
                (
                    "Apply Configuration?",
                    "Note: No desktop profile or bundles are selected.\n\n\
                     This will run 'nixos-rebuild switch' with only your custom packages.\n\n\
                     Make sure you have reviewed the preview above.",
                    false
                )
            } else {
                (
                    "Apply Configuration?",
                    "This will run 'nixos-rebuild switch' with your selected configuration.\n\n\
                     Make sure you have reviewed the preview above.",
                    false
                )
            };

            let dialog = adw::MessageDialog::builder()
                .transient_for(&window)
                .modal(true)
                .heading(heading)
                .body(body)
                .build();

            dialog.add_responses(&[
                ("cancel", "Cancel"),
                ("apply", "Apply"),
            ]);

            if destructive {
                dialog.set_response_appearance("apply", adw::ResponseAppearance::Destructive);
            } else {
                dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);
            }
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");

            dialog.connect_response(
                None,
                glib::clone!(@weak self as page => move |_, response| {
                    if response == "apply" {
                        page.do_apply();
                    }
                }),
            );

            dialog.present();
        }
    }

    fn do_apply(&self) {
        let imp = self.imp();

        if *imp.is_applying.borrow() {
            tracing::warn!("Already applying, ignoring request");
            return;
        }

        *imp.is_applying.borrow_mut() = true;

        // Disable apply button and show spinner
        if let Some(ref button) = *imp.apply_button.borrow() {
            button.set_sensitive(false);
            button.set_label("Applying...");
        }

        // Show spinner and status
        if let Some(ref spinner) = *imp.spinner.borrow() {
            spinner.set_visible(true);
            spinner.set_spinning(true);
        }
        if let Some(ref label) = *imp.status_label.borrow() {
            label.set_visible(true);
            label.set_label("Building configuration...");
        }

        // Clear log
        if let Some(ref view) = *imp.log_view.borrow() {
            view.buffer().set_text("Starting nixos-rebuild switch...\n\n");
        }

        // Get current state from main window
        let (selected_profile, enabled_bundles, bundle_packages, hostname, dns_servers, user_groups, username, bluetooth_enabled, custom_packages, network_config, services_config) = if let Some(window) = self
            .root()
            .and_then(|r| r.downcast::<crate::window::MainWindow>().ok())
        {
            let state = window.get_app_state();
            (
                state.selected_profile.clone(),
                state.enabled_bundles.iter().cloned().collect::<Vec<_>>(),
                state
                    .bundle_packages
                    .iter()
                    .map(|(k, v)| (k.clone(), v.iter().cloned().collect::<Vec<_>>()))
                    .collect::<std::collections::HashMap<_, _>>(),
                state.hostname.clone(),
                state.dns_servers.clone(),
                state.user_groups.iter().cloned().collect::<Vec<_>>(),
                state.username.clone(),
                state.bluetooth_enabled,
                state.custom_packages.iter().cloned().collect::<Vec<_>>(),
                state.network_config.clone(),
                state.services_config.clone(),
            )
        } else {
            self.append_log("Error: Could not get application state\n");
            self.finish_apply(false);
            return;
        };

        self.append_log(&format!(
            "Profile: {}\n",
            selected_profile.as_deref().unwrap_or("None")
        ));
        self.append_log(&format!("Bundles: {:?}\n", enabled_bundles));
        if !custom_packages.is_empty() {
            self.append_log(&format!("Custom packages: {:?}\n", custom_packages));
        }
        self.append_log(&format!(
            "Hostname: {}\n",
            hostname.as_deref().unwrap_or("(unchanged)")
        ));
        if !dns_servers.is_empty() {
            self.append_log(&format!("DNS servers: {:?}\n", dns_servers));
        }
        if !user_groups.is_empty() {
            self.append_log(&format!("User groups: {:?}\n", user_groups));
        }
        if bluetooth_enabled {
            self.append_log("Bluetooth: Enabled\n");
        }
        if network_config.has_settings() {
            self.append_log(&format!("Network: SSH={}, Tailscale={}, TCP ports={:?}\n",
                network_config.ssh_enabled, network_config.tailscale_enabled, network_config.allowed_tcp_ports));
        }
        if services_config.has_settings() {
            self.append_log(&format!("Services: {:?}\n", services_config.enabled_services()));
        }
        self.append_log("\n");

        // Spawn helper with pkexec
        let helper_path = std::env::var("NIXOS_TOOLKIT_HELPER")
            .unwrap_or_else(|_| "nixos-toolkit-helper".to_string());
        self.append_log(&format!("Launching privileged helper: {}\n", helper_path));
        self.append_log("(you may be prompted for your password)...\n");

        let helper_result = HelperClient::spawn_privileged();

        match helper_result {
            Ok(mut client) => {
                // First ensure directories exist
                self.append_log("Ensuring directories exist...\n");
                if let Err(e) = client.send(&HelperRequest::EnsureDirectories) {
                    self.append_log(&format!("Error sending EnsureDirectories: {}\n", e));
                    self.finish_apply(false);
                    return;
                }

                // Wait for response
                let response = client.recv_timeout(std::time::Duration::from_secs(10));
                tracing::info!("EnsureDirectories response: {:?}", response);
                match response {
                    Some(HelperResponse::Ok) => {
                        self.append_log("Directories ready.\n");
                    }
                    Some(HelperResponse::Error { message, details }) => {
                        self.append_log(&format!("Error: {}\n", message));
                        if let Some(d) = details {
                            self.append_log(&format!("Details: {}\n", d));
                        }
                        self.finish_apply(false);
                        return;
                    }
                    Some(other) => {
                        self.append_log(&format!("Unexpected response from helper: {:?}\n", other));
                        self.finish_apply(false);
                        return;
                    }
                    None => {
                        self.append_log("No response from helper (timeout or connection lost)\n");
                        self.finish_apply(false);
                        return;
                    }
                }

                // Send Apply request
                self.append_log("Generating configuration and running nixos-rebuild switch...\n\n");
                let request = HelperRequest::Apply {
                    selected_profile,
                    enabled_bundles,
                    bundle_packages,
                    hostname,
                    dns_servers,
                    user_groups,
                    username,
                    bluetooth_enabled,
                    custom_packages,
                    network_config,
                    services_config,
                    rebuild_type: RebuildType::Switch,
                };

                if let Err(e) = client.send(&request) {
                    self.append_log(&format!("Error sending Apply request: {}\n", e));
                    self.finish_apply(false);
                    return;
                }

                // Poll for responses using glib timeout
                let client = Rc::new(RefCell::new(Some(client)));
                let page = self.downgrade();

                glib::timeout_add_local(
                    std::time::Duration::from_millis(100),
                    move || {
                        let Some(page) = page.upgrade() else {
                            return glib::ControlFlow::Break;
                        };

                        let mut client_ref = client.borrow_mut();
                        let Some(ref mut client) = *client_ref else {
                            return glib::ControlFlow::Break;
                        };

                        // Try to receive responses
                        while let Some(response) = client.try_recv() {
                            match response {
                                HelperResponse::Log { level, message } => {
                                    page.append_log(&format!("[{}] {}\n", level.as_str(), message));
                                }
                                HelperResponse::ApplyComplete { success, message } => {
                                    page.append_log(&format!("\n{}\n", message));

                                    // Save state using the same helper session if apply succeeded
                                    if success {
                                        if let Some(window) = page
                                            .root()
                                            .and_then(|r| r.downcast::<crate::window::MainWindow>().ok())
                                        {
                                            let state = window.get_app_state();
                                            let ipc_state = state.to_ipc_state();
                                            page.append_log("Saving state...\n");

                                            if let Err(e) = client.send(&HelperRequest::WriteState { state: ipc_state }) {
                                                page.append_log(&format!("Warning: Could not save state: {}\n", e));
                                            } else {
                                                // Wait briefly for response
                                                match client.recv_timeout(std::time::Duration::from_secs(3)) {
                                                    Some(HelperResponse::Ok) => {
                                                        page.append_log("State saved for next session.\n");
                                                    }
                                                    Some(HelperResponse::Error { message, .. }) => {
                                                        page.append_log(&format!("Warning: State save failed: {}\n", message));
                                                    }
                                                    _ => {
                                                        page.append_log("State save completed.\n");
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    page.finish_apply(success);
                                    *client_ref = None;
                                    return glib::ControlFlow::Break;
                                }
                                HelperResponse::Error { message, details } => {
                                    page.append_log(&format!("\nError: {}\n", message));
                                    if let Some(d) = details {
                                        page.append_log(&format!("Details: {}\n", d));
                                    }
                                    page.finish_apply(false);
                                    *client_ref = None;
                                    return glib::ControlFlow::Break;
                                }
                                _ => {
                                    tracing::debug!("Unexpected response: {:?}", response);
                                }
                            }
                        }

                        glib::ControlFlow::Continue
                    },
                );
            }
            Err(e) => {
                self.append_log(&format!("Failed to start helper: {}\n", e));
                self.append_log("\nTo apply changes manually:\n");
                self.append_log("1. Run: sudo nixos-rebuild switch\n");
                self.finish_apply(false);
            }
        }
    }

    fn finish_apply(&self, success: bool) {
        let imp = self.imp();
        *imp.is_applying.borrow_mut() = false;

        // Re-enable button
        if let Some(ref button) = *imp.apply_button.borrow() {
            button.set_sensitive(true);
            button.set_label("Apply Changes");
        }

        // Stop and hide spinner
        if let Some(ref spinner) = *imp.spinner.borrow() {
            spinner.set_spinning(false);
            spinner.set_visible(false);
        }

        // Update status label
        if let Some(ref label) = *imp.status_label.borrow() {
            if success {
                label.set_label("✓ Complete");
                label.remove_css_class("error");
                label.add_css_class("success");
            } else {
                label.set_label("✗ Failed");
                label.remove_css_class("success");
                label.add_css_class("error");
            }
        }

        if success {
            self.append_log("\n✓ Configuration applied successfully!\n");
            // State is already saved inline using the same helper session
        } else {
            self.append_log("\n✗ Apply failed. Check the log above for details.\n");
        }
    }

    /// Save current state to persist selections for next app launch
    fn save_state(&self) {
        // Get current state from main window
        if let Some(window) = self
            .root()
            .and_then(|r| r.downcast::<crate::window::MainWindow>().ok())
        {
            let state = window.get_app_state();
            let ipc_state = state.to_ipc_state();

            self.append_log("Saving state...\n");

            // Spawn helper to save state (reuse existing helper session if possible)
            match HelperClient::spawn_privileged() {
                Ok(mut client) => {
                    if let Err(e) = client.send(&HelperRequest::WriteState { state: ipc_state }) {
                        self.append_log(&format!("Warning: Could not save state: {}\n", e));
                        return;
                    }

                    // Wait for response with short timeout
                    match client.recv_timeout(std::time::Duration::from_secs(5)) {
                        Some(HelperResponse::Ok) => {
                            self.append_log("State saved for next session.\n");
                        }
                        Some(HelperResponse::Error { message, .. }) => {
                            self.append_log(&format!("Warning: State save failed: {}\n", message));
                        }
                        _ => {
                            self.append_log("Warning: State save response timeout.\n");
                        }
                    }
                }
                Err(e) => {
                    self.append_log(&format!("Warning: Could not save state: {}\n", e));
                }
            }
        }
    }

    fn do_dry_run(&self) {
        self.append_log("\n--- Dry Run ---\n");
        self.append_log("Would run: nixos-rebuild dry-build\n");
        self.refresh_preview();
        self.append_log("Preview updated. No changes applied.\n");
    }

    fn append_log(&self, text: &str) {
        if let Some(ref view) = *self.imp().log_view.borrow() {
            let buffer = view.buffer();
            let mut end = buffer.end_iter();
            buffer.insert(&mut end, text);

            // Scroll to end using idle callback to avoid conflicts with window redraws
            let view_clone = view.clone();
            glib::idle_add_local_once(move || {
                if let Some(buffer) = view_clone.buffer().into() {
                    let buffer: gtk::TextBuffer = buffer;
                    let end_iter = buffer.end_iter();
                    view_clone.scroll_to_iter(&mut end_iter.clone(), 0.0, false, 0.0, 1.0);
                }
            });
        }
    }
}

impl Default for ApplyPage {
    fn default() -> Self {
        Self::new()
    }
}
