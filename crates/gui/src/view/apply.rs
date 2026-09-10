//! Apply Changes page.

use crate::app::{AppModel, Busy};
use crate::message::Message;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{theme, Element};

const PREVIEW_HEIGHT: f32 = 200.0;
const LOG_HEIGHT: f32 = 200.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let apply_busy = matches!(app.busy, Busy::Applying | Busy::DryRun);
    let buttons_enabled = app.busy == Busy::Idle;

    let apply_label = match app.busy {
        Busy::Applying => crate::fl!("apply-changes-busy"),
        Busy::DryRun => crate::fl!("dry-run-busy"),
        _ => crate::fl!("apply-changes"),
    };
    let mut apply_button = widget::button::suggested(apply_label);
    if buttons_enabled {
        apply_button = apply_button.on_press(Message::RequestApply);
    }

    let mut dry_run =
        widget::button::standard(crate::fl!("dry-run")).tooltip(crate::fl!("dry-run-tooltip"));
    if buttons_enabled {
        dry_run = dry_run.on_press(Message::RequestDryRun);
    }

    let mut actions = widget::row::with_capacity(4)
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .push(apply_button)
        .push(dry_run);

    if apply_busy {
        actions = actions.push(widget::indeterminate_circular().size(20.0));
        let status = match app.busy {
            Busy::DryRun => crate::fl!("apply-validating"),
            _ => crate::fl!("apply-building"),
        };
        actions = actions.push(widget::text::body(status));
    }

    let preview = if app.apply_preview.is_empty() {
        crate::fl!("apply-preview-placeholder")
    } else {
        app.apply_preview.clone()
    };
    let log = if app.apply_log.is_empty() {
        crate::fl!("apply-log-placeholder")
    } else {
        app.apply_log.clone()
    };

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-apply-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-apply-desc"))
            .width(Length::Fill)
            .into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("apply-preview"),
                Some(crate::fl!("apply-preview-desc")),
            ))
            .add(crate::widget::code_view(preview, PREVIEW_HEIGHT))
            .into(),
        widget::button::standard(crate::fl!("refresh-preview"))
            .on_press(Message::RefreshPreview)
            .into(),
        actions.into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("apply-log"),
                Some(crate::fl!("apply-log-desc")),
            ))
            .add(crate::widget::code_view(log, LOG_HEIGHT))
            .into(),
    ])
    .into()
}
