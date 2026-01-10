//! Onboarding page for first-time setup

use crate::integration::{classic_integration_snippet, flake_integration_snippet};
use adw::prelude::*;
use adw::subclass::prelude::*;
use common::{ConfigMode, IntegrationStatus, SystemInfo};
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct OnboardingPage {
        pub status_row: RefCell<Option<adw::ActionRow>>,
        pub snippet_view: RefCell<Option<gtk::TextView>>,
        pub system_info: RefCell<SystemInfo>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OnboardingPage {
        const NAME: &'static str = "NixosToolkitOnboardingPage";
        type Type = super::OnboardingPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for OnboardingPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for OnboardingPage {}
    impl BoxImpl for OnboardingPage {}
}

glib::wrapper! {
    pub struct OnboardingPage(ObjectSubclass<imp::OnboardingPage>)
        @extends gtk::Box, gtk::Widget;
}

impl OnboardingPage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("orientation", gtk::Orientation::Vertical)
            .property("spacing", 0)
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Wrap everything in a scrolled window for scrollable content
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
            .label("Welcome to NixOS Toolkit")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("This tool helps you manage your NixOS configuration declaratively.\nFollow the steps below to complete the one-time setup.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Status card
        let status_group = adw::PreferencesGroup::builder()
            .title("System Status")
            .build();

        let status_row = adw::ActionRow::builder()
            .title("Checking...")
            .subtitle("Detecting system configuration")
            .build();
        status_row.add_prefix(&gtk::Image::from_icon_name("emblem-system-symbolic"));
        status_group.add(&status_row);
        *imp.status_row.borrow_mut() = Some(status_row);

        content.append(&status_group);

        // Integration instructions
        let instructions_group = adw::PreferencesGroup::builder()
            .title("One-Time Setup")
            .description("Follow these steps to integrate the toolkit with your NixOS configuration")
            .build();

        // Code snippet view
        let snippet_scroll = gtk::ScrolledWindow::builder()
            .height_request(280)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let snippet_view = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .wrap_mode(gtk::WrapMode::Word)
            .build();
        snippet_view.add_css_class("card");
        snippet_scroll.set_child(Some(&snippet_view));
        *imp.snippet_view.borrow_mut() = Some(snippet_view);

        instructions_group.add(&snippet_scroll);
        content.append(&instructions_group);

        // Action buttons
        let button_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Start)
            .build();

        let copy_button = gtk::Button::builder()
            .label("Copy Snippet")
            .css_classes(["suggested-action"])
            .build();
        copy_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.copy_snippet();
        }));

        let open_folder_button = gtk::Button::builder()
            .label("Open /etc/nixos")
            .build();
        open_folder_button.connect_clicked(|_| {
            let _ = std::process::Command::new("xdg-open")
                .arg("/etc/nixos")
                .spawn();
        });

        let verify_button = gtk::Button::builder()
            .label("Verify Integration")
            .build();
        verify_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.verify_integration();
        }));

        button_box.append(&copy_button);
        button_box.append(&open_folder_button);
        button_box.append(&verify_button);
        content.append(&button_box);

        scroll.set_child(Some(&content));
        self.append(&scroll);

        // Initial update
        self.update_system_info(&SystemInfo::default());
    }

    pub fn update_system_info(&self, info: &SystemInfo) {
        let imp = self.imp();
        *imp.system_info.borrow_mut() = info.clone();

        // Update status row
        if let Some(ref status_row) = *imp.status_row.borrow() {
            if !info.is_nixos {
                status_row.set_title("Not NixOS");
                status_row.set_subtitle("This tool only works on NixOS systems");
                status_row.add_css_class("error");
            } else {
                let mode_str = info.config_mode.display_name();
                let status_str = match info.integration_status {
                    IntegrationStatus::Integrated => "Integrated",
                    IntegrationStatus::NotIntegrated => "Not integrated",
                    IntegrationStatus::Unknown => "Unknown",
                };

                status_row.set_title(&format!("NixOS Detected ({})", mode_str));
                status_row.set_subtitle(&format!(
                    "Integration: {} | Version: {}",
                    status_str,
                    info.nixos_version.as_deref().unwrap_or("unknown")
                ));

                match info.integration_status {
                    IntegrationStatus::Integrated => {
                        status_row.remove_css_class("error");
                        status_row.remove_css_class("warning");
                    }
                    IntegrationStatus::NotIntegrated => {
                        status_row.remove_css_class("error");
                        status_row.add_css_class("warning");
                    }
                    IntegrationStatus::Unknown => {}
                }
            }
        }

        // Update snippet based on config mode
        if let Some(ref snippet_view) = *imp.snippet_view.borrow() {
            let snippet = match info.config_mode {
                ConfigMode::Flake => flake_integration_snippet(),
                ConfigMode::Classic | ConfigMode::Unknown => classic_integration_snippet(),
            };
            snippet_view.buffer().set_text(&snippet);
        }
    }

    fn copy_snippet(&self) {
        let imp = self.imp();
        if let Some(ref snippet_view) = *imp.snippet_view.borrow() {
            let buffer = snippet_view.buffer();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);

            if let Some(display) = gtk::gdk::Display::default() {
                let clipboard = display.clipboard();
                clipboard.set_text(&text);

                // Show toast notification
                if let Some(window) = self.root().and_then(|r| r.downcast::<adw::ApplicationWindow>().ok()) {
                    let toast = adw::Toast::new("Snippet copied to clipboard");
                    // Try to find toast overlay and add the toast
                    if let Some(content) = window.content() {
                        if let Some(overlay) = content.downcast_ref::<adw::ToastOverlay>() {
                            overlay.add_toast(toast);
                        } else {
                            tracing::info!("Copied snippet to clipboard (no toast overlay)");
                        }
                    } else {
                        // Fallback: just log
                        let _ = toast; // silence unused warning
                        tracing::info!("Copied snippet to clipboard");
                    }
                }
            }
        }
    }

    fn verify_integration(&self) {
        // Re-run detection
        let info = crate::integration::detect_system();
        self.update_system_info(&info);

        // Show result
        let message = match info.integration_status {
            IntegrationStatus::Integrated => "Integration verified! You're ready to use the toolkit.",
            IntegrationStatus::NotIntegrated => "Integration not detected. Please add the import and run 'nixos-rebuild switch'.",
            IntegrationStatus::Unknown => "Could not verify integration. Please check manually.",
        };

        tracing::info!("Integration check: {}", message);
    }
}

impl Default for OnboardingPage {
    fn default() -> Self {
        Self::new()
    }
}
