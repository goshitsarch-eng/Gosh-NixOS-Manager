//! Network page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{theme, Element};

const PRESET_TCP: [(u16, &str); 4] = [
    (22, "port-ssh"),
    (80, "port-http"),
    (443, "port-https"),
    (8080, "port-alt-http"),
];

const PRESET_UDP: [(u16, &str); 4] = [
    (53, "port-dns"),
    (123, "port-ntp"),
    (443, "port-https-quic"),
    (51820, "port-wireguard"),
];

fn preset_label(key: &str) -> String {
    match key {
        "port-ssh" => crate::fl!("port-ssh"),
        "port-http" => crate::fl!("port-http"),
        "port-https" => crate::fl!("port-https"),
        "port-alt-http" => crate::fl!("port-alt-http"),
        "port-dns" => crate::fl!("port-dns"),
        "port-ntp" => crate::fl!("port-ntp"),
        "port-https-quic" => crate::fl!("port-https-quic"),
        _ => crate::fl!("port-wireguard"),
    }
}

fn port_chips<'a>(
    selected: &'a [u16],
    presets: &'a [(u16, &'static str)],
    to_message: impl Fn(u16, bool) -> Message + Copy + 'a,
) -> cosmic::widget::Row<'a, Message, cosmic::Theme> {
    let spacing = theme::spacing();
    let mut chips = widget::row::with_capacity(presets.len())
        .spacing(spacing.space_xs)
        .align_y(Alignment::Center);
    for (port, key) in presets {
        let is_selected = selected.contains(port);
        chips = chips.push(crate::widget::port_chip(
            preset_label(key),
            is_selected,
            to_message(*port, !is_selected),
        ));
    }
    chips
}

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let net = &app.state.network_config;

    let tcp_chips = port_chips(&net.allowed_tcp_ports, &PRESET_TCP, |port, enabled| {
        Message::ToggleTcpPort { port, enabled }
    });
    let udp_chips = port_chips(&net.allowed_udp_ports, &PRESET_UDP, |port, enabled| {
        Message::ToggleUdpPort { port, enabled }
    });

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
    let wg_port = app.state.wireguard_listen_port();

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
                    .control(tcp_chips),
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
            .add(
                settings::item::builder(crate::fl!("network-quick-udp"))
                    .description(crate::fl!("network-quick-udp-desc"))
                    .icon(super::icon("network-workgroup-symbolic"))
                    .control(udp_chips),
            )
            .add(
                settings::item::builder(crate::fl!("network-custom-udp"))
                    .icon(super::icon("network-wireless-symbolic"))
                    .control(
                        widget::text_input(
                            crate::fl!("network-custom-udp-placeholder"),
                            app.custom_udp_input.as_str(),
                        )
                        .on_input(Message::CustomUdpPortsChanged)
                        .on_submit(Message::CustomUdpPortsChanged),
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
            .add(
                settings::item::builder(crate::fl!("network-wireguard"))
                    .description(crate::fl!("network-wireguard-desc"))
                    .icon(super::icon("network-vpn-symbolic"))
                    .toggler(net.wireguard_enabled, Message::SetWireguardEnabled),
            )
            .add(
                settings::item::builder(crate::fl!("network-wireguard-port"))
                    .description(crate::fl!("network-wireguard-port-desc"))
                    .icon(super::icon("network-wired-symbolic"))
                    .control(widget::spin_button(
                        wg_port.to_string(),
                        crate::fl!("network-wireguard-port"),
                        wg_port,
                        1u16,
                        1u16,
                        65535u16,
                        Message::SetWireguardListenPort,
                    )),
            )
            .add(super::info_item(
                crate::fl!("network-wireguard-info"),
                crate::fl!("network-wireguard-info-desc"),
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
