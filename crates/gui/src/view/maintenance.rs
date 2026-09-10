//! Maintenance page.

use crate::app::{AppModel, Busy};
use crate::message::Message;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

const LOG_HEIGHT: f32 = 200.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let idle = app.busy == Busy::Idle;
    let calculating = app.busy == Busy::LoadingDisk;

    let mut actions = settings::section().title(crate::fl!("maintenance-actions"));
    for action in super::catalog_maintenance_actions() {
        let id = action.id.clone();
        let mut run =
            widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
                .tooltip(crate::fl!("run-action"));
        if idle {
            run = run.on_press(Message::RequestMaintenance { id });
        }
        actions = actions.add(
            settings::item::builder(action.name.as_str())
                .description(action.description.as_str())
                .icon(super::icon(action.icon.as_str()))
                .control(run),
        );
    }

    let store_sub = match (app.disk_usage.as_ref(), calculating) {
        (_, true) => crate::fl!("calculating"),
        (None, false) => crate::fl!("loading"),
        (Some(info), false) if info.store_size.is_empty() => {
            crate::fl!("maintenance-disk-error")
        }
        (Some(info), false) => info.store_size.clone(),
    };
    let generations_sub = match (app.disk_usage.as_ref(), calculating) {
        (_, true) => crate::fl!("calculating"),
        (None, false) => crate::fl!("loading"),
        (Some(info), false) => {
            crate::fl!(
                "maintenance-generation-count",
                count = info.generation_count
            )
        }
    };

    let refresh_label = if calculating {
        crate::fl!("refresh-disk-busy")
    } else {
        crate::fl!("refresh-disk")
    };
    let mut refresh = widget::button::standard(refresh_label);
    if idle {
        refresh = refresh.on_press(Message::LoadDiskUsage);
    }

    let log = if app.maintenance_log().is_empty() {
        crate::fl!("maintenance-log-placeholder")
    } else {
        app.maintenance_log().to_owned()
    };

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-maintenance-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-maintenance-desc"))
            .width(Length::Fill)
            .into(),
        actions.into(),
        settings::section()
            .title(crate::fl!("maintenance-disk"))
            .add(super::info_item(
                crate::fl!("maintenance-store-size"),
                store_sub,
                "drive-harddisk-symbolic",
            ))
            .add(super::info_item(
                crate::fl!("maintenance-generations"),
                generations_sub,
                "document-open-recent-symbolic",
            ))
            .add(refresh)
            .into(),
        settings::section()
            .title(crate::fl!("maintenance-log"))
            .add(crate::widget::code_view(log, LOG_HEIGHT))
            .into(),
    ])
    .into()
}
