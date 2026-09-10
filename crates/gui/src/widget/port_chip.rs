//! Independent preset-port toggle (GTK `ToggleButton` chips).

use cosmic::widget;
use cosmic::Element;
use std::borrow::Cow;

/// Standard button when off, suggested when the port is open.
#[must_use]
pub fn port_chip<'a, Message: Clone + 'static>(
    label: impl Into<Cow<'a, str>>,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let label = label.into();
    if selected {
        widget::button::suggested(label).on_press(on_press).into()
    } else {
        widget::button::standard(label).on_press(on_press).into()
    }
}
