//! Dim empty-state copy (GTK `.dim-label` ActionRow).

use cosmic::iced::Length;
use cosmic::widget;
use cosmic::{theme, Element};
use std::borrow::Cow;

/// Title + caption used when a list has no rows.
#[must_use]
pub fn empty_placeholder<'a, Message: 'static>(
    title: impl Into<Cow<'a, str>> + 'a,
    description: impl Into<Cow<'a, str>> + 'a,
) -> Element<'a, Message> {
    let spacing = theme::spacing();
    widget::column::with_capacity(2)
        .push(widget::text::body(title).width(Length::Fill))
        .push(widget::text::caption(description).width(Length::Fill))
        .spacing(spacing.space_xxs)
        .width(Length::Fill)
        .into()
}
