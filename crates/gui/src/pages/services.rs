//! Services page for toggling common NixOS services

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::collections::HashMap;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ServicesPage {
        pub service_switches: RefCell<HashMap<String, adw::SwitchRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServicesPage {
        const NAME: &'static str = "NixosToolkitServicesPage";
        type Type = super::ServicesPage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for ServicesPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for ServicesPage {}
    impl BoxImpl for ServicesPage {}
}

glib::wrapper! {
    pub struct ServicesPage(ObjectSubclass<imp::ServicesPage>)
        @extends gtk::Box, gtk::Widget;
}

impl ServicesPage {
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
            .label("System Services")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Enable or disable common system services. Changes require a system rebuild.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // === Hardware Services ===
        let hardware_group = adw::PreferencesGroup::builder()
            .title("Hardware Services")
            .description("Services for hardware support and drivers")
            .build();

        let hardware_services = [
            ServiceDef {
                id: "printing",
                name: "Printing (CUPS)",
                description: "Enable printing support via CUPS",
                icon: "printer-symbolic",
                nix_option: "services.printing.enable",
            },
            ServiceDef {
                id: "avahi",
                name: "Avahi/mDNS",
                description: "Enable network service discovery (Bonjour compatible)",
                icon: "network-workgroup-symbolic",
                nix_option: "services.avahi.enable",
            },
            ServiceDef {
                id: "fwupd",
                name: "Firmware Updates",
                description: "Enable fwupd for firmware updates (LVFS)",
                icon: "software-update-available-symbolic",
                nix_option: "services.fwupd.enable",
            },
            ServiceDef {
                id: "upower",
                name: "UPower",
                description: "Power management service for laptops",
                icon: "battery-symbolic",
                nix_option: "services.upower.enable",
            },
        ];

        for service in &hardware_services {
            let row = self.create_service_row(service);
            hardware_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        content.append(&hardware_group);

        // === Network Services ===
        let network_group = adw::PreferencesGroup::builder()
            .title("Network Services")
            .description("Networking and connectivity services")
            .build();

        let network_services = [
            ServiceDef {
                id: "networkmanager",
                name: "NetworkManager",
                description: "Modern network configuration manager",
                icon: "network-wired-symbolic",
                nix_option: "networking.networkmanager.enable",
            },
            ServiceDef {
                id: "resolved",
                name: "systemd-resolved",
                description: "System DNS resolver with caching",
                icon: "network-server-symbolic",
                nix_option: "services.resolved.enable",
            },
        ];

        for service in &network_services {
            let row = self.create_service_row(service);
            network_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        content.append(&network_group);

        // === Remote Access Services ===
        let remote_group = adw::PreferencesGroup::builder()
            .title("Remote Access")
            .description("Remote desktop and access services")
            .build();

        let remote_services = [
            ServiceDef {
                id: "rustdesk",
                name: "RustDesk",
                description: "Open-source remote desktop (like TeamViewer/AnyDesk)",
                icon: "computer-symbolic",
                nix_option: "services.rustdesk-server.enable",
            },
        ];

        for service in &remote_services {
            let row = self.create_service_row(service);
            remote_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        // RustDesk info
        let rustdesk_info = adw::ActionRow::builder()
            .title("RustDesk Client")
            .subtitle("Install rustdesk package for the client app")
            .build();
        rustdesk_info.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        remote_group.add(&rustdesk_info);

        content.append(&remote_group);

        // === Sync & Backup Services ===
        let sync_group = adw::PreferencesGroup::builder()
            .title("Sync & Backup")
            .description("File synchronization and backup services")
            .build();

        let sync_services = [
            ServiceDef {
                id: "syncthing",
                name: "Syncthing",
                description: "Continuous file synchronization (runs as user service)",
                icon: "emblem-synchronizing-symbolic",
                nix_option: "services.syncthing.enable",
            },
            ServiceDef {
                id: "locate",
                name: "Locate Database",
                description: "Enable mlocate/plocate for fast file searching",
                icon: "system-search-symbolic",
                nix_option: "services.locate.enable",
            },
        ];

        for service in &sync_services {
            let row = self.create_service_row(service);
            sync_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        content.append(&sync_group);

        // === Desktop Services ===
        let desktop_group = adw::PreferencesGroup::builder()
            .title("Desktop Services")
            .description("Services for desktop environments")
            .build();

        let desktop_services = [
            ServiceDef {
                id: "flatpak",
                name: "Flatpak",
                description: "Enable Flatpak application support",
                icon: "package-x-generic-symbolic",
                nix_option: "services.flatpak.enable",
            },
            ServiceDef {
                id: "gnome_keyring",
                name: "GNOME Keyring",
                description: "Secure storage for passwords and keys",
                icon: "channel-secure-symbolic",
                nix_option: "services.gnome.gnome-keyring.enable",
            },
            ServiceDef {
                id: "gnome_tweaks",
                name: "GNOME Tweaks",
                description: "Advanced GNOME desktop customization tool",
                icon: "preferences-other-symbolic",
                nix_option: "environment.systemPackages.gnome-tweaks",
            },
            ServiceDef {
                id: "dconf",
                name: "dconf",
                description: "Configuration system for GNOME/GTK apps",
                icon: "preferences-system-symbolic",
                nix_option: "programs.dconf.enable",
            },
        ];

        for service in &desktop_services {
            let row = self.create_service_row(service);
            desktop_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        content.append(&desktop_group);

        // === Development Services ===
        let dev_group = adw::PreferencesGroup::builder()
            .title("Development")
            .description("Services for software development")
            .build();

        let dev_services = [
            ServiceDef {
                id: "docker",
                name: "Docker",
                description: "Container runtime daemon",
                icon: "application-x-executable-symbolic",
                nix_option: "virtualisation.docker.enable",
            },
            ServiceDef {
                id: "libvirtd",
                name: "libvirtd",
                description: "Virtualization management daemon for KVM/QEMU",
                icon: "computer-symbolic",
                nix_option: "virtualisation.libvirtd.enable",
            },
            ServiceDef {
                id: "postgresql",
                name: "PostgreSQL",
                description: "PostgreSQL database server",
                icon: "drive-harddisk-symbolic",
                nix_option: "services.postgresql.enable",
            },
            ServiceDef {
                id: "redis",
                name: "Redis",
                description: "In-memory data structure store",
                icon: "drive-harddisk-symbolic",
                nix_option: "services.redis.servers.\"\".enable",
            },
        ];

        for service in &dev_services {
            let row = self.create_service_row(service);
            dev_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        content.append(&dev_group);

        // === System Services ===
        let system_group = adw::PreferencesGroup::builder()
            .title("System")
            .description("Core system services")
            .build();

        let system_services = [
            ServiceDef {
                id: "earlyoom",
                name: "Early OOM",
                description: "Kill processes early when system runs low on memory",
                icon: "dialog-warning-symbolic",
                nix_option: "services.earlyoom.enable",
            },
            ServiceDef {
                id: "auto_upgrade",
                name: "Auto Upgrade",
                description: "Automatically upgrade NixOS (use with caution)",
                icon: "software-update-available-symbolic",
                nix_option: "system.autoUpgrade.enable",
            },
            ServiceDef {
                id: "auto_gc",
                name: "Auto Garbage Collect",
                description: "Automatically clean up old Nix store paths",
                icon: "user-trash-symbolic",
                nix_option: "nix.gc.automatic",
            },
            ServiceDef {
                id: "store_optimize",
                name: "Store Optimization",
                description: "Automatically optimize Nix store (deduplication)",
                icon: "drive-harddisk-symbolic",
                nix_option: "nix.settings.auto-optimise-store",
            },
        ];

        for service in &system_services {
            let row = self.create_service_row(service);
            system_group.add(&row);
            imp.service_switches.borrow_mut().insert(service.id.to_string(), row);
        }

        content.append(&system_group);

        // Note
        let note_group = adw::PreferencesGroup::new();
        let note = adw::ActionRow::builder()
            .title("Note")
            .subtitle("Service changes require a system rebuild. Some services may require additional configuration.")
            .build();
        note.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        note_group.add(&note);
        content.append(&note_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn create_service_row(&self, service: &ServiceDef) -> adw::SwitchRow {
        let row = adw::SwitchRow::builder()
            .title(service.name)
            .subtitle(service.description)
            .active(false)
            .build();
        row.add_prefix(&gtk::Image::from_icon_name(service.icon));

        // Add NixOS option as tooltip
        row.set_tooltip_text(Some(&format!("NixOS option: {}", service.nix_option)));

        let service_id = service.id.to_string();
        row.connect_active_notify(move |switch| {
            tracing::info!("Service {} set to: {}", service_id, switch.is_active());
        });

        row
    }

    /// Get all enabled services
    pub fn get_enabled_services(&self) -> Vec<String> {
        let imp = self.imp();
        imp.service_switches
            .borrow()
            .iter()
            .filter(|(_, switch)| switch.is_active())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get service configuration for Nix generation
    pub fn get_services_config(&self) -> ServicesConfig {
        let imp = self.imp();
        let switches = imp.service_switches.borrow();

        ServicesConfig {
            printing: switches.get("printing").map(|s| s.is_active()).unwrap_or(false),
            avahi: switches.get("avahi").map(|s| s.is_active()).unwrap_or(false),
            fwupd: switches.get("fwupd").map(|s| s.is_active()).unwrap_or(false),
            upower: switches.get("upower").map(|s| s.is_active()).unwrap_or(false),
            networkmanager: switches.get("networkmanager").map(|s| s.is_active()).unwrap_or(false),
            resolved: switches.get("resolved").map(|s| s.is_active()).unwrap_or(false),
            rustdesk: switches.get("rustdesk").map(|s| s.is_active()).unwrap_or(false),
            syncthing: switches.get("syncthing").map(|s| s.is_active()).unwrap_or(false),
            locate: switches.get("locate").map(|s| s.is_active()).unwrap_or(false),
            flatpak: switches.get("flatpak").map(|s| s.is_active()).unwrap_or(false),
            gnome_keyring: switches.get("gnome_keyring").map(|s| s.is_active()).unwrap_or(false),
            gnome_tweaks: switches.get("gnome_tweaks").map(|s| s.is_active()).unwrap_or(false),
            dconf: switches.get("dconf").map(|s| s.is_active()).unwrap_or(false),
            docker: switches.get("docker").map(|s| s.is_active()).unwrap_or(false),
            libvirtd: switches.get("libvirtd").map(|s| s.is_active()).unwrap_or(false),
            postgresql: switches.get("postgresql").map(|s| s.is_active()).unwrap_or(false),
            redis: switches.get("redis").map(|s| s.is_active()).unwrap_or(false),
            earlyoom: switches.get("earlyoom").map(|s| s.is_active()).unwrap_or(false),
            auto_upgrade: switches.get("auto_upgrade").map(|s| s.is_active()).unwrap_or(false),
            auto_gc: switches.get("auto_gc").map(|s| s.is_active()).unwrap_or(false),
            store_optimize: switches.get("store_optimize").map(|s| s.is_active()).unwrap_or(false),
        }
    }
}

impl Default for ServicesPage {
    fn default() -> Self {
        Self::new()
    }
}

/// Definition of a service for the UI
struct ServiceDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    nix_option: &'static str,
}

/// Services configuration state
#[derive(Debug, Clone, Default)]
pub struct ServicesConfig {
    pub printing: bool,
    pub avahi: bool,
    pub fwupd: bool,
    pub upower: bool,
    pub networkmanager: bool,
    pub resolved: bool,
    pub rustdesk: bool,
    pub syncthing: bool,
    pub locate: bool,
    pub flatpak: bool,
    pub gnome_keyring: bool,
    pub gnome_tweaks: bool,
    pub dconf: bool,
    pub docker: bool,
    pub libvirtd: bool,
    pub postgresql: bool,
    pub redis: bool,
    pub earlyoom: bool,
    pub auto_upgrade: bool,
    pub auto_gc: bool,
    pub store_optimize: bool,
}
