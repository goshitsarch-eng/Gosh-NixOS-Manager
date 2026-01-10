//! Bundle selection page with expandable package customization

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::actions::{default_bundles, BundleDef};
use common::{ArmCompat, CpuArch};
use gtk::glib;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, Default)]
    pub struct BundlesPage {
        /// Tracks which packages are enabled per bundle: bundle_id -> set of enabled packages
        pub enabled_packages: RefCell<HashMap<String, HashSet<String>>>,
        /// References to bundle expander rows
        pub bundle_rows: RefCell<Vec<(String, adw::ExpanderRow)>>,
        /// References to package check buttons: (bundle_id, package_name) -> CheckButton
        pub package_checks: RefCell<HashMap<(String, String), gtk::CheckButton>>,
        /// Summary label reference
        pub summary_label: RefCell<Option<gtk::Label>>,
        /// Flag to indicate we're syncing state (prevent signal feedback)
        pub is_syncing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BundlesPage {
        const NAME: &'static str = "NixosToolkitBundlesPage";
        type Type = super::BundlesPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for BundlesPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for BundlesPage {}
    impl BoxImpl for BundlesPage {}
}

glib::wrapper! {
    pub struct BundlesPage(ObjectSubclass<imp::BundlesPage>)
        @extends gtk::Box, gtk::Widget;
}

impl BundlesPage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("orientation", gtk::Orientation::Vertical)
            .property("spacing", 0)
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();
        let is_arm = CpuArch::detect().is_arm();

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
            .label("Software Bundles")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Enable bundles and expand to customize individual packages.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // ARM warning banner
        if is_arm {
            let arm_banner = adw::Banner::builder()
                .title("Running on ARM64 - some packages may not be available")
                .revealed(true)
                .build();
            arm_banner.add_css_class("warning");
            content.append(&arm_banner);
        }

        // Bundles group
        let bundles_group = adw::PreferencesGroup::builder()
            .title("Available Bundles")
            .description("Click to expand and customize packages")
            .build();

        // Add bundle rows
        let bundles = default_bundles();
        let mut bundle_rows = Vec::new();

        for bundle in &bundles {
            let row = self.create_bundle_row(bundle, is_arm);
            bundles_group.add(&row);
            bundle_rows.push((bundle.id.clone(), row));
        }

        *imp.bundle_rows.borrow_mut() = bundle_rows;
        content.append(&bundles_group);

        // Summary section
        let summary_group = adw::PreferencesGroup::builder()
            .title("Selected Packages")
            .description("Packages that will be installed")
            .build();

        let summary_label = gtk::Label::builder()
            .label("No packages selected")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        *imp.summary_label.borrow_mut() = Some(summary_label.clone());

        let summary_frame = gtk::Frame::new(None);
        summary_frame.set_child(Some(&summary_label));
        summary_frame.add_css_class("card");

        summary_group.add(&summary_frame);
        content.append(&summary_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn create_bundle_row(&self, bundle: &BundleDef, is_arm: bool) -> adw::ExpanderRow {
        let imp = self.imp();

        // Build subtitle with ARM warning if needed
        let subtitle = if is_arm && bundle.arm_compat != ArmCompat::Full {
            if let Some(ref note) = bundle.arm_note {
                format!("{} | ⚠️ {}", bundle.description, note)
            } else {
                format!("{} | ⚠️ {}", bundle.description, bundle.arm_compat.display_name())
            }
        } else {
            bundle.description.clone()
        };

        let row = adw::ExpanderRow::builder()
            .title(&bundle.name)
            .subtitle(&subtitle)
            .show_enable_switch(true)
            .enable_expansion(false)
            .build();

        // Add icon
        row.add_prefix(&gtk::Image::from_icon_name(&bundle.icon));

        // Add ARM compatibility indicator for ARM systems
        if is_arm && bundle.arm_compat != ArmCompat::Full {
            let compat_icon = match bundle.arm_compat {
                ArmCompat::None => gtk::Image::from_icon_name("action-unavailable-symbolic"),
                ArmCompat::Limited | ArmCompat::Partial => gtk::Image::from_icon_name("dialog-warning-symbolic"),
                ArmCompat::Full => gtk::Image::from_icon_name("emblem-ok-symbolic"),
            };
            compat_icon.add_css_class("warning");
            row.add_prefix(&compat_icon);
        }

        // Add package count suffix
        let count_label = gtk::Label::builder()
            .label(&format!("{} packages", bundle.packages.len()))
            .css_classes(["dim-label"])
            .build();
        row.add_suffix(&count_label);

        // Disable for ARM-incompatible bundles
        if is_arm && bundle.arm_compat == ArmCompat::None {
            row.set_sensitive(false);
        }

        // Add individual package rows inside the expander
        for package in &bundle.packages {
            let pkg_row = adw::ActionRow::builder()
                .title(package)
                .subtitle(&format!("nixpkgs#{}", package))
                .build();

            let check = gtk::CheckButton::builder()
                .valign(gtk::Align::Center)
                .build();

            // Store reference
            imp.package_checks.borrow_mut().insert(
                (bundle.id.clone(), package.clone()),
                check.clone(),
            );

            // Connect package toggle
            let bundle_id = bundle.id.clone();
            let package_name = package.clone();
            check.connect_toggled(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |check| {
                    page.toggle_package(&bundle_id, &package_name, check.is_active());
                }
            ));

            pkg_row.add_prefix(&check);
            pkg_row.set_activatable_widget(Some(&check));
            row.add_row(&pkg_row);
        }

        // Connect bundle enable switch
        let bundle_id = bundle.id.clone();
        let packages = bundle.packages.clone();
        row.connect_enable_expansion_notify(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |expander| {
                let enabled = expander.enables_expansion();
                page.toggle_bundle(&bundle_id, enabled, &packages);
            }
        ));

        row
    }

    fn toggle_bundle(&self, bundle_id: &str, enabled: bool, packages: &[String]) {
        let imp = self.imp();

        // Skip window update during sync to prevent feedback loops
        let is_syncing = imp.is_syncing.get();

        // Update all package checkboxes in this bundle
        for package in packages {
            let key = (bundle_id.to_string(), package.clone());
            if let Some(check) = imp.package_checks.borrow().get(&key) {
                check.set_active(enabled);
            }
        }

        // Update internal state
        if enabled {
            let pkg_set: HashSet<String> = packages.iter().cloned().collect();
            imp.enabled_packages.borrow_mut().insert(bundle_id.to_string(), pkg_set);
        } else {
            imp.enabled_packages.borrow_mut().remove(bundle_id);
        }

        // Update main window state (skip during sync to prevent feedback)
        if !is_syncing {
            if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
                window.update_app_state(|state| {
                    if enabled {
                        state.enable_bundle(bundle_id);
                    } else {
                        state.disable_bundle(bundle_id);
                    }
                });
            }

            tracing::info!(
                "Bundle {} {} (packages: {})",
                bundle_id,
                if enabled { "enabled" } else { "disabled" },
                packages.join(", ")
            );
        }

        self.update_summary();
    }

    fn toggle_package(&self, bundle_id: &str, package: &str, enabled: bool) {
        let imp = self.imp();

        // Update internal state
        let mut enabled_packages = imp.enabled_packages.borrow_mut();
        let bundle_packages = enabled_packages
            .entry(bundle_id.to_string())
            .or_insert_with(HashSet::new);

        if enabled {
            bundle_packages.insert(package.to_string());
        } else {
            bundle_packages.remove(package);
        }

        drop(enabled_packages);
        self.update_summary();

        tracing::info!(
            "Package {} in bundle {} {}",
            package,
            bundle_id,
            if enabled { "enabled" } else { "disabled" }
        );
    }

    fn update_summary(&self) {
        let imp = self.imp();

        // Collect all enabled packages
        let enabled_packages = imp.enabled_packages.borrow();
        let mut all_packages: Vec<String> = enabled_packages
            .values()
            .flat_map(|set| set.iter().cloned())
            .collect();
        all_packages.sort();
        all_packages.dedup();

        // Update summary label
        if let Some(ref label) = *imp.summary_label.borrow() {
            if all_packages.is_empty() {
                label.set_label("No packages selected");
            } else {
                label.set_label(&all_packages.join(", "));
            }
        }
    }

    /// Get all enabled packages across all bundles
    pub fn get_enabled_packages(&self) -> HashSet<String> {
        let imp = self.imp();
        imp.enabled_packages
            .borrow()
            .values()
            .flat_map(|set| set.iter().cloned())
            .collect()
    }

    /// Get enabled bundles (bundles with at least one package enabled)
    pub fn get_enabled_bundles(&self) -> HashSet<String> {
        let imp = self.imp();
        imp.enabled_packages
            .borrow()
            .iter()
            .filter(|(_, packages)| !packages.is_empty())
            .map(|(bundle_id, _)| bundle_id.clone())
            .collect()
    }

    /// Get enabled packages for a specific bundle
    pub fn get_bundle_packages(&self, bundle_id: &str) -> HashSet<String> {
        let imp = self.imp();
        imp.enabled_packages
            .borrow()
            .get(bundle_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Sync UI state from saved AppState (called after state is loaded)
    pub fn sync_from_state(&self, enabled_bundles: &std::collections::HashSet<String>) {
        let imp = self.imp();
        let bundles = default_bundles();

        tracing::debug!("Syncing bundles from state: {:?}", enabled_bundles);

        // Set syncing flag to prevent signal feedback loops
        imp.is_syncing.set(true);

        for (bundle_id, row) in imp.bundle_rows.borrow().iter() {
            let is_enabled = enabled_bundles.contains(bundle_id);

            // Set the expander row's enable switch
            row.set_enable_expansion(is_enabled);

            // If enabled, also check all packages
            if is_enabled {
                if let Some(bundle) = bundles.iter().find(|b| &b.id == bundle_id) {
                    let pkg_set: HashSet<String> = bundle.packages.iter().cloned().collect();
                    imp.enabled_packages.borrow_mut().insert(bundle_id.clone(), pkg_set.clone());

                    // Update checkboxes
                    for package in &bundle.packages {
                        let key = (bundle_id.clone(), package.clone());
                        if let Some(check) = imp.package_checks.borrow().get(&key) {
                            check.set_active(true);
                        }
                    }
                }
            }
        }

        // Clear syncing flag
        imp.is_syncing.set(false);

        self.update_summary();
        tracing::info!("Bundles synced from state: {} enabled", enabled_bundles.len());
    }
}

impl Default for BundlesPage {
    fn default() -> Self {
        Self::new()
    }
}
