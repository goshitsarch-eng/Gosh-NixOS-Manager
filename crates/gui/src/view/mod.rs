//! View layer: page dispatch, banner, scrollable settings column.

mod apply;
mod bundles;
mod generations;
mod hardware;
mod maintenance;
mod network;
mod onboarding;
mod packages;
mod profiles;
mod services;
mod system;

use crate::app::AppModel;
use crate::message::{Message, Page};
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::{theme, Apply, Element};

/// Root content for the main window.
///
/// Architecture already wraps this in `widget::toaster` in `Application::view`.
/// Nesting a second toaster over the same `Toasts` would duplicate toast UI.
pub fn root<'a>(app: &'a AppModel) -> Element<'a, Message> {
    let spacing = theme::spacing();
    let mut children = Vec::with_capacity(2);

    if let Some(banner) = app.banner() {
        children.push(crate::widget::status_banner(
            banner.kind,
            banner.text.as_str(),
        ));
    }
    children.push(page(app));

    settings::view_column(children)
        .padding(spacing.space_m)
        .width(Length::Fill)
        .apply(widget::scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn page<'a>(app: &'a AppModel) -> Element<'a, Message> {
    match app.page() {
        Page::Onboarding => onboarding::view(app),
        Page::Profiles => profiles::view(app),
        Page::Bundles => bundles::view(app),
        Page::Packages => packages::view(app),
        Page::System => system::view(app),
        Page::Hardware => hardware::view(app),
        Page::Network => network::view(app),
        Page::Services => services::view(app),
        Page::Generations => generations::view(app),
        Page::Maintenance => maintenance::view(app),
        Page::Apply => apply::view(app),
    }
}

/// Title + wrapping description. Full controls land in later tasks.
pub(crate) fn titled_page<'a>(title: String, description: String) -> Element<'a, Message> {
    titled_page_with(title, description, std::iter::empty())
}

pub(crate) fn titled_page_with<'a>(
    title: String,
    description: String,
    extra: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut children = vec![
        widget::text::title2(title).width(Length::Fill).into(),
        widget::text::body(description).width(Length::Fill).into(),
    ];
    children.extend(extra);
    settings::view_column(children).into()
}
