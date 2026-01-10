//! Custom packages page for manually adding NixPkgs

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::actions::default_bundles;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PackagesPage {
        pub custom_packages: RefCell<HashSet<String>>,
        pub package_rows: RefCell<Vec<(String, adw::ActionRow)>>,
        pub packages_list: RefCell<Option<gtk::ListBox>>,
        pub entry: RefCell<Option<gtk::Entry>>,
        pub is_syncing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PackagesPage {
        const NAME: &'static str = "NixosToolkitPackagesPage";
        type Type = super::PackagesPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for PackagesPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for PackagesPage {}
    impl BoxImpl for PackagesPage {}
}

glib::wrapper! {
    pub struct PackagesPage(ObjectSubclass<imp::PackagesPage>)
        @extends gtk::Box, gtk::Widget;
}

impl PackagesPage {
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
            .label("Custom Packages")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Add individual packages from nixpkgs. Paste package names from search.nixos.org.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Input section
        let input_group = adw::PreferencesGroup::builder()
            .title("Add Package")
            .description("Paste package names like 'zed-editor' or 'pkgs.zed-editor' or even 'environment.systemPackages = [ pkgs.zed ];'")
            .build();

        let input_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let entry = gtk::Entry::builder()
            .placeholder_text("Package name (e.g., zed-editor, htop, neofetch)")
            .hexpand(true)
            .build();

        let add_button = gtk::Button::builder()
            .label("Add")
            .css_classes(["suggested-action"])
            .build();

        // Connect add button
        add_button.connect_clicked(glib::clone!(
            #[weak] entry,
            #[weak(rename_to = page)] self,
            move |_| {
                let text = entry.text();
                let packages = page.parse_package_input(&text);
                if !packages.is_empty() {
                    page.add_packages(packages);
                    entry.set_text("");
                }
            }
        ));

        // Connect Enter key on entry
        entry.connect_activate(glib::clone!(
            #[weak(rename_to = page)] self,
            #[weak] entry,
            move |_| {
                let text = entry.text();
                let packages = page.parse_package_input(&text);
                if !packages.is_empty() {
                    page.add_packages(packages);
                    entry.set_text("");
                }
            }
        ));

        *imp.entry.borrow_mut() = Some(entry.clone());

        input_box.append(&entry);
        input_box.append(&add_button);
        input_group.add(&input_box);
        content.append(&input_group);

        // Installed packages section
        let packages_group = adw::PreferencesGroup::builder()
            .title("Installed Custom Packages")
            .description("Packages will be installed when you click Apply")
            .build();

        let packages_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();

        // Placeholder row when empty
        let placeholder = adw::ActionRow::builder()
            .title("No custom packages")
            .subtitle("Add packages above to get started")
            .css_classes(["dim-label"])
            .build();
        packages_list.append(&placeholder);

        *imp.packages_list.borrow_mut() = Some(packages_list.clone());

        packages_group.add(&packages_list);
        content.append(&packages_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    /// Parse user input - supports multiple formats
    fn parse_package_input(&self, input: &str) -> Vec<String> {
        let input = input.trim();
        if input.is_empty() {
            return Vec::new();
        }

        let mut packages = Vec::new();

        // Handle full Nix expression: environment.systemPackages = [ pkgs.xxx ];
        if input.contains("environment.systemPackages") || input.contains('[') {
            // Extract content between brackets
            if let Some(start) = input.find('[') {
                if let Some(end) = input.rfind(']') {
                    let content = &input[start + 1..end];
                    // Split on whitespace and process each
                    for token in content.split_whitespace() {
                        let pkg = self.clean_package_name(token);
                        if !pkg.is_empty() && self.is_valid_package_name(&pkg) {
                            packages.push(pkg);
                        }
                    }
                    return packages;
                }
            }
        }

        // Handle comma-separated
        if input.contains(',') {
            for part in input.split(',') {
                let pkg = self.clean_package_name(part.trim());
                if !pkg.is_empty() && self.is_valid_package_name(&pkg) {
                    packages.push(pkg);
                }
            }
            return packages;
        }

        // Handle newline-separated
        if input.contains('\n') {
            for line in input.lines() {
                let pkg = self.clean_package_name(line.trim());
                if !pkg.is_empty() && self.is_valid_package_name(&pkg) {
                    packages.push(pkg);
                }
            }
            return packages;
        }

        // Handle space-separated (but be careful with "with pkgs;")
        if input.contains(' ') && !input.starts_with("pkgs.") {
            // Could be multiple packages or "with pkgs; [ ... ]"
            if input.starts_with("with pkgs;") {
                // Extract packages after "with pkgs;"
                let rest = input.trim_start_matches("with pkgs;").trim();
                return self.parse_package_input(rest);
            }

            // Otherwise, try space-separated
            for part in input.split_whitespace() {
                let pkg = self.clean_package_name(part);
                if !pkg.is_empty() && self.is_valid_package_name(&pkg) {
                    packages.push(pkg);
                }
            }
            if !packages.is_empty() {
                return packages;
            }
        }

        // Single package
        let pkg = self.clean_package_name(input);
        if !pkg.is_empty() && self.is_valid_package_name(&pkg) {
            packages.push(pkg);
        }

        packages
    }

    /// Clean a package name - remove prefixes and suffixes
    fn clean_package_name(&self, name: &str) -> String {
        let mut name = name.trim();

        // Strip pkgs. prefix
        if name.starts_with("pkgs.") {
            name = &name[5..];
        }

        // Strip nixpkgs# prefix (for flake references)
        if name.starts_with("nixpkgs#") {
            name = &name[8..];
        }

        // Strip trailing semicolons and brackets
        name = name.trim_end_matches(';').trim_end_matches(']').trim_start_matches('[');

        name.trim().to_string()
    }

    /// Check if a package name is valid
    fn is_valid_package_name(&self, name: &str) -> bool {
        if name.is_empty() || name.len() > 128 {
            return false;
        }

        // Must start with a letter
        let first = name.chars().next().unwrap();
        if !first.is_ascii_alphabetic() {
            return false;
        }

        // Rest can be alphanumeric, hyphens, underscores, dots (for nested packages)
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    }

    /// Add packages to the list
    fn add_packages(&self, packages: Vec<String>) {
        let imp = self.imp();

        // Check for duplicates in existing custom packages
        let mut current = imp.custom_packages.borrow_mut();
        let mut added = Vec::new();
        let mut duplicates = Vec::new();
        let mut in_bundle = Vec::new();

        // Get all bundle packages for duplicate detection
        let bundle_packages = self.get_all_bundle_packages();

        for pkg in packages {
            if current.contains(&pkg) {
                duplicates.push(pkg);
            } else if let Some(bundle_name) = bundle_packages.get(&pkg) {
                in_bundle.push((pkg, bundle_name.clone()));
            } else {
                current.insert(pkg.clone());
                added.push(pkg);
            }
        }
        drop(current);

        // Add rows for new packages
        for pkg in &added {
            self.add_package_row(pkg);
        }

        // Update main window state
        if !added.is_empty() {
            self.update_window_state();
        }

        // Show feedback via toast
        if let Some(window) = self.root().and_then(|r| r.downcast::<adw::ApplicationWindow>().ok()) {
            let toast_overlay = window.child().and_then(|c| c.first_child()).and_then(|c| c.downcast::<adw::ToastOverlay>().ok());

            if !added.is_empty() {
                let msg = if added.len() == 1 {
                    format!("Added: {}", added[0])
                } else {
                    format!("Added {} packages", added.len())
                };

                if let Some(ref overlay) = toast_overlay {
                    let toast = adw::Toast::new(&msg);
                    toast.set_timeout(3);
                    overlay.add_toast(toast);
                }
            }

            if !duplicates.is_empty() {
                let msg = format!("Already added: {}", duplicates.join(", "));
                if let Some(ref overlay) = toast_overlay {
                    let toast = adw::Toast::new(&msg);
                    toast.set_timeout(3);
                    overlay.add_toast(toast);
                }
            }

            for (pkg, bundle) in &in_bundle {
                let msg = format!("'{}' is already in '{}' bundle", pkg, bundle);
                if let Some(ref overlay) = toast_overlay {
                    let toast = adw::Toast::new(&msg);
                    toast.set_timeout(4);
                    overlay.add_toast(toast);
                }
            }
        }
    }

    /// Get all packages from all bundles for duplicate detection
    fn get_all_bundle_packages(&self) -> std::collections::HashMap<String, String> {
        let bundles = default_bundles();
        let mut map = std::collections::HashMap::new();

        for bundle in bundles {
            for pkg in &bundle.packages {
                map.insert(pkg.clone(), bundle.name.clone());
            }
        }

        map
    }

    /// Add a single package row to the list
    fn add_package_row(&self, package: &str) {
        let imp = self.imp();

        if let Some(ref list) = *imp.packages_list.borrow() {
            // Remove placeholder if present
            if let Some(first_row) = list.first_child() {
                if first_row.css_classes().contains(&glib::GString::from("dim-label")) {
                    list.remove(&first_row);
                }
            }

            let row = adw::ActionRow::builder()
                .title(package)
                .subtitle(&format!("pkgs.{}", package))
                .build();

            // Add package icon
            row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));

            // Add remove button
            let remove_button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat", "circular"])
                .tooltip_text("Remove package")
                .build();

            let pkg_name = package.to_string();
            remove_button.connect_clicked(glib::clone!(
                #[weak(rename_to = page)] self,
                move |_| {
                    page.remove_package(&pkg_name);
                }
            ));

            row.add_suffix(&remove_button);
            list.append(&row);

            // Track the row
            imp.package_rows.borrow_mut().push((package.to_string(), row));
        }
    }

    /// Remove a package
    fn remove_package(&self, package: &str) {
        let imp = self.imp();

        // Remove from internal state
        imp.custom_packages.borrow_mut().remove(package);

        // Remove from UI
        if let Some(ref list) = *imp.packages_list.borrow() {
            let mut rows = imp.package_rows.borrow_mut();
            if let Some(pos) = rows.iter().position(|(name, _)| name == package) {
                let (_, row) = rows.remove(pos);
                list.remove(&row);
            }

            // Add placeholder if empty
            if rows.is_empty() {
                let placeholder = adw::ActionRow::builder()
                    .title("No custom packages")
                    .subtitle("Add packages above to get started")
                    .css_classes(["dim-label"])
                    .build();
                list.append(&placeholder);
            }
        }

        // Update main window state
        self.update_window_state();
    }

    /// Update the main window's app state
    fn update_window_state(&self) {
        let imp = self.imp();

        if imp.is_syncing.get() {
            return;
        }

        if let Some(window) = self.root().and_then(|r| r.downcast::<crate::window::MainWindow>().ok()) {
            let packages: Vec<String> = imp.custom_packages.borrow().iter().cloned().collect();
            window.update_app_state(move |state| {
                state.custom_packages.clear();
                for pkg in &packages {
                    state.custom_packages.insert(pkg.clone());
                }
                state.has_changes = true;
            });
        }
    }

    /// Sync UI from loaded state
    pub fn sync_from_state(&self, packages: &HashSet<String>) {
        let imp = self.imp();
        imp.is_syncing.set(true);

        // Clear existing UI
        if let Some(ref list) = *imp.packages_list.borrow() {
            // Remove all existing rows
            while let Some(row) = list.first_child() {
                list.remove(&row);
            }
        }
        imp.package_rows.borrow_mut().clear();

        // Update internal state
        *imp.custom_packages.borrow_mut() = packages.clone();

        // Add rows for each package
        if packages.is_empty() {
            if let Some(ref list) = *imp.packages_list.borrow() {
                let placeholder = adw::ActionRow::builder()
                    .title("No custom packages")
                    .subtitle("Add packages above to get started")
                    .css_classes(["dim-label"])
                    .build();
                list.append(&placeholder);
            }
        } else {
            for pkg in packages {
                self.add_package_row(pkg);
            }
        }

        imp.is_syncing.set(false);
    }
}

impl Default for PackagesPage {
    fn default() -> Self {
        Self::new()
    }
}
