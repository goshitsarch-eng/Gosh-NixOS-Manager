//! Read-only selectable monospace text in a card. Not `text_editor`.

use cosmic::iced::widget::text::Wrapping;
use cosmic::iced::Length;
use cosmic::widget::{self, container};
use cosmic::{theme, Apply, Element};
use std::borrow::Cow;

/// Selectable monospace text in a scrollable card.
#[must_use]
pub fn code_view<'a, Message: Clone + 'static>(
    contents: impl Into<Cow<'a, str>> + 'a,
    height: f32,
) -> Element<'a, Message> {
    let spacing = theme::spacing();
    let text = widget::selectable_text::monotext(contents)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill);

    container(text)
        .class(theme::Container::Card)
        .padding(spacing.space_xs)
        .width(Length::Fill)
        .apply(widget::scrollable)
        .height(Length::Fixed(height))
        .width(Length::Fill)
        .into()
}
