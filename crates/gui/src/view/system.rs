//! System Settings page.

use crate::app::AppModel;
use crate::config::ColorSchemePreference;
use crate::message::Message;
use common::SystemActionType;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let hostname = app
        .state
        .hostname
        .as_deref()
        .or(app.system_info.hostname.as_deref())
        .unwrap_or("nixos");
    let dns = app.state.dns_servers.join(", ");
    let username = app.state.username.clone().unwrap_or_else(|| {
        std::env::var("USER").unwrap_or_else(|_| crate::fl!("system-username-default"))
    });

    let style_idx = match app.prefs.color_scheme {
        ColorSchemePreference::System => 0,
        ColorSchemePreference::Light => 1,
        ColorSchemePreference::Dark => 2,
    };
    let style_options = vec![
        crate::fl!("style-system"),
        crate::fl!("style-light"),
        crate::fl!("style-dark"),
    ];

    let mut groups = settings::section().title(crate::fl!("system-groups"));
    groups = groups.add(
        settings::item::builder(crate::fl!("system-username"))
            .icon(super::icon("avatar-default-symbolic"))
            .control(
                widget::text_input(crate::fl!("system-username-placeholder"), username)
                    .on_input(Message::UsernameChanged)
                    .on_submit(Message::UsernameChanged),
            ),
    );
    for action in super::catalog_system_actions() {
        let SystemActionType::UserGroup { group } = &action.action_type else {
            continue;
        };
        let enabled = app.state.is_in_group(group);
        let group = group.clone();
        groups = groups.add(
            settings::item::builder(action.name.as_str())
                .description(action.description.as_str())
                .icon(super::icon(action.icon.as_str()))
                .toggler(enabled, move |enabled| Message::ToggleUserGroup {
                    group: group.clone(),
                    enabled,
                }),
        );
    }
    groups = groups.add(super::info_item(
        crate::fl!("system-groups-note"),
        crate::fl!("system-groups-note-desc"),
        "dialog-information-symbolic",
    ));

    let mut dns_item = settings::item::builder(crate::fl!("system-dns-servers"))
        .icon(super::icon("network-server-symbolic"));
    if let Some(error) = app.field_errors.dns.as_deref() {
        dns_item = dns_item.description(error.to_owned());
    }
    let dns_row = dns_item.control(
        widget::text_input(crate::fl!("system-dns-placeholder"), dns)
            .on_input(Message::DnsServersChanged)
            .on_submit(Message::DnsServersChanged),
    );

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-system-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-system-desc"))
            .width(Length::Fill)
            .into(),
        settings::section()
            .title(crate::fl!("system-appearance"))
            .add(
                settings::item::builder(crate::fl!("system-style"))
                    .description(crate::fl!("system-style-desc"))
                    .icon(super::icon("weather-clear-symbolic"))
                    .control(widget::dropdown(style_options, Some(style_idx), |index| {
                        Message::ColorSchemeChanged(match index {
                            1 => ColorSchemePreference::Light,
                            2 => ColorSchemePreference::Dark,
                            _ => ColorSchemePreference::System,
                        })
                    })),
            )
            .into(),
        settings::section()
            .title(crate::fl!("system-identity"))
            .add(
                settings::item::builder(crate::fl!("system-hostname"))
                    .icon(super::icon("computer-symbolic"))
                    .control(
                        widget::text_input(crate::fl!("system-hostname-placeholder"), hostname)
                            .on_input(Message::HostnameChanged)
                            .on_submit(Message::HostnameChanged),
                    ),
            )
            .add(super::info_item(
                crate::fl!("system-hostname-note"),
                crate::fl!("system-hostname-note-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("system-dns"),
                Some(crate::fl!("system-dns-desc")),
            ))
            .add(dns_row)
            .add(super::info_item(
                crate::fl!("system-dns-format"),
                crate::fl!("system-dns-format-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
        groups.into(),
    ])
    .into()
}
