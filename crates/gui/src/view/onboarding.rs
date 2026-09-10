//! Getting Started page.

use crate::app::AppModel;
use crate::integration::{classic_integration_snippet, flake_integration_snippet};
use crate::message::Message;
use common::{ConfigMode, IntegrationStatus};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{theme, Element};

/// GTK snippet TextView was 280px tall.
const SNIPPET_HEIGHT: f32 = 280.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let info = &app.system_info;
    let (status_title, status_sub) = if info.is_nixos {
        let integration = match info.integration_status {
            IntegrationStatus::Integrated => crate::fl!("integration-integrated"),
            IntegrationStatus::NotIntegrated => crate::fl!("integration-not-integrated"),
            IntegrationStatus::Unknown => crate::fl!("integration-unknown"),
        };
        let version = info
            .nixos_version
            .as_deref()
            .map_or_else(|| crate::fl!("onboarding-version-unknown"), str::to_owned);
        (
            format!(
                "{} ({})",
                crate::fl!("onboarding-nixos-detected"),
                info.config_mode.display_name()
            ),
            format!(
                "{}: {} | {}: {}",
                crate::fl!("onboarding-integration"),
                integration,
                crate::fl!("onboarding-version"),
                version
            ),
        )
    } else {
        (
            crate::fl!("onboarding-not-nixos"),
            crate::fl!("onboarding-not-nixos-sub"),
        )
    };

    let snippet = match info.config_mode {
        ConfigMode::Flake => flake_integration_snippet(),
        ConfigMode::Classic | ConfigMode::Unknown => classic_integration_snippet(),
    };

    let buttons = widget::row::with_capacity(3)
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .push(
            widget::button::suggested(crate::fl!("copy-snippet"))
                .on_press(Message::CopyIntegrationSnippet),
        )
        .push(
            widget::button::standard(crate::fl!("open-nixos-dir")).on_press(Message::OpenEtcNixos),
        )
        .push(
            widget::button::standard(crate::fl!("verify-integration"))
                .on_press(Message::VerifyIntegration),
        );

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-onboarding-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-onboarding-desc"))
            .width(Length::Fill)
            .into(),
        settings::section()
            .title(crate::fl!("onboarding-status"))
            .add(
                settings::item::builder(status_title)
                    .description(status_sub)
                    .icon(super::icon("emblem-system-symbolic"))
                    .control(widget::space::horizontal()),
            )
            .into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("onboarding-setup"),
                Some(crate::fl!("onboarding-setup-desc")),
            ))
            .add(crate::widget::code_view(snippet, SNIPPET_HEIGHT))
            .add(buttons)
            .into(),
    ])
    .into()
}
