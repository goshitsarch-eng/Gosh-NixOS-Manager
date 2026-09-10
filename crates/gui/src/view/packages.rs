//! Custom Packages page.

use crate::app::AppModel;
use crate::message::Message;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{theme, Element};

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let add_row = widget::row::with_capacity(2)
        .spacing(spacing.space_s)
        .align_y(Alignment::Center)
        .push(
            widget::text_input(
                crate::fl!("packages-placeholder"),
                app.package_input.as_str(),
            )
            .on_input(Message::PackageInputChanged)
            .on_submit(|_| Message::AddPackagesFromInput)
            .width(Length::Fill),
        )
        .push(widget::button::suggested(crate::fl!("add")).on_press(Message::AddPackagesFromInput));

    let mut installed = settings::section().header(super::section_header(
        crate::fl!("packages-installed"),
        Some(crate::fl!("packages-installed-desc")),
    ));

    if app.state.custom_packages.is_empty() {
        installed = installed.add(crate::widget::empty_placeholder(
            crate::fl!("packages-empty"),
            crate::fl!("packages-empty-desc"),
        ));
    } else {
        let mut packages: Vec<_> = app.state.custom_packages.iter().cloned().collect();
        packages.sort();
        for package in packages {
            installed = installed.add(
                settings::item::builder(package.clone())
                    .description(format!("pkgs.{package}"))
                    .icon(super::icon("package-x-generic-symbolic"))
                    .control(
                        widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
                            .tooltip(crate::fl!("remove-package"))
                            .on_press(Message::RemoveCustomPackage(package)),
                    ),
            );
        }
    }

    settings::view_column(vec![
        widget::text::title2(crate::fl!("page-packages-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-packages-desc"))
            .width(Length::Fill)
            .into(),
        settings::section()
            .header(super::section_header(
                crate::fl!("packages-add"),
                Some(crate::fl!("packages-add-desc")),
            ))
            .add(add_row)
            .into(),
        installed.into(),
    ])
    .into()
}
