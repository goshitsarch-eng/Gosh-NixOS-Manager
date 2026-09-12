//! Maintenance page.

use crate::app::{AppModel, Busy};
use crate::message::Message;
use common::config::ConfigMode;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

const LOG_HEIGHT: f32 = 200.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let idle = app.busy == Busy::Idle;
    let buttons_enabled = idle && !app.helper_missing;
    let calculating = app.busy == Busy::LoadingDisk;

    let mut actions = settings::section().title(crate::fl!("maintenance-actions"));
    for action in super::catalog_maintenance_actions() {
        let id = action.id.clone();
        let run_tooltip = if app.helper_missing {
            crate::fl!("helper-missing-action")
        } else {
            crate::fl!("run-action")
        };
        let mut run =
            widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
                .tooltip(run_tooltip);
        if buttons_enabled {
            run = run.on_press(Message::RequestMaintenance { id });
        }
        let (name, description) = if action.id == "update_channels" {
            if app.system_info.config_mode == ConfigMode::Flake {
                (
                    crate::fl!("maintenance-update-flake"),
                    crate::fl!("maintenance-update-flake-desc"),
                )
            } else {
                (
                    crate::fl!("maintenance-update-channels"),
                    crate::fl!("maintenance-update-channels-desc"),
                )
            }
        } else {
            (action.name.clone(), action.description.clone())
        };
        actions = actions.add(
            settings::item::builder(name)
                .description(description)
                .icon(super::icon(action.icon.as_str()))
                .control(run),
        );
    }

    let store_sub = match (app.disk_usage.as_ref(), calculating) {
        (_, true) => crate::fl!("calculating"),
        (None, false) => crate::fl!("loading"),
        (Some(info), false) if info.store_size.is_empty() => info
            .error
            .clone()
            .unwrap_or_else(|| crate::fl!("maintenance-disk-error")),
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
    if buttons_enabled {
        refresh = refresh.on_press(Message::LoadDiskUsage);
    } else if app.helper_missing {
        refresh = refresh.tooltip(crate::fl!("helper-missing-action"));
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
