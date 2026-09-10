//! Network page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{theme, Element};

const PRESET_PORTS: [(u16, &str); 4] = [
    (22, "port-ssh"),
    (80, "port-http"),
    (443, "port-https"),
    (8080, "port-alt-http"),
];

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let net = &app.state.network_config;

    let mut chips = widget::row::with_capacity(4)
        .spacing(spacing.space_xs)
        .align_y(Alignment::Center);
    for (port, key) in PRESET_PORTS {
        let selected = net.allowed_tcp_ports.contains(&port);
        let label = match key {
            "port-ssh" => crate::fl!("port-ssh"),
            "port-http" => crate::fl!("port-http"),
            "port-https" => crate::fl!("port-https"),
            _ => crate::fl!("port-alt-http"),
        };
        chips = chips.push(crate::widget::port_chip(
            label,
            selected,
            Message::ToggleTcpPort {
                port,
                enabled: !selected,
            },
        ));
    }

    let root_idx = match net.ssh_root_login.as_str() {
        "prohibit-password" => 1,
        "yes" => 2,
        _ => 0,
    };
    let root_options = vec![
        crate::fl!("root-login-no"),
        crate::fl!("root-login-prohibit"),
        crate::fl!("root-login-yes"),
    ];
    let ssh_port = if net.ssh_port == 0 { 22 } else { net.ssh_port };

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-network-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-network-desc"))
            .width(Length::Fill)
            .into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("network-firewall"),
                Some(crate::fl!("network-firewall-desc")),
            ))
            .add(
                settings::item::builder(crate::fl!("network-firewall-enable"))
                    .description(crate::fl!("network-firewall-enable-desc"))
                    .icon(super::icon("security-high-symbolic"))
                    .toggler(net.firewall_enabled, Message::SetFirewallEnabled),
            )
            .add(
                settings::item::builder(crate::fl!("network-quick-ports"))
                    .description(crate::fl!("network-quick-ports-desc"))
                    .icon(super::icon("network-server-symbolic"))
                    .control(chips),
            )
            .add(
                settings::item::builder(crate::fl!("network-custom-tcp"))
                    .icon(super::icon("network-wired-symbolic"))
                    .control(
                        widget::text_input(
                            crate::fl!("network-custom-tcp-placeholder"),
                            app.custom_tcp_input.as_str(),
                        )
                        .on_input(Message::CustomTcpPortsChanged)
                        .on_submit(Message::CustomTcpPortsChanged),
                    ),
            )
            .add(super::info_item(
                crate::fl!("network-format"),
                crate::fl!("network-format-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("network-ssh"),
                Some(crate::fl!("network-ssh-desc")),
            ))
            .add(
                settings::item::builder(crate::fl!("network-ssh-enable"))
                    .description(crate::fl!("network-ssh-enable-desc"))
                    .icon(super::icon("utilities-terminal-symbolic"))
                    .toggler(net.ssh_enabled, Message::SetSshEnabled),
            )
            .add(
                settings::item::builder(crate::fl!("network-ssh-port"))
                    .description(crate::fl!("network-ssh-port-desc"))
                    .icon(super::icon("network-wired-symbolic"))
                    .control(widget::spin_button(
                        ssh_port.to_string(),
                        crate::fl!("network-ssh-port"),
                        ssh_port,
                        1u16,
                        1u16,
                        65535u16,
                        Message::SetSshPort,
                    )),
            )
            .add(
                settings::item::builder(crate::fl!("network-ssh-password"))
                    .description(crate::fl!("network-ssh-password-desc"))
                    .icon(super::icon("dialog-password-symbolic"))
                    .toggler(net.ssh_password_auth, Message::SetSshPasswordAuth),
            )
            .add(
                settings::item::builder(crate::fl!("network-ssh-root"))
                    .description(crate::fl!("network-ssh-root-desc"))
                    .icon(super::icon("system-users-symbolic"))
                    .control(widget::dropdown(root_options, Some(root_idx), |index| {
                        Message::SetSshRootLogin(index as u8)
                    })),
            )
            .add(
                settings::item::builder(crate::fl!("network-fail2ban"))
                    .description(crate::fl!("network-fail2ban-desc"))
                    .icon(super::icon("security-medium-symbolic"))
                    .toggler(net.fail2ban_enabled, Message::SetFail2banEnabled),
            )
            .add(super::info_item(
                crate::fl!("network-ssh-warning"),
                crate::fl!("network-ssh-warning-desc"),
                "dialog-warning-symbolic",
            ))
            .into(),
        settings::section()
            .title(crate::fl!("network-vpn"))
            .add(
                settings::item::builder(crate::fl!("network-tailscale"))
                    .description(crate::fl!("network-tailscale-desc"))
                    .icon(super::icon("network-vpn-symbolic"))
                    .toggler(net.tailscale_enabled, Message::SetTailscaleEnabled),
            )
            .add(super::info_item(
                crate::fl!("network-tailscale-after"),
                crate::fl!("network-tailscale-after-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
        settings::section()
            .add(super::info_item(
                crate::fl!("network-note"),
                crate::fl!("network-note-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
    ])
    .into()
}
