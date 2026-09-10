//! Desktop Profiles page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::Element;

pub fn view(_app: &AppModel) -> Element<'_, Message> {
    super::titled_page(
        crate::fl!("page-profiles-title"),
        crate::fl!("page-profiles-desc"),
    )
}
