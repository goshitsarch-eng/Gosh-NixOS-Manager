//! Hardware page.

use crate::app::{AppModel, BannerKind};
use crate::message::Message;
use cosmic::Element;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let extra = app.cpu_arch().is_arm().then(|| {
        crate::widget::status_banner(BannerKind::Warning, crate::fl!("banner-arm-hardware"))
    });
    super::titled_page_with(
        crate::fl!("page-hardware-title"),
        crate::fl!("page-hardware-desc"),
        extra,
    )
}
