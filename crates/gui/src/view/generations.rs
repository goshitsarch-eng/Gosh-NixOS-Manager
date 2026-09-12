//! Generations page.

use crate::app::{AppModel, Busy};
use crate::message::Message;
use common::ipc::Generation;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{theme, Element};

const LOG_HEIGHT: f32 = 150.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let loading = app.busy == Busy::LoadingGenerations;
    let idle = app.busy == Busy::Idle;
    let buttons_enabled = idle && !app.helper_missing;

    let mut refresh = widget::button::standard(crate::fl!("refresh"))
        .leading_icon(widget::icon::from_name("view-refresh-symbolic"));
    if buttons_enabled {
        refresh = refresh.on_press(Message::LoadGenerations);
    } else if app.helper_missing {
        refresh = refresh.tooltip(crate::fl!("helper-missing-action"));
    }
    let mut rollback = widget::button::suggested(crate::fl!("rollback-previous"))
        .leading_icon(widget::icon::from_name("edit-undo-symbolic"));
    if app.helper_missing {
        rollback = rollback.tooltip(crate::fl!("helper-missing-action"));
    }
    if buttons_enabled {
        rollback = rollback.on_press(Message::RollbackToPrevious);
    }

    let toolbar = widget::row::with_capacity(2)
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .push(refresh)
        .push(rollback);

    let mut list = settings::section().title(crate::fl!("generations-available"));
    if loading {
        list = list.add(
            widget::row::with_capacity(2)
                .spacing(spacing.space_s)
                .align_y(Alignment::Center)
                .push(widget::indeterminate_circular().size(20.0))
                .push(widget::text::body(crate::fl!("loading"))),
        );
    } else if app.generations.is_empty() {
        list = list.add(crate::widget::empty_placeholder(
            crate::fl!("generations-empty"),
            crate::fl!("generations-empty-desc"),
        ));
    } else {
        for generation in &app.generations {
            list = list.add(generation_row(
                generation,
                buttons_enabled,
                app.helper_missing,
            ));
        }
    }

    let log = if app.generations_log().is_empty() {
        crate::fl!("generations-log-placeholder")
    } else {
        app.generations_log().to_owned()
    };

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-generations-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-generations-desc"))
            .width(Length::Fill)
            .into(),
        toolbar.into(),
        list.into(),
        settings::section()
            .title(crate::fl!("generations-boot"))
            .add(super::info_item(
                crate::fl!("generations-boot-title"),
                crate::fl!("generations-boot-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
        settings::section()
            .title(crate::fl!("generations-log"))
            .add(crate::widget::code_view(log, LOG_HEIGHT))
            .into(),
    ])
    .into()
}

fn generation_row(
    generation: &Generation,
    buttons_enabled: bool,
    helper_missing: bool,
) -> cosmic::widget::Row<'_, Message, cosmic::Theme> {
    let title = if generation.current {
        crate::fl!("generation-n-current", n = generation.number)
    } else {
        crate::fl!("generation-n", n = generation.number)
    };
    let icon_name = if generation.current {
        "emblem-ok-symbolic"
    } else {
        "document-open-recent-symbolic"
    };
    let item = settings::item::builder(title)
        .description(generation_subtitle(generation))
        .icon(super::icon(icon_name));

    if generation.current {
        item.control(widget::text::caption(crate::fl!("generation-current")))
    } else {
        let number = generation.number;
        let switch_tooltip = if helper_missing {
            crate::fl!("helper-missing-action")
        } else {
            crate::fl!("switch-generation")
        };
        let delete_tooltip = if helper_missing {
            crate::fl!("helper-missing-action")
        } else {
            crate::fl!("delete-generation")
        };
        let mut switch =
            widget::button::icon(widget::icon::from_name("system-switch-user-symbolic"))
                .tooltip(switch_tooltip);
        let mut delete = widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
            .class(cosmic::theme::Button::Destructive)
            .tooltip(delete_tooltip);
        if buttons_enabled {
            switch = switch.on_press(Message::RequestRollback { generation: number });
            delete = delete.on_press(Message::RequestDeleteGeneration { generation: number });
        }
        item.control(
            widget::row::with_capacity(2)
                .spacing(theme::spacing().space_xxs)
                .align_y(Alignment::Center)
                .push(switch)
                .push(delete),
        )
    }
}

fn generation_subtitle(generation: &Generation) -> String {
    match (
        generation.nixos_version.as_deref(),
        generation.kernel_version.as_deref(),
    ) {
        (Some(nixos), Some(kernel)) => {
            format!("{} | NixOS {nixos} | Kernel {kernel}", generation.date)
        }
        (Some(nixos), None) => format!("{} | NixOS {nixos}", generation.date),
        (None, Some(kernel)) => format!("{} | Kernel {kernel}", generation.date),
        (None, None) => generation.date.clone(),
    }
}
