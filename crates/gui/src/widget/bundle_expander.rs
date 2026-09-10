//! ExpanderRow + enable-switch replacement (libcosmic has no expander).

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::{theme, Apply, Element};
use std::borrow::Cow;

/// Bundle header (icon, title, count, toggler, chevron) with an optional body.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn bundle_expander<'a, Message: Clone + 'static>(
    icon_name: impl Into<Cow<'a, str>>,
    title: impl Into<Cow<'a, str>>,
    subtitle: impl Into<Cow<'a, str>>,
    package_count: impl Into<Cow<'a, str>>,
    enabled: bool,
    expanded: bool,
    sensitive: bool,
    compat_icon: Option<&'a str>,
    on_toggle: impl Fn(bool) -> Message + 'a,
    on_expand: Message,
    body: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let spacing = theme::spacing();
    let icon_name = icon_name.into();
    let title: Cow<'a, str> = title.into();
    let subtitle: Cow<'a, str> = subtitle.into();
    let package_count: Cow<'a, str> = package_count.into();
    let chevron = if expanded {
        "pan-up-symbolic"
    } else {
        "pan-down-symbolic"
    };

    let mut header = widget::row::with_capacity(6)
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .push(widget::icon::from_name(icon_name.into_owned()).size(16));

    if let Some(name) = compat_icon {
        header = header.push(widget::icon::from_name(name).size(16));
    }

    header = header
        .push(
            widget::column::with_capacity(2)
                .spacing(spacing.space_xxxs)
                .push(widget::text::body(title))
                .push(widget::text::caption(subtitle))
                .width(Length::Fill),
        )
        .push(widget::text::caption(package_count))
        .push(
            widget::toggler(enabled)
                .width(Length::Shrink)
                .on_toggle_maybe(sensitive.then_some(on_toggle)),
        )
        .push(
            widget::button::icon(widget::icon::from_name(chevron))
                .on_press_maybe(sensitive.then_some(on_expand)),
        );

    let mut inner = widget::column::with_capacity(2)
        .spacing(spacing.space_xs)
        .push(header);

    if sensitive {
        if let Some(body) = body {
            inner = inner.push(body);
        }
    }

    inner
        .apply(container)
        .class(theme::Container::Card)
        .padding(spacing.space_s)
        .width(Length::Fill)
        .into()
}
