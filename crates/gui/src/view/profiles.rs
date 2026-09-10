//! Desktop Profiles page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

const PREVIEW_HEIGHT: f32 = 200.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let profiles = super::catalog_profiles();
    let selected = app
        .state
        .selected_profile
        .as_deref()
        .and_then(|id| profiles.iter().position(|p| p.id == id));

    let mut list = settings::section().title(crate::fl!("profiles-available"));
    for (index, profile) in profiles.iter().enumerate() {
        let id = profile.id.clone();
        list = list.add(
            settings::item::builder(profile.name.as_str())
                .description(profile.description.as_str())
                .icon(super::icon(profile.icon.as_str()))
                .radio(index, selected, move |_| Message::SelectProfile(id.clone())),
        );
    }

    let preview = if app.profile_preview.is_empty() {
        crate::fl!("profiles-preview-placeholder")
    } else {
        app.profile_preview.clone()
    };

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-profiles-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-profiles-desc"))
            .width(Length::Fill)
            .into(),
        list.into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("profiles-preview"),
                Some(crate::fl!("profiles-preview-desc")),
            ))
            .add(crate::widget::code_view(preview, PREVIEW_HEIGHT))
            .into(),
    ])
    .into()
}
