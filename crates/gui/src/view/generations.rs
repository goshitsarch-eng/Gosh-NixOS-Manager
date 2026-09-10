//! Generations page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::Element;

const LOG_HEIGHT: f32 = 150.0;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let extra = (!app.generations_log().is_empty())
        .then(|| crate::widget::code_view(app.generations_log(), LOG_HEIGHT));
    super::titled_page_with(
        crate::fl!("page-generations-title"),
        crate::fl!("page-generations-desc"),
        extra,
    )
}
