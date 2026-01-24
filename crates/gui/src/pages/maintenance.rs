//! Maintenance page for garbage collection and system cleanup

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::actions::{default_maintenance_actions, MaintenanceActionDef};
use common::ipc::{DiskUsageInfo, HelperRequest, HelperResponse};
use gtk::gio;
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct MaintenancePage {
        pub log_view: RefCell<Option<gtk::TextView>>,
        pub is_running: RefCell<bool>,
        pub store_size_row: RefCell<Option<adw::ActionRow>>,
        pub generations_row: RefCell<Option<adw::ActionRow>>,
        pub is_fetching_disk_info: RefCell<bool>,
        pub refresh_button: RefCell<Option<gtk::Button>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MaintenancePage {
        const NAME: &'static str = "NixosToolkitMaintenancePage";
        type Type = super::MaintenancePage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for MaintenancePage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for MaintenancePage {}
    impl BoxImpl for MaintenancePage {}
}

glib::wrapper! {
    pub struct MaintenancePage(ObjectSubclass<imp::MaintenancePage>)
        @extends gtk::Box, gtk::Widget;
}

impl MaintenancePage {
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
            .label("System Maintenance")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Clean up disk space and maintain your NixOS system.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // Actions group
        let actions_group = adw::PreferencesGroup::builder()
            .title("Maintenance Actions")
            .build();

        // Add action rows
        let actions = default_maintenance_actions();
        for action in &actions {
            let row = self.create_action_row(action);
            actions_group.add(&row);
        }

        content.append(&actions_group);

        // Disk usage info
        let disk_group = adw::PreferencesGroup::builder()
            .title("Disk Usage")
            .build();

        let store_size_row = adw::ActionRow::builder()
            .title("Nix Store Size")
            .subtitle("Loading...")
            .build();
        store_size_row.add_prefix(&gtk::Image::from_icon_name("drive-harddisk-symbolic"));
        disk_group.add(&store_size_row);
        *imp.store_size_row.borrow_mut() = Some(store_size_row);

        let generations_row = adw::ActionRow::builder()
            .title("System Generations")
            .subtitle("Loading...")
            .build();
        generations_row.add_prefix(&gtk::Image::from_icon_name("document-open-recent-symbolic"));
        disk_group.add(&generations_row);
        *imp.generations_row.borrow_mut() = Some(generations_row);

        content.append(&disk_group);

        // Refresh button
        let refresh_button = gtk::Button::builder()
            .label("Refresh Disk Info")
            .halign(gtk::Align::Start)
            .build();
        refresh_button.connect_clicked(glib::clone!(@weak self as page, @weak refresh_button => move |_| {
            page.refresh_disk_info(&refresh_button);
        }));
        content.append(&refresh_button);
        *imp.refresh_button.borrow_mut() = Some(refresh_button);

        // Log section
        let log_group = adw::PreferencesGroup::builder()
            .title("Output Log")
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
        log_view.buffer().set_text("# Maintenance log will appear here\n# Select an action above to run it");
        log_scroll.set_child(Some(&log_view));
        *imp.log_view.borrow_mut() = Some(log_view);

        log_group.add(&log_scroll);
        content.append(&log_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);

        // Auto-load disk info when page is realized
        self.connect_realize(|page| {
            if let Some(ref button) = *page.imp().refresh_button.borrow() {
                page.refresh_disk_info(button);
            }
        });
    }

    fn create_action_row(&self, action: &MaintenanceActionDef) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(&action.name)
            .subtitle(&action.description)
            .activatable(true)
            .build();

        // Add icon
        row.add_prefix(&gtk::Image::from_icon_name(&action.icon));

        // Add run button
        let run_button = gtk::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .tooltip_text("Run this action")
            .build();

        let action_id = action.id.clone();
        let action_name = action.name.clone();
        let action_command = action.command.clone();
        let action_warning = action.warning.clone();

        run_button.connect_clicked(glib::clone!(@weak self as page => move |_| {
            page.run_action(&action_id, &action_name, &action_command, action_warning.as_deref());
        }));

        row.add_suffix(&run_button);
        row
    }

    fn run_action(&self, _id: &str, name: &str, command: &str, warning: Option<&str>) {
        let imp = self.imp();

        if *imp.is_running.borrow() {
            self.append_log("Another action is already running. Please wait.\n");
            return;
        }

        // Show confirmation for dangerous actions
        if let Some(warning_msg) = warning {
            if let Some(window) = self.root().and_then(|r| r.downcast::<adw::ApplicationWindow>().ok()) {
                let dialog = adw::MessageDialog::builder()
                    .transient_for(&window)
                    .modal(true)
                    .heading(&format!("Run {}?", name))
                    .body(warning_msg)
                    .build();

                dialog.add_responses(&[
                    ("cancel", "Cancel"),
                    ("confirm", "Run Anyway"),
                ]);
                dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");

                let command = command.to_string();
                let name = name.to_string();
                dialog.connect_response(
                    None,
                    glib::clone!(@weak self as page => move |_, response| {
                        if response == "confirm" {
                            page.execute_command(&name, &command);
                        }
                    }),
                );

                dialog.present();
                return;
            }
        }

        self.execute_command(name, command);
    }

    fn execute_command(&self, name: &str, command: &str) {
        let imp = self.imp();
        *imp.is_running.borrow_mut() = true;

        self.append_log(&format!("\n--- Running: {} ---\n", name));
        self.append_log(&format!("$ pkexec {}\n\n", command));

        // Clone values for the async closure
        let command = command.to_string();
        let name = name.to_string();

        // Spawn the command using pkexec for privilege elevation
        glib::spawn_future_local(glib::clone!(@weak self as page => async move {
            let result = page.run_privileged_command(&command).await;

            match result {
                Ok(output) => {
                    if !output.stdout.is_empty() {
                        page.append_log(&output.stdout);
                    }
                    if !output.stderr.is_empty() {
                        page.append_log(&format!("\nStderr:\n{}", output.stderr));
                    }
                    if output.success {
                        page.append_log(&format!("\n--- {} completed successfully ---\n", name));
                    } else {
                        page.append_log(&format!("\n--- {} failed with exit code: {} ---\n",
                            name, output.exit_code.unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    page.append_log(&format!("\nError: {}\n", e));
                    page.append_log("Tip: Make sure polkit is configured and you have permission to run system commands.\n");
                }
            }

            *page.imp().is_running.borrow_mut() = false;
        }));
    }

    async fn run_privileged_command(&self, command: &str) -> Result<CommandOutput, String> {
        // Split command into program and args
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        let program = parts[0];
        let args = &parts[1..];

        // Try pkexec first (standard polkit)
        let result = tokio_process_spawn("pkexec", &[program].iter().chain(args.iter()).copied().collect::<Vec<_>>()).await;

        if result.is_ok() {
            return result;
        }

        // Fall back to trying without pkexec if the command doesn't require root
        // (like nix-channel --update which can work for user channels)
        if command.contains("nix-channel") {
            return tokio_process_spawn(program, args).await;
        }

        result
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

    fn refresh_disk_info(&self, button: &gtk::Button) {
        let imp = self.imp();

        // Prevent concurrent fetches
        if *imp.is_fetching_disk_info.borrow() {
            return;
        }
        *imp.is_fetching_disk_info.borrow_mut() = true;

        // Disable button to prevent multiple clicks
        button.set_sensitive(false);
        button.set_label("Calculating...");

        // Show loading state
        if let Some(ref row) = *imp.store_size_row.borrow() {
            row.set_subtitle("Calculating...");
        }
        if let Some(ref row) = *imp.generations_row.borrow() {
            row.set_subtitle("Calculating...");
        }

        // Spawn async task
        glib::spawn_future_local(glib::clone!(@weak self as page, @weak button => async move {
            let result = page.fetch_disk_usage().await;

            match result {
                Ok(info) => {
                    let imp = page.imp();
                    if let Some(ref row) = *imp.store_size_row.borrow() {
                        row.set_subtitle(&info.store_size);
                    }
                    if let Some(ref row) = *imp.generations_row.borrow() {
                        let subtitle = if info.generation_count == 1 {
                            "1 generation".to_string()
                        } else {
                            format!("{} generations", info.generation_count)
                        };
                        row.set_subtitle(&subtitle);
                    }
                    if let Some(error) = info.error {
                        page.append_log(&format!("Warning: {}\n", error));
                    }
                }
                Err(e) => {
                    page.append_log(&format!("Error fetching disk info: {}\n", e));
                    let imp = page.imp();
                    if let Some(ref row) = *imp.store_size_row.borrow() {
                        row.set_subtitle("Error - see log");
                    }
                    if let Some(ref row) = *imp.generations_row.borrow() {
                        row.set_subtitle("Error - see log");
                    }
                }
            }

            // Re-enable button
            button.set_sensitive(true);
            button.set_label("Refresh Disk Info");
            *page.imp().is_fetching_disk_info.borrow_mut() = false;
        }));
    }

    async fn fetch_disk_usage(&self) -> Result<DiskUsageInfo, String> {
        gio::spawn_blocking(|| {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Find helper binary path (same logic as HelperClient)
            let helper_path = std::env::var("NIXOS_TOOLKIT_HELPER").ok().or_else(|| {
                // Try relative to current exe (development mode)
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(dir) = exe.parent() {
                        let helper = dir.join("nixos-toolkit-helper");
                        if helper.exists() {
                            return Some(helper.to_string_lossy().to_string());
                        }
                    }
                }
                // Try system paths
                for path in &[
                    "/run/current-system/sw/bin/nixos-toolkit-helper",
                    "/usr/local/bin/nixos-toolkit-helper",
                    "/usr/bin/nixos-toolkit-helper",
                ] {
                    if std::path::Path::new(path).exists() {
                        return Some(path.to_string());
                    }
                }
                None
            }).unwrap_or_else(|| "nixos-toolkit-helper".to_string());

            // Spawn helper via pkexec
            // Remove SHELL env var to avoid pkexec rejecting nix-develop shells
            let mut child = Command::new("pkexec")
                .arg(&helper_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env_remove("SHELL")
                .spawn()
                .map_err(|e| format!("Failed to spawn helper: {}", e))?;

            // Send request
            let request = HelperRequest::GetDiskUsage;
            let json = serde_json::to_string(&request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;

            if let Some(stdin) = child.stdin.as_mut() {
                writeln!(stdin, "{}", json).map_err(|e| format!("Failed to write: {}", e))?;
            }
            // Close stdin to signal we're done sending
            drop(child.stdin.take());

            // Read response
            let output = child
                .wait_with_output()
                .map_err(|e| format!("Failed to wait for helper: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            for line in stdout.lines() {
                if let Ok(response) = serde_json::from_str::<HelperResponse>(line) {
                    match response {
                        HelperResponse::DiskUsage(info) => return Ok(info),
                        HelperResponse::Error { message, details } => {
                            return Err(format!(
                                "{}: {}",
                                message,
                                details.unwrap_or_default()
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // Include stderr in error for debugging
            if !stderr.is_empty() {
                Err(format!("No valid response from helper. Stderr: {}", stderr.trim()))
            } else if !output.status.success() {
                Err(format!("Helper exited with code: {:?}", output.status.code()))
            } else {
                Err("No valid response from helper".to_string())
            }
        })
        .await
        .map_err(|e| format!("Spawn error: {:?}", e))?
    }
}

/// Output from a command execution
struct CommandOutput {
    stdout: String,
    stderr: String,
    success: bool,
    exit_code: Option<i32>,
}

/// Spawn a process and capture its output
async fn tokio_process_spawn(program: &str, args: &[&str]) -> Result<CommandOutput, String> {
    use std::process::{Command, Stdio};

    // Use blocking spawn since we're in GTK context
    let output = std::thread::spawn({
        let program = program.to_string();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        move || {
            Command::new(&program)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        }
    })
    .join()
    .map_err(|_| "Thread panicked".to_string())?
    .map_err(|e| format!("Failed to execute command: {}", e))?;

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
        exit_code: output.status.code(),
    })
}

impl Default for MaintenancePage {
    fn default() -> Self {
        Self::new()
    }
}
