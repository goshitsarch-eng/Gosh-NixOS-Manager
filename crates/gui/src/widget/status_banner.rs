//! Full-width status bar. Not dismissible (GTK banners have no close button).

use crate::app::BannerKind;
use cosmic::iced::{Alignment, Background, Border, Color, Length, Shadow};
use cosmic::widget::{self, container};
use cosmic::{theme, Apply, Element};
use std::borrow::Cow;

/// Error / warning / success / info banner. No close button.
#[must_use]
pub fn status_banner<'a, Message: 'static>(
    kind: BannerKind,
    text: impl Into<Cow<'a, str>> + 'a,
) -> Element<'a, Message> {
    let spacing = theme::spacing();
    let icon = widget::icon::from_name(banner_icon(kind)).size(16);
    let label = widget::text::body(text).width(Length::Fill);

    widget::row::with_capacity(2)
        .push(icon)
        .push(label)
        .spacing(spacing.space_xs)
        .align_y(Alignment::Center)
        .apply(container)
        .class(theme::Container::custom(banner_style(kind)))
        .padding(spacing.space_xs)
        .width(Length::Fill)
        .into()
}

fn banner_icon(kind: BannerKind) -> &'static str {
    match kind {
        BannerKind::Error => "dialog-error-symbolic",
        BannerKind::Warning => "dialog-warning-symbolic",
        BannerKind::Success => "emblem-ok-symbolic",
        BannerKind::Info => "dialog-information-symbolic",
    }
}

fn banner_style(kind: BannerKind) -> fn(&cosmic::Theme) -> widget::container::Style {
    match kind {
        BannerKind::Error => error_container,
        BannerKind::Warning => warning_container,
        BannerKind::Success => success_container,
        BannerKind::Info => info_container,
    }
}

fn error_container(theme: &cosmic::Theme) -> widget::container::Style {
    tone_container(
        theme,
        theme.cosmic().destructive_color(),
        theme.cosmic().destructive.on,
    )
}

fn warning_container(theme: &cosmic::Theme) -> widget::container::Style {
    tone_container(
        theme,
        theme.cosmic().warning_color(),
        theme.cosmic().warning.on,
    )
}

fn success_container(theme: &cosmic::Theme) -> widget::container::Style {
    tone_container(
        theme,
        theme.cosmic().success_color(),
        theme.cosmic().success.on,
    )
}

fn info_container(theme: &cosmic::Theme) -> widget::container::Style {
    tone_container(
        theme,
        theme.cosmic().accent_color(),
        theme.cosmic().accent.on,
    )
}

fn tone_container(
    theme: &cosmic::Theme,
    background: impl Into<Color>,
    on: impl Into<Color>,
) -> widget::container::Style {
    let cosmic = theme.cosmic();
    let on = on.into();
    widget::container::Style {
        icon_color: Some(on),
        text_color: Some(on),
        background: Some(Background::Color(background.into())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: cosmic.corner_radii.radius_0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}
