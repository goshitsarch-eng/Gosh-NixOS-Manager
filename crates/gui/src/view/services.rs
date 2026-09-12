//! Services page.

use crate::app::AppModel;
use crate::message::Message;
use common::ipc::ServicesConfig;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

struct ServiceDef {
    id: &'static str,
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
            let enabled = service_enabled(config, service.id);
            let id = service.id.to_owned();
            section = section.add(
                settings::item::builder(service_name(service.id))
                    .description(format!(
                        "{} — {}",
                        service_description(service.id),
                        crate::fl!("service-nix-option", option = service.nix_option)
                    ))
                    .icon(super::icon(service.icon))
                    .toggler(enabled, move |enabled| Message::ToggleService {
                        id: id.clone(),
                        enabled,
                    }),
            );
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

fn service_name(id: &str) -> String {
    match id {
        "printing" => crate::fl!("service-printing"),
        "avahi" => crate::fl!("service-avahi"),
        "fwupd" => crate::fl!("service-fwupd"),
        "upower" => crate::fl!("service-upower"),
        "networkmanager" => crate::fl!("service-networkmanager"),
        "resolved" => crate::fl!("service-resolved"),
        "rustdesk" => crate::fl!("service-rustdesk"),
        "syncthing" => crate::fl!("service-syncthing"),
        "locate" => crate::fl!("service-locate"),
        "flatpak" => crate::fl!("service-flatpak"),
        "gnome_keyring" => crate::fl!("service-gnome-keyring"),
        "gnome_tweaks" => crate::fl!("service-gnome-tweaks"),
        "dconf" => crate::fl!("service-dconf"),
        "docker" => crate::fl!("service-docker"),
        "libvirtd" => crate::fl!("service-libvirtd"),
        "postgresql" => crate::fl!("service-postgresql"),
        "redis" => crate::fl!("service-redis"),
        "earlyoom" => crate::fl!("service-earlyoom"),
        "auto_upgrade" => crate::fl!("service-auto-upgrade"),
        "auto_gc" => crate::fl!("service-auto-gc"),
        "store_optimize" => crate::fl!("service-store-optimize"),
        other => other.to_string(),
    }
}

fn service_description(id: &str) -> String {
    match id {
        "printing" => crate::fl!("service-printing-desc"),
        "avahi" => crate::fl!("service-avahi-desc"),
        "fwupd" => crate::fl!("service-fwupd-desc"),
        "upower" => crate::fl!("service-upower-desc"),
        "networkmanager" => crate::fl!("service-networkmanager-desc"),
        "resolved" => crate::fl!("service-resolved-desc"),
        "rustdesk" => crate::fl!("service-rustdesk-desc"),
        "syncthing" => crate::fl!("service-syncthing-desc"),
        "locate" => crate::fl!("service-locate-desc"),
        "flatpak" => crate::fl!("service-flatpak-desc"),
        "gnome_keyring" => crate::fl!("service-gnome-keyring-desc"),
        "gnome_tweaks" => crate::fl!("service-gnome-tweaks-desc"),
        "dconf" => crate::fl!("service-dconf-desc"),
        "docker" => crate::fl!("service-docker-desc"),
        "libvirtd" => crate::fl!("service-libvirtd-desc"),
        "postgresql" => crate::fl!("service-postgresql-desc"),
        "redis" => crate::fl!("service-redis-desc"),
        "earlyoom" => crate::fl!("service-earlyoom-desc"),
        "auto_upgrade" => crate::fl!("service-auto-upgrade-desc"),
        "auto_gc" => crate::fl!("service-auto-gc-desc"),
        "store_optimize" => crate::fl!("service-store-optimize-desc"),
        _ => String::new(),
    }
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
                    icon: "printer-symbolic",
                    nix_option: "services.printing.enable",
                },
                ServiceDef {
                    id: "avahi",
                    icon: "network-workgroup-symbolic",
                    nix_option: "services.avahi.enable",
                },
                ServiceDef {
                    id: "fwupd",
                    icon: "software-update-available-symbolic",
                    nix_option: "services.fwupd.enable",
                },
                ServiceDef {
                    id: "upower",
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
                    icon: "network-wired-symbolic",
                    nix_option: "networking.networkmanager.enable",
                },
                ServiceDef {
                    id: "resolved",
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
                icon: "computer-symbolic",
                nix_option: "environment.systemPackages (rustdesk)",
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
                    icon: "emblem-synchronizing-symbolic",
                    nix_option: "services.syncthing.enable",
                },
                ServiceDef {
                    id: "locate",
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
                    icon: "package-x-generic-symbolic",
                    nix_option: "services.flatpak.enable",
                },
                ServiceDef {
                    id: "gnome_keyring",
                    icon: "channel-secure-symbolic",
                    nix_option: "services.gnome.gnome-keyring.enable",
                },
                ServiceDef {
                    id: "gnome_tweaks",
                    icon: "preferences-other-symbolic",
                    nix_option: "environment.systemPackages.gnome-tweaks",
                },
                ServiceDef {
                    id: "dconf",
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
                    icon: "application-x-executable-symbolic",
                    nix_option: "virtualisation.docker.enable",
                },
                ServiceDef {
                    id: "libvirtd",
                    icon: "computer-symbolic",
                    nix_option: "virtualisation.libvirtd.enable",
                },
                ServiceDef {
                    id: "postgresql",
                    icon: "drive-harddisk-symbolic",
                    nix_option: "services.postgresql.enable",
                },
                ServiceDef {
                    id: "redis",
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
                    icon: "dialog-warning-symbolic",
                    nix_option: "services.earlyoom.enable",
                },
                ServiceDef {
                    id: "auto_upgrade",
                    icon: "software-update-available-symbolic",
                    nix_option: "system.autoUpgrade.enable",
                },
                ServiceDef {
                    id: "auto_gc",
                    icon: "user-trash-symbolic",
                    nix_option: "nix.gc.automatic",
                },
                ServiceDef {
                    id: "store_optimize",
                    icon: "drive-harddisk-symbolic",
                    nix_option: "nix.settings.auto-optimise-store",
                },
            ],
            extra: None,
        },
    ]
}
