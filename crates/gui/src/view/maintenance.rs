//! Maintenance page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::Element;

const LOG_HEIGHT: f32 = 200.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let extra = (!app.maintenance_log().is_empty())
        .then(|| crate::widget::code_view(app.maintenance_log(), LOG_HEIGHT));
    super::titled_page_with(
        crate::fl!("page-maintenance-title"),
        crate::fl!("page-maintenance-desc"),
        extra,
    )
}
