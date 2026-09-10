//! System Settings page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::Element;

pub fn view(_app: &AppModel) -> Element<'_, Message> {
    super::titled_page(
        crate::fl!("page-system-title"),
        crate::fl!("page-system-desc"),
    )
}
