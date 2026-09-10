//! Services page.

use crate::app::AppModel;
use crate::message::Message;
use common::ipc::ServicesConfig;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

struct ServiceDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    nix_option: &'static str,
}

struct ServiceGroup {
    title: String,
    description: String,
    services: &'static [ServiceDef],
    extra: Option<(String, String, &'static str)>,
}

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let config = &app.state.services_config;
    let mut children = vec![
        widget::text::title2(crate::fl!("page-services-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-services-desc"))
            .width(Length::Fill)
            .into(),
    ];

    for group in service_groups() {
        let mut section =
            settings::section().header(super::section_header(group.title, Some(group.description)));
        for service in group.services {
            section = section.add(service_row(service, config));
        }
        if let Some((title, description, icon)) = group.extra {
            section = section.add(super::info_item(title, description, icon));
        }
        children.push(section.into());
    }

    children.push(
        settings::section()
            .add(super::info_item(
                crate::fl!("services-note"),
                crate::fl!("services-note-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
    );

    settings::view_column(children).into()
}

fn service_row<'a>(service: &ServiceDef, config: &ServicesConfig) -> Element<'a, Message> {
    let enabled = service_enabled(config, service.id);
    let id = service.id.to_owned();
    widget::tooltip(
        settings::item::builder(service.name)
            .description(service.description)
            .icon(super::icon(service.icon))
            .control(
                widget::toggler(enabled)
                    .width(Length::Shrink)
                    .on_toggle(move |enabled| Message::ToggleService {
                        id: id.clone(),
                        enabled,
                    }),
            ),
        widget::text::caption(crate::fl!(
            "service-nix-option",
            option = service.nix_option
        )),
        widget::tooltip::Position::Bottom,
    )
    .into()
}

fn service_enabled(config: &ServicesConfig, id: &str) -> bool {
    match id {
        "printing" => config.printing,
        "avahi" => config.avahi,
        "fwupd" => config.fwupd,
        "upower" => config.upower,
        "networkmanager" => config.networkmanager,
        "resolved" => config.resolved,
        "rustdesk" => config.rustdesk,
        "syncthing" => config.syncthing,
        "locate" => config.locate,
        "flatpak" => config.flatpak,
        "gnome_keyring" => config.gnome_keyring,
        "gnome_tweaks" => config.gnome_tweaks,
        "dconf" => config.dconf,
        "docker" => config.docker,
        "libvirtd" => config.libvirtd,
        "postgresql" => config.postgresql,
        "redis" => config.redis,
        "earlyoom" => config.earlyoom,
        "auto_upgrade" => config.auto_upgrade,
        "auto_gc" => config.auto_gc,
        "store_optimize" => config.store_optimize,
        _ => false,
    }
}

fn service_groups() -> [ServiceGroup; 7] {
    [
        ServiceGroup {
            title: crate::fl!("services-hardware"),
            description: crate::fl!("services-hardware-desc"),
            services: &[
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
            ],
            extra: None,
        },
        ServiceGroup {
            title: crate::fl!("services-network"),
            description: crate::fl!("services-network-desc"),
            services: &[
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
            ],
            extra: None,
        },
        ServiceGroup {
            title: crate::fl!("services-remote"),
            description: crate::fl!("services-remote-desc"),
            services: &[ServiceDef {
                id: "rustdesk",
                name: "RustDesk",
                description: "Open-source remote desktop (like TeamViewer/AnyDesk)",
                icon: "computer-symbolic",
                nix_option: "services.rustdesk-server.enable",
            }],
            extra: Some((
                crate::fl!("services-rustdesk-client"),
                crate::fl!("services-rustdesk-client-desc"),
                "dialog-information-symbolic",
            )),
        },
        ServiceGroup {
            title: crate::fl!("services-sync"),
            description: crate::fl!("services-sync-desc"),
            services: &[
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
            ],
            extra: None,
        },
        ServiceGroup {
            title: crate::fl!("services-desktop"),
            description: crate::fl!("services-desktop-desc"),
            services: &[
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
            ],
            extra: None,
        },
        ServiceGroup {
            title: crate::fl!("services-dev"),
            description: crate::fl!("services-dev-desc"),
            services: &[
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
                    nix_option: r#"services.redis.servers."".enable"#,
                },
            ],
            extra: None,
        },
        ServiceGroup {
            title: crate::fl!("services-system"),
            description: crate::fl!("services-system-desc"),
            services: &[
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
            ],
            extra: None,
        },
    ]
}
