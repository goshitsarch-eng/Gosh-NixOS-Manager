//! Main application window with sidebar navigation

use crate::helper::HelperClient;
use crate::integration::detect_system;
use crate::pages::{
    ApplyPage, BundlesPage, GenerationsPage, HardwarePage, MaintenancePage,
    NetworkPage, OnboardingPage, PackagesPage, ProfilesPage, ServicesPage, SystemPage,
};
use crate::state::AppState;
use adw::prelude::*;
use adw::subclass::prelude::*;
use common::ipc::HelperRequest;
use common::{IntegrationStatus, SystemInfo};
use gtk::{gio, glib};
use std::cell::RefCell;
use std::time::Duration;

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MainWindow {
        pub split_view: adw::NavigationSplitView,
        pub sidebar_list: gtk::ListBox,
        pub content_stack: gtk::Stack,
        pub status_banner: adw::Banner,
        pub app_state: RefCell<AppState>,
        pub system_info: RefCell<SystemInfo>,
    }

    impl Default for MainWindow {
        fn default() -> Self {
            Self {
                split_view: adw::NavigationSplitView::new(),
                sidebar_list: gtk::ListBox::new(),
                content_stack: gtk::Stack::new(),
                status_banner: adw::Banner::new(""),
                app_state: RefCell::new(AppState::default()),
                system_info: RefCell::new(SystemInfo::default()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MainWindow {
        const NAME: &'static str = "NixosToolkitMainWindow";
        type Type = super::MainWindow;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for MainWindow {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
            self.obj().setup_actions();
            self.obj().detect_and_update();
            self.obj().load_state();
        }
    }

    impl WidgetImpl for MainWindow {}
    impl WindowImpl for MainWindow {}
    impl ApplicationWindowImpl for MainWindow {}
    impl AdwApplicationWindowImpl for MainWindow {}
}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Native, gtk::Root;
}

impl MainWindow {
    pub fn new(app: &crate::app::NixosToolkitApp) -> Self {
        let window: Self = glib::Object::builder()
            .property("application", app)
            .property("default-width", 1000)
            .property("default-height", 700)
            .property("title", "NixOS Toolkit")
            .build();
        window
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Setup sidebar
        imp.sidebar_list.add_css_class("navigation-sidebar");
        imp.sidebar_list.set_selection_mode(gtk::SelectionMode::Single);

        // Navigation items
        let nav_items = [
            ("onboarding", "Getting Started", "go-home-symbolic"),
            ("profiles", "Desktop Profiles", "user-desktop-symbolic"),
            ("bundles", "Software Bundles", "package-x-generic-symbolic"),
            ("packages", "Custom Packages", "list-add-symbolic"),
            ("system", "System Settings", "preferences-system-symbolic"),
            ("hardware", "Hardware", "video-display-symbolic"),
            ("network", "Network", "network-workgroup-symbolic"),
            ("services", "Services", "system-run-symbolic"),
            ("generations", "Generations", "document-open-recent-symbolic"),
            ("maintenance", "Maintenance", "user-trash-symbolic"),
            ("apply", "Apply Changes", "emblem-synchronizing-symbolic"),
        ];

        for (id, label, icon) in nav_items {
            let row = adw::ActionRow::builder()
                .title(label)
                .activatable(true)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(icon));
            row.set_widget_name(id);
            imp.sidebar_list.append(&row);
        }

        // Connect sidebar selection
        imp.sidebar_list.connect_row_selected(
            glib::clone!(@weak self as window => move |_, row| {
                if let Some(row) = row {
                    let name = row.widget_name();
                    window.imp().content_stack.set_visible_child_name(&name);
                    window.imp().split_view.set_show_content(true);
                }
            }),
        );

        // Setup content stack
        imp.content_stack.set_transition_type(gtk::StackTransitionType::Crossfade);

        // Add pages
        let onboarding = OnboardingPage::new();
        let profiles = ProfilesPage::new();
        let bundles = BundlesPage::new();
        let packages = PackagesPage::new();
        let system = SystemPage::new();
        let hardware = HardwarePage::new();
        let network = NetworkPage::new();
        let services = ServicesPage::new();
        let generations = GenerationsPage::new();
        let maintenance = MaintenancePage::new();
        let apply = ApplyPage::new();

        imp.content_stack.add_named(&onboarding, Some("onboarding"));
        imp.content_stack.add_named(&profiles, Some("profiles"));
        imp.content_stack.add_named(&bundles, Some("bundles"));
        imp.content_stack.add_named(&packages, Some("packages"));
        imp.content_stack.add_named(&system, Some("system"));
        imp.content_stack.add_named(&hardware, Some("hardware"));
        imp.content_stack.add_named(&network, Some("network"));
        imp.content_stack.add_named(&services, Some("services"));
        imp.content_stack.add_named(&generations, Some("generations"));
        imp.content_stack.add_named(&maintenance, Some("maintenance"));
        imp.content_stack.add_named(&apply, Some("apply"));

        // Create sidebar navigation page
        let sidebar_toolbar = adw::ToolbarView::new();
        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.set_show_title(true);
        sidebar_toolbar.add_top_bar(&sidebar_header);

        let sidebar_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&imp.sidebar_list)
            .build();
        sidebar_toolbar.set_content(Some(&sidebar_scroll));

        let sidebar_page = adw::NavigationPage::builder()
            .title("NixOS Toolkit")
            .child(&sidebar_toolbar)
            .build();

        // Create content navigation page
        let content_toolbar = adw::ToolbarView::new();

        // Add banner at top of content
        imp.status_banner.set_revealed(false);
        content_toolbar.add_top_bar(&imp.status_banner);

        let content_header = adw::HeaderBar::new();
        content_toolbar.add_top_bar(&content_header);
        content_toolbar.set_content(Some(&imp.content_stack));

        let content_page = adw::NavigationPage::builder()
            .child(&content_toolbar)
            .build();

        // Configure split view
        imp.split_view.set_sidebar(Some(&sidebar_page));
        imp.split_view.set_content(Some(&content_page));
        imp.split_view.set_min_sidebar_width(220.0);
        imp.split_view.set_max_sidebar_width(320.0);

        // Add breakpoint for responsive design
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            600.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&imp.split_view, "collapsed", Some(&true.to_value()));
        self.add_breakpoint(breakpoint);

        self.set_content(Some(&imp.split_view));

        // Select first row by default
        if let Some(first_row) = imp.sidebar_list.row_at_index(0) {
            imp.sidebar_list.select_row(Some(&first_row));
        }
    }

    fn setup_actions(&self) {
        // Refresh action
        let refresh_action = gio::ActionEntry::builder("refresh")
            .activate(|window: &Self, _, _| {
                window.detect_and_update();
            })
            .build();

        self.add_action_entries([refresh_action]);
    }

    fn detect_and_update(&self) {
        let imp = self.imp();

        // Detect system configuration
        let info = detect_system();
        *imp.system_info.borrow_mut() = info.clone();

        // Update banner based on integration status
        if !info.is_nixos {
            imp.status_banner.set_title("Not running on NixOS");
            imp.status_banner.add_css_class("error");
            imp.status_banner.set_revealed(true);
        } else {
            match info.integration_status {
                IntegrationStatus::Integrated => {
                    imp.status_banner.set_title("Integrated - Ready to apply changes");
                    imp.status_banner.remove_css_class("error");
                    imp.status_banner.remove_css_class("warning");
                    imp.status_banner.add_css_class("success");
                    imp.status_banner.set_revealed(true);
                }
                IntegrationStatus::NotIntegrated => {
                    imp.status_banner.set_title("Setup required - See Getting Started");
                    imp.status_banner.remove_css_class("error");
                    imp.status_banner.remove_css_class("success");
                    imp.status_banner.add_css_class("warning");
                    imp.status_banner.set_revealed(true);
                }
                IntegrationStatus::Unknown => {
                    imp.status_banner.set_revealed(false);
                }
            }
        }

        // Update pages with system info
        if let Some(onboarding) = imp
            .content_stack
            .child_by_name("onboarding")
            .and_then(|w| w.downcast::<OnboardingPage>().ok())
        {
            onboarding.update_system_info(&info);
        }

        tracing::info!(
            "System detection complete: is_nixos={}, mode={:?}, integration={:?}",
            info.is_nixos,
            info.config_mode,
            info.integration_status
        );
    }

    pub fn get_system_info(&self) -> SystemInfo {
        self.imp().system_info.borrow().clone()
    }

    pub fn get_app_state(&self) -> AppState {
        let imp = self.imp();
        let mut state = imp.app_state.borrow().clone();

        // Collect current network config from NetworkPage
        if let Some(network_page) = imp
            .content_stack
            .child_by_name("network")
            .and_then(|w| w.downcast::<NetworkPage>().ok())
        {
            state.network_config = network_page.get_network_config();
        }

        // Collect current services config from ServicesPage
        if let Some(services_page) = imp
            .content_stack
            .child_by_name("services")
            .and_then(|w| w.downcast::<ServicesPage>().ok())
        {
            state.services_config = services_page.get_services_config();
        }

        state
    }

    pub fn update_app_state<F>(&self, f: F)
    where
        F: FnOnce(&mut AppState),
    {
        f(&mut self.imp().app_state.borrow_mut());
    }

    /// Load persisted state from the helper on startup
    fn load_state(&self) {
        let imp = self.imp();

        tracing::info!("Loading application state...");

        // Try to spawn helper and read state
        match HelperClient::spawn_privileged() {
            Ok(mut client) => {
                tracing::debug!("Helper spawned successfully, requesting state...");

                if let Err(e) = client.send(&HelperRequest::ReadState) {
                    tracing::warn!("Failed to request state: {}", e);
                    eprintln!("Failed to request state from helper: {}", e);
                    self.show_state_load_warning("Failed to communicate with helper");
                    return;
                }

                // Wait for response with timeout
                match client.recv_timeout(Duration::from_secs(5)) {
                    Some(common::ipc::HelperResponse::State(ipc_state)) => {
                        let loaded_state = AppState::from_ipc_state(ipc_state);
                        tracing::info!(
                            "State loaded successfully: profile={:?}, bundles={:?}, packages={:?}",
                            loaded_state.selected_profile,
                            loaded_state.enabled_bundles,
                            loaded_state.custom_packages
                        );

                        // Store state first, then sync UI
                        *imp.app_state.borrow_mut() = loaded_state.clone();

                        // Sync UI pages with loaded state
                        self.sync_pages_from_state(&loaded_state);
                    }
                    Some(common::ipc::HelperResponse::Error { message, details }) => {
                        tracing::warn!("State read error: {} ({:?})", message, details);
                        eprintln!("State read error: {} ({:?})", message, details);
                        self.show_state_load_warning(&format!("State read error: {}", message));
                    }
                    None => {
                        tracing::warn!("State read timeout - helper may have failed");
                        eprintln!("State read timeout - helper may have failed");
                        self.show_state_load_warning("State read timeout");
                    }
                    _ => {
                        tracing::debug!("Unexpected response when reading state");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Could not spawn helper for state load: {}", e);
                eprintln!("Could not spawn helper: {} - state will not be restored", e);
                self.show_state_load_warning("Could not load saved state - using defaults");
            }
        }
    }

    /// Show a warning when state loading fails
    fn show_state_load_warning(&self, message: &str) {
        let imp = self.imp();
        imp.status_banner.set_title(message);
        imp.status_banner.remove_css_class("success");
        imp.status_banner.add_css_class("warning");
        imp.status_banner.set_revealed(true);
    }

    /// Sync page UIs with loaded state
    fn sync_pages_from_state(&self, state: &AppState) {
        let imp = self.imp();

        // Sync BundlesPage
        if let Some(bundles_page) = imp
            .content_stack
            .child_by_name("bundles")
            .and_then(|w| w.downcast::<BundlesPage>().ok())
        {
            bundles_page.sync_from_state(&state.enabled_bundles, &state.bundle_packages);
        }

        // Sync ProfilesPage
        if let Some(profiles_page) = imp
            .content_stack
            .child_by_name("profiles")
            .and_then(|w| w.downcast::<ProfilesPage>().ok())
        {
            profiles_page.sync_from_state(state.selected_profile.as_deref());
        }

        // Sync PackagesPage
        if let Some(packages_page) = imp
            .content_stack
            .child_by_name("packages")
            .and_then(|w| w.downcast::<PackagesPage>().ok())
        {
            packages_page.sync_from_state(&state.custom_packages);
        }

        // Sync NetworkPage
        if let Some(network_page) = imp
            .content_stack
            .child_by_name("network")
            .and_then(|w| w.downcast::<NetworkPage>().ok())
        {
            network_page.set_network_config(&state.network_config);
        }

        // Sync ServicesPage
        if let Some(services_page) = imp
            .content_stack
            .child_by_name("services")
            .and_then(|w| w.downcast::<ServicesPage>().ok())
        {
            services_page.set_services_config(&state.services_config);
        }
    }
}
