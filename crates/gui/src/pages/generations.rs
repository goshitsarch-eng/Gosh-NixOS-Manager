//! Generations page for viewing and managing NixOS system generations

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct GenerationsPage {
        pub generations_list: RefCell<Option<gtk::ListBox>>,
        pub log_view: RefCell<Option<gtk::TextView>>,
        pub is_loading: RefCell<bool>,
        pub current_generation: RefCell<Option<u32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenerationsPage {
        const NAME: &'static str = "NixosToolkitGenerationsPage";
        type Type = super::GenerationsPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for GenerationsPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for GenerationsPage {}
    impl BoxImpl for GenerationsPage {}
}

glib::wrapper! {
    pub struct GenerationsPage(ObjectSubclass<imp::GenerationsPage>)
        @extends gtk::Box, gtk::Widget;
}

impl GenerationsPage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("orientation", gtk::Orientation::Vertical)
            .property("spacing", 0)
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Wrap everything in a scrolled window
        let outer_scroll = gtk::ScrolledWindow::builder()
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
            .label("System Generations")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("View, manage, and rollback NixOS system generations. Each generation represents a complete system configuration that you can boot into.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Actions bar
        let actions_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .build();

        let refresh_button = gtk::Button::builder()
            .label("Refresh")
            .icon_name("view-refresh-symbolic")
            .build();
        refresh_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.load_generations();
        }));
        actions_box.append(&refresh_button);

        let rollback_button = gtk::Button::builder()
            .label("Rollback to Previous")
            .icon_name("edit-undo-symbolic")
            .css_classes(["suggested-action"])
            .build();
        rollback_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.rollback_to_previous();
        }));
        actions_box.append(&rollback_button);

        content.append(&actions_box);

        // Generations list
        let generations_group = adw::PreferencesGroup::builder()
            .title("Available Generations")
            .description("Select a generation to view details or perform actions")
            .build();

        let scroll = gtk::ScrolledWindow::builder()
            .height_request(250)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        scroll.set_child(Some(&list));
        *imp.generations_list.borrow_mut() = Some(list);

        generations_group.add(&scroll);
        content.append(&generations_group);

        // Boot menu info
        let boot_info = adw::PreferencesGroup::builder()
            .title("Boot Menu")
            .build();

        let boot_row = adw::ActionRow::builder()
            .title("Boot into different generations")
            .subtitle("At boot time, press a key (usually Esc or Enter) to access the GRUB/systemd-boot menu and select older generations.")
            .build();
        boot_row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        boot_info.add(&boot_row);

        content.append(&boot_info);

        // Log section
        let log_group = adw::PreferencesGroup::builder()
            .title("Operation Log")
            .build();

        let log_scroll = gtk::ScrolledWindow::builder()
            .height_request(150)
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
        log_view.buffer().set_text("# Generation operations will be logged here\n");
        log_scroll.set_child(Some(&log_view));
        *imp.log_view.borrow_mut() = Some(log_view);

        log_group.add(&log_scroll);
        content.append(&log_group);

        outer_scroll.set_child(Some(&content));
        self.append(&outer_scroll);

        // Load generations when widget is realized (ensures async task runs properly)
        self.connect_realize(|page| {
            page.load_generations();
        });
    }

    fn load_generations(&self) {
        let imp = self.imp();

        if *imp.is_loading.borrow() {
            return;
        }
        *imp.is_loading.borrow_mut() = true;

        self.append_log("Loading generations...\n");

        // Clear existing list
        if let Some(ref list) = *imp.generations_list.borrow() {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            // Add loading indicator
            let loading_row = adw::ActionRow::builder()
                .title("Loading...")
                .build();
            loading_row.add_prefix(&gtk::Spinner::builder().spinning(true).build());
            list.append(&loading_row);
        }

        // Spawn async task to load generations
        glib::spawn_future_local(glib::clone!(@weak self as page => async move {
            let result = page.fetch_generations().await;
            page.display_generations(result);
            *page.imp().is_loading.borrow_mut() = false;
        }));
    }

    async fn fetch_generations(&self) -> Vec<GenerationInfo> {
        // Read generations directly from filesystem (doesn't require root)
        let mut generations = Vec::new();
        let profiles_dir = "/nix/var/nix/profiles";

        tracing::debug!("Fetching generations from {}", profiles_dir);

        // Check if the profiles directory exists
        if !std::path::Path::new(profiles_dir).exists() {
            tracing::warn!("Profiles directory does not exist: {}", profiles_dir);
            self.append_log(&format!("Warning: {} not found. Are you running on NixOS?\n", profiles_dir));
            return generations;
        }

        // Find current generation by reading the system symlink
        let current_gen = std::fs::read_link(format!("{}/system", profiles_dir))
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .and_then(|s| {
                // Parse "system-N-link" format
                s.strip_prefix("system-")
                    .and_then(|rest| rest.strip_suffix("-link"))
                    .and_then(|num| num.parse::<u32>().ok())
            });

        tracing::debug!("Current generation: {:?}", current_gen);

        // Read the profiles directory
        if let Ok(entries) = std::fs::read_dir(profiles_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Match pattern: system-N-link
                if name.starts_with("system-") && name.ends_with("-link") {
                    if let Some(num_str) = name.strip_prefix("system-").and_then(|s| s.strip_suffix("-link")) {
                        if let Ok(number) = num_str.parse::<u32>() {
                            // Get modification time for date
                            let date = entry.metadata()
                                .and_then(|m| m.modified())
                                .ok()
                                .and_then(|t| {
                                    t.duration_since(std::time::UNIX_EPOCH).ok()
                                })
                                .map(|d| {
                                    // Convert to a readable format using glib's DateTime
                                    let secs = d.as_secs() as i64;
                                    glib::DateTime::from_unix_local(secs)
                                        .and_then(|dt| dt.format("%Y-%m-%d %H:%M"))
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|_| "Unknown".to_string())
                                })
                                .unwrap_or_else(|| "Unknown".to_string());

                            let current = current_gen == Some(number);

                            generations.push(GenerationInfo {
                                number,
                                date,
                                current,
                                nixos_version: None,
                                kernel_version: None,
                            });
                        }
                    }
                }
            }
        }

        // Sort by generation number descending (newest first)
        generations.sort_by(|a, b| b.number.cmp(&a.number));
        generations
    }

    fn parse_generation_line(line: &str) -> Option<GenerationInfo> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let number: u32 = parts[0].parse().ok()?;
        let current = line.contains("(current)");

        // Try to extract date (format varies)
        let date = if parts.len() >= 3 {
            format!("{} {}", parts[1], parts.get(2).unwrap_or(&""))
        } else {
            "Unknown".to_string()
        };

        Some(GenerationInfo {
            number,
            date,
            current,
            nixos_version: None,
            kernel_version: None,
        })
    }

    fn display_generations(&self, generations: Vec<GenerationInfo>) {
        let imp = self.imp();

        if let Some(ref list) = *imp.generations_list.borrow() {
            // Clear loading indicator
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            if generations.is_empty() {
                let empty_row = adw::ActionRow::builder()
                    .title("No generations found")
                    .subtitle("This might indicate an issue with your NixOS installation")
                    .build();
                empty_row.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
                list.append(&empty_row);
                self.append_log("No generations found.\n");
                return;
            }

            // Store current generation
            for gen in &generations {
                if gen.current {
                    *imp.current_generation.borrow_mut() = Some(gen.number);
                    break;
                }
            }

            self.append_log(&format!("Found {} generations.\n", generations.len()));

            for gen in generations {
                let row = self.create_generation_row(&gen);
                list.append(&row);
            }
        }
    }

    fn create_generation_row(&self, gen: &GenerationInfo) -> adw::ActionRow {
        let title = if gen.current {
            format!("Generation {} (current)", gen.number)
        } else {
            format!("Generation {}", gen.number)
        };

        let row = adw::ActionRow::builder()
            .title(&title)
            .subtitle(&gen.date)
            .build();

        // Add icon
        let icon_name = if gen.current {
            "emblem-ok-symbolic"
        } else {
            "document-open-recent-symbolic"
        };
        row.add_prefix(&gtk::Image::from_icon_name(icon_name));

        // Add current badge if applicable
        if gen.current {
            let badge = gtk::Label::builder()
                .label("Current")
                .css_classes(["success", "caption"])
                .valign(gtk::Align::Center)
                .build();
            row.add_suffix(&badge);
        } else {
            // Add switch button for non-current generations
            let switch_button = gtk::Button::builder()
                .icon_name("system-switch-user-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .tooltip_text("Switch to this generation")
                .build();

            let gen_number = gen.number;
            switch_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
                page.switch_to_generation(gen_number);
            }));
            row.add_suffix(&switch_button);

            // Add delete button
            let delete_button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat", "error"])
                .tooltip_text("Delete this generation")
                .build();

            let gen_number = gen.number;
            delete_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
                page.confirm_delete_generation(gen_number);
            }));
            row.add_suffix(&delete_button);
        }

        row
    }

    fn rollback_to_previous(&self) {
        let imp = self.imp();
        let current = *imp.current_generation.borrow();

        if let Some(current_gen) = current {
            if current_gen > 1 {
                self.switch_to_generation(current_gen - 1);
            } else {
                self.append_log("Cannot rollback: already at the first generation.\n");
            }
        } else {
            self.append_log("Cannot rollback: current generation unknown.\n");
        }
    }

    fn switch_to_generation(&self, generation: u32) {
        if let Some(window) = self.root().and_then(|r| r.downcast::<adw::ApplicationWindow>().ok()) {
            let dialog = adw::MessageDialog::builder()
                .transient_for(&window)
                .modal(true)
                .heading(&format!("Switch to Generation {}?", generation))
                .body("This will rebuild your system to use the selected generation. The system will activate the new configuration.")
                .build();

            dialog.add_responses(&[
                ("cancel", "Cancel"),
                ("boot", "Set for Next Boot"),
                ("switch", "Switch Now"),
            ]);
            dialog.set_response_appearance("switch", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");

            dialog.connect_response(
                None,
                glib::clone!(@weak self as page => move |_, response| {
                    match response {
                        "switch" => page.execute_switch(generation, "switch"),
                        "boot" => page.execute_switch(generation, "boot"),
                        _ => {}
                    }
                }),
            );

            dialog.present();
        }
    }

    fn execute_switch(&self, generation: u32, mode: &str) {
        self.append_log(&format!("\n--- Switching to generation {} ({}) ---\n", generation, mode));

        let mode = mode.to_string();
        let mode_for_check = mode.clone();
        glib::spawn_future_local(glib::clone!(@weak self as page => async move {
            let result = std::thread::spawn(move || {
                // Build the profile path for the specific generation
                let profile_path = format!("/nix/var/nix/profiles/system-{}-link", generation);

                // Use pkexec for privilege elevation
                let output = std::process::Command::new("pkexec")
                    .args([&profile_path.replace("-link", ""), "/bin/switch-to-configuration", &mode])
                    .output();

                // Alternative: use nix-env to switch profile then activate
                if output.is_err() {
                    std::process::Command::new("pkexec")
                        .args(["nix-env", "-p", "/nix/var/nix/profiles/system", "--switch-generation", &generation.to_string()])
                        .output()
                        .ok();

                    std::process::Command::new("pkexec")
                        .args(["/nix/var/nix/profiles/system/bin/switch-to-configuration", &mode])
                        .output()
                }
                else {
                    output
                }
            })
            .join()
            .ok()
            .and_then(|r| r.ok());

            match result {
                Some(output) if output.status.success() => {
                    page.append_log("Switch successful!\n");
                    if mode_for_check == "boot" {
                        page.append_log("The selected generation will be activated on next boot.\n");
                    }
                    page.load_generations();
                }
                Some(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    page.append_log(&format!("Switch failed:\n{}\n", stderr));
                }
                None => {
                    page.append_log("Failed to execute switch command.\n");
                }
            }
        }));
    }

    fn confirm_delete_generation(&self, generation: u32) {
        if let Some(window) = self.root().and_then(|r| r.downcast::<adw::ApplicationWindow>().ok()) {
            let dialog = adw::MessageDialog::builder()
                .transient_for(&window)
                .modal(true)
                .heading(&format!("Delete Generation {}?", generation))
                .body("This will permanently delete this generation. You will not be able to boot into or rollback to this configuration.")
                .build();

            dialog.add_responses(&[
                ("cancel", "Cancel"),
                ("delete", "Delete"),
            ]);
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");

            dialog.connect_response(
                None,
                glib::clone!(@weak self as page => move |_, response| {
                    if response == "delete" {
                        page.delete_generation(generation);
                    }
                }),
            );

            dialog.present();
        }
    }

    fn delete_generation(&self, generation: u32) {
        self.append_log(&format!("\n--- Deleting generation {} ---\n", generation));

        glib::spawn_future_local(glib::clone!(@weak self as page => async move {
            let result = std::thread::spawn(move || {
                std::process::Command::new("pkexec")
                    .args(["nix-env", "-p", "/nix/var/nix/profiles/system", "--delete-generations", &generation.to_string()])
                    .output()
            })
            .join()
            .ok()
            .and_then(|r| r.ok());

            match result {
                Some(output) if output.status.success() => {
                    page.append_log(&format!("Generation {} deleted.\n", generation));
                    page.load_generations();
                }
                Some(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    page.append_log(&format!("Delete failed:\n{}\n", stderr));
                }
                None => {
                    page.append_log("Failed to execute delete command.\n");
                }
            }
        }));
    }

    fn append_log(&self, text: &str) {
        if let Some(ref view) = *self.imp().log_view.borrow() {
            let buffer = view.buffer();
            let mut end = buffer.end_iter();
            buffer.insert(&mut end, text);

            // Scroll to end
            let mark = buffer.create_mark(None, &buffer.end_iter(), false);
            view.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
        }
    }
}

impl Default for GenerationsPage {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a NixOS generation (local struct for parsing)
#[derive(Debug, Clone)]
struct GenerationInfo {
    number: u32,
    date: String,
    current: bool,
    nixos_version: Option<String>,
    kernel_version: Option<String>,
}
