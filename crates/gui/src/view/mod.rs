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
use common::actions::{
    default_bundles, default_maintenance_actions, default_profiles, default_system_actions,
};
use common::{BundleDef, MaintenanceActionDef, ProfileDef, SystemActionDef};
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::{theme, Apply, Element};
use std::borrow::Cow;
use std::sync::OnceLock;

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

pub(crate) fn catalog_profiles() -> &'static [ProfileDef] {
    static CELL: OnceLock<Vec<ProfileDef>> = OnceLock::new();
    CELL.get_or_init(default_profiles)
}

pub(crate) fn catalog_bundles() -> &'static [BundleDef] {
    static CELL: OnceLock<Vec<BundleDef>> = OnceLock::new();
    CELL.get_or_init(default_bundles)
}

pub(crate) fn catalog_system_actions() -> &'static [SystemActionDef] {
    static CELL: OnceLock<Vec<SystemActionDef>> = OnceLock::new();
    CELL.get_or_init(default_system_actions)
}

pub(crate) fn catalog_maintenance_actions() -> &'static [MaintenanceActionDef] {
    static CELL: OnceLock<Vec<MaintenanceActionDef>> = OnceLock::new();
    CELL.get_or_init(default_maintenance_actions)
}

pub(crate) fn icon<'a, Message: 'static>(
    name: impl Into<std::sync::Arc<str>>,
) -> Element<'a, Message> {
    widget::icon::from_name(name).size(16).into()
}

pub(crate) fn info_item<'a>(
    title: impl Into<Cow<'a, str>>,
    description: impl Into<Cow<'a, str>>,
    icon_name: &'static str,
) -> cosmic::widget::Row<'a, Message, cosmic::Theme> {
    settings::item::builder(title)
        .description(description)
        .icon(icon(icon_name))
        .control(widget::space::horizontal())
}

pub(crate) fn section_header<'a>(
    title: String,
    description: Option<String>,
) -> Element<'a, Message> {
    let spacing = theme::spacing();
    let mut col = widget::column::with_capacity(2)
        .spacing(spacing.space_xxxs)
        .push(widget::text::heading(title));
    if let Some(description) = description {
        col = col.push(widget::text::caption(description).width(Length::Fill));
    }
    col.into()
}
