//! Network configuration page (Firewall, SSH, VPN)

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::ipc::NetworkConfig;
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct NetworkPage {
        // Firewall settings
        pub firewall_enabled: RefCell<Option<adw::SwitchRow>>,
        pub open_ports_entry: RefCell<Option<adw::EntryRow>>,
        pub allowed_tcp_ports: RefCell<Vec<u16>>,
        pub allowed_udp_ports: RefCell<Vec<u16>>,
        // SSH settings
        pub ssh_enabled: RefCell<Option<adw::SwitchRow>>,
        pub ssh_port: RefCell<Option<adw::SpinRow>>,
        pub ssh_password_auth: RefCell<Option<adw::SwitchRow>>,
        pub ssh_root_login: RefCell<Option<adw::ComboRow>>,
        pub fail2ban_enabled: RefCell<Option<adw::SwitchRow>>,
        // VPN settings
        pub tailscale_enabled: RefCell<Option<adw::SwitchRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworkPage {
        const NAME: &'static str = "NixosToolkitNetworkPage";
        type Type = super::NetworkPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for NetworkPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for NetworkPage {}
    impl BoxImpl for NetworkPage {}
}

glib::wrapper! {
    pub struct NetworkPage(ObjectSubclass<imp::NetworkPage>)
        @extends gtk::Box, gtk::Widget;
}

impl NetworkPage {
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
            .label("Network & Security")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Configure firewall rules, SSH access, and VPN settings.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // === Firewall Configuration ===
        let firewall_group = adw::PreferencesGroup::builder()
            .title("Firewall")
            .description("Configure NixOS firewall rules")
            .build();

        let fw_enabled = adw::SwitchRow::builder()
            .title("Enable Firewall")
            .subtitle("Block incoming connections except for allowed ports")
            .active(true)
            .build();
        fw_enabled.add_prefix(&gtk::Image::from_icon_name("security-high-symbolic"));
        firewall_group.add(&fw_enabled);
        *imp.firewall_enabled.borrow_mut() = Some(fw_enabled);

        // Preset ports
        let presets_row = adw::ActionRow::builder()
            .title("Quick Open Ports")
            .subtitle("Common service ports")
            .build();
        presets_row.add_prefix(&gtk::Image::from_icon_name("network-server-symbolic"));

        let presets_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .valign(gtk::Align::Center)
            .build();

        let port_presets = [
            ("SSH", 22),
            ("HTTP", 80),
            ("HTTPS", 443),
            ("Alt HTTP", 8080),
        ];

        for (name, port) in port_presets {
            let button = gtk::ToggleButton::builder()
                .label(&format!("{} ({})", name, port))
                .css_classes(["flat", "caption"])
                .build();

            let port_val = port;
            button.connect_toggled(glib::clone!(@weak self as page => move |btn| {
                page.toggle_port(port_val, btn.is_active());
            }));

            presets_box.append(&button);
        }
        presets_row.add_suffix(&presets_box);
        firewall_group.add(&presets_row);

        // Custom ports entry
        let ports_entry = adw::EntryRow::builder()
            .title("Additional TCP Ports")
            .text("")
            .show_apply_button(true)
            .build();
        ports_entry.add_prefix(&gtk::Image::from_icon_name("network-wired-symbolic"));

        let ports_help = adw::ActionRow::builder()
            .title("Format")
            .subtitle("Enter port numbers separated by commas (e.g., 3000, 5432, 6379)")
            .build();
        ports_help.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));

        ports_entry.connect_apply(glib::clone!(@weak self as page => move |entry| {
            page.parse_custom_ports(&entry.text());
        }));

        // Also update state when text changes
        ports_entry.connect_changed(glib::clone!(@weak self as page => move |entry| {
            page.parse_custom_ports(&entry.text());
        }));

        firewall_group.add(&ports_entry);
        firewall_group.add(&ports_help);
        *imp.open_ports_entry.borrow_mut() = Some(ports_entry);

        content.append(&firewall_group);

        // === SSH Configuration ===
        let ssh_group = adw::PreferencesGroup::builder()
            .title("SSH Server")
            .description("Configure OpenSSH server for remote access")
            .build();

        let ssh_enabled = adw::SwitchRow::builder()
            .title("Enable SSH Server")
            .subtitle("Allow remote SSH connections to this machine")
            .active(false)
            .build();
        ssh_enabled.add_prefix(&gtk::Image::from_icon_name("utilities-terminal-symbolic"));
        ssh_group.add(&ssh_enabled);
        *imp.ssh_enabled.borrow_mut() = Some(ssh_enabled);

        // SSH Port
        let ssh_port = adw::SpinRow::builder()
            .title("SSH Port")
            .subtitle("Port number for SSH connections")
            .adjustment(&gtk::Adjustment::new(22.0, 1.0, 65535.0, 1.0, 10.0, 0.0))
            .build();
        ssh_port.add_prefix(&gtk::Image::from_icon_name("network-wired-symbolic"));
        ssh_group.add(&ssh_port);
        *imp.ssh_port.borrow_mut() = Some(ssh_port);

        // Password authentication
        let password_auth = adw::SwitchRow::builder()
            .title("Password Authentication")
            .subtitle("Allow password-based SSH login (key-only is more secure)")
            .active(false)
            .build();
        password_auth.add_prefix(&gtk::Image::from_icon_name("dialog-password-symbolic"));
        ssh_group.add(&password_auth);
        *imp.ssh_password_auth.borrow_mut() = Some(password_auth);

        // Root login
        let root_login = adw::ComboRow::builder()
            .title("Root Login")
            .subtitle("Control root user SSH access")
            .build();
        root_login.set_model(Some(&gtk::StringList::new(&[
            "Disabled (recommended)",
            "Prohibit Password (keys only)",
            "Enabled (not recommended)",
        ])));
        root_login.set_selected(0);
        root_login.add_prefix(&gtk::Image::from_icon_name("system-users-symbolic"));
        ssh_group.add(&root_login);
        *imp.ssh_root_login.borrow_mut() = Some(root_login);

        // Fail2ban
        let fail2ban = adw::SwitchRow::builder()
            .title("Fail2Ban")
            .subtitle("Ban IPs with too many failed login attempts")
            .active(false)
            .build();
        fail2ban.add_prefix(&gtk::Image::from_icon_name("security-medium-symbolic"));
        ssh_group.add(&fail2ban);
        *imp.fail2ban_enabled.borrow_mut() = Some(fail2ban);

        // Security warning
        let ssh_warning = adw::ActionRow::builder()
            .title("Security Recommendation")
            .subtitle("Use SSH keys instead of passwords. Disable root login for better security.")
            .build();
        ssh_warning.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
        ssh_group.add(&ssh_warning);

        content.append(&ssh_group);

        // === VPN Configuration ===
        let vpn_group = adw::PreferencesGroup::builder()
            .title("VPN")
            .description("Configure VPN services")
            .build();

        let tailscale = adw::SwitchRow::builder()
            .title("Tailscale")
            .subtitle("Enable Tailscale mesh VPN service")
            .active(false)
            .build();
        tailscale.add_prefix(&gtk::Image::from_icon_name("network-vpn-symbolic"));
        vpn_group.add(&tailscale);
        *imp.tailscale_enabled.borrow_mut() = Some(tailscale);

        let tailscale_info = adw::ActionRow::builder()
            .title("After enabling")
            .subtitle("Run 'sudo tailscale up' to authenticate and connect to your tailnet")
            .build();
        tailscale_info.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        vpn_group.add(&tailscale_info);

        // Placeholder for WireGuard
        let wireguard_row = adw::ActionRow::builder()
            .title("WireGuard")
            .subtitle("Native kernel VPN - configure manually in configuration.nix")
            .build();
        wireguard_row.add_prefix(&gtk::Image::from_icon_name("network-vpn-symbolic"));
        let wg_status = gtk::Label::builder()
            .label("Manual")
            .css_classes(["dim-label", "caption"])
            .valign(gtk::Align::Center)
            .build();
        wireguard_row.add_suffix(&wg_status);
        vpn_group.add(&wireguard_row);

        content.append(&vpn_group);

        // Note about changes
        let note_group = adw::PreferencesGroup::new();
        let note = adw::ActionRow::builder()
            .title("Note")
            .subtitle("Network changes require a system rebuild to take effect.")
            .build();
        note.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        note_group.add(&note);
        content.append(&note_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn toggle_port(&self, port: u16, enabled: bool) {
        let imp = self.imp();
        let mut ports = imp.allowed_tcp_ports.borrow_mut();

        if enabled {
            if !ports.contains(&port) {
                ports.push(port);
                ports.sort();
            }
        } else {
            ports.retain(|&p| p != port);
        }

        tracing::info!("Open ports updated: {:?}", *ports);
    }

    fn parse_custom_ports(&self, text: &str) {
        let imp = self.imp();
        let mut ports = imp.allowed_tcp_ports.borrow_mut();

        // Parse comma-separated ports
        for part in text.split(',') {
            let trimmed = part.trim();
            if let Ok(port) = trimmed.parse::<u16>() {
                if !ports.contains(&port) {
                    ports.push(port);
                }
            }
        }

        ports.sort();
        ports.dedup();
        tracing::info!("Custom ports parsed: {:?}", *ports);
    }

    /// Get the current network configuration for Nix generation
    pub fn get_network_config(&self) -> NetworkConfig {
        let imp = self.imp();

        let root_login = match imp.ssh_root_login.borrow().as_ref().map(|c| c.selected()) {
            Some(0) => "no",
            Some(1) => "prohibit-password",
            Some(2) => "yes",
            _ => "no",
        };

        NetworkConfig {
            // Firewall
            firewall_enabled: imp.firewall_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(true),
            allowed_tcp_ports: imp.allowed_tcp_ports.borrow().clone(),
            allowed_udp_ports: imp.allowed_udp_ports.borrow().clone(),
            // SSH
            ssh_enabled: imp.ssh_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            ssh_port: imp.ssh_port.borrow().as_ref().map(|s| s.value() as u16).unwrap_or(22),
            ssh_password_auth: imp.ssh_password_auth.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            ssh_root_login: root_login.to_string(),
            fail2ban_enabled: imp.fail2ban_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            // VPN
            tailscale_enabled: imp.tailscale_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
        }
    }

    /// Set network configuration from loaded state
    pub fn set_network_config(&self, config: &NetworkConfig) {
        let imp = self.imp();

        // Firewall
        if let Some(switch) = imp.firewall_enabled.borrow().as_ref() {
            switch.set_active(config.firewall_enabled);
        }
        *imp.allowed_tcp_ports.borrow_mut() = config.allowed_tcp_ports.clone();
        *imp.allowed_udp_ports.borrow_mut() = config.allowed_udp_ports.clone();

        // SSH
        if let Some(switch) = imp.ssh_enabled.borrow().as_ref() {
            switch.set_active(config.ssh_enabled);
        }
        if let Some(spin) = imp.ssh_port.borrow().as_ref() {
            spin.set_value(config.ssh_port as f64);
        }
        if let Some(switch) = imp.ssh_password_auth.borrow().as_ref() {
            switch.set_active(config.ssh_password_auth);
        }
        if let Some(combo) = imp.ssh_root_login.borrow().as_ref() {
            let idx = match config.ssh_root_login.as_str() {
                "no" => 0,
                "prohibit-password" => 1,
                "yes" => 2,
                _ => 0,
            };
            combo.set_selected(idx);
        }
        if let Some(switch) = imp.fail2ban_enabled.borrow().as_ref() {
            switch.set_active(config.fail2ban_enabled);
        }

        // VPN
        if let Some(switch) = imp.tailscale_enabled.borrow().as_ref() {
            switch.set_active(config.tailscale_enabled);
        }
    }
}

impl Default for NetworkPage {
    fn default() -> Self {
        Self::new()
    }
}
