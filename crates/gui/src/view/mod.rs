//! View layer. UX replaces this skeleton.

use crate::app::AppModel;
use crate::message::Message;

/// Root content for the main window. UX replaces this.
pub fn root<'a>(app: &'a AppModel) -> cosmic::Element<'a, Message> {
    let page_name = app.page().title();
    let mut col = cosmic::widget::column::with_capacity(3)
        .push(cosmic::widget::text::title2(crate::fl!("app-title")))
        .push(cosmic::widget::text::body(page_name))
        .spacing(cosmic::theme::spacing().space_s);

    if let Some(banner) = app.banner() {
        col = col.push(cosmic::widget::text::body(&banner.text));
    }

    col = col.push(cosmic::widget::text::caption(app.cpu_arch().display_name()));
    if !app.generations_log().is_empty() {
        col = col.push(cosmic::widget::text::caption(app.generations_log()));
    }
    if !app.maintenance_log().is_empty() {
        col = col.push(cosmic::widget::text::caption(app.maintenance_log()));
    }

    col.into()
}
