//! Custom Packages page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::Element;

pub fn view(_app: &AppModel) -> Element<'_, Message> {
    super::titled_page(
        crate::fl!("page-packages-title"),
        crate::fl!("page-packages-desc"),
    )
}
