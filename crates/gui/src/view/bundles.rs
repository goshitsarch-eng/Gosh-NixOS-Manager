//! Software Bundles page.

use crate::app::{AppModel, BannerKind};
use crate::message::Message;
use common::ArmCompat;
use cosmic::iced::Length;
use cosmic::widget::{self, container, settings};
use cosmic::{theme, Element};
use std::collections::BTreeSet;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let is_arm = app.cpu_arch().is_arm();
    let mut children = vec![
        widget::text::title2(crate::fl!("page-bundles-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-bundles-desc"))
            .width(Length::Fill)
            .into(),
    ];

    if is_arm {
        children.push(crate::widget::status_banner(
            BannerKind::Warning,
            crate::fl!("banner-arm-bundles"),
        ));
    }

    children.push(super::section_header(
        crate::fl!("bundles-available"),
        Some(crate::fl!("bundles-available-desc")),
    ));

    for bundle in super::catalog_bundles() {
        let sensitive = !(is_arm && bundle.arm_compat == ArmCompat::None);
        let enabled = app.state.is_bundle_enabled(&bundle.id);
        let expanded = app.state.expanded_bundles.contains(&bundle.id);
        let selected = app.state.get_bundle_packages(&bundle.id);
        let subtitle = bundle_subtitle(bundle, is_arm);
        let compat_icon = (is_arm && bundle.arm_compat != ArmCompat::Full)
            .then_some(bundle.arm_compat.icon_name());

        let body = expanded.then(|| {
            let mut packages = settings::section();
            for package in &bundle.packages {
                let checked = selected.is_some_and(|set| set.contains(&package.id));
                let bundle_id = bundle.id.clone();
                let package_id = package.id.clone();
                packages = packages.add(
                    settings::item::builder(package.display_name.as_str())
                        .description(format!("pkgs.{}", package.id))
                        .checkbox(checked, move |enabled| Message::ToggleBundlePackage {
                            bundle_id: bundle_id.clone(),
                            package: package_id.clone(),
                            enabled,
                        }),
                );
            }
            Element::from(packages)
        });

        let id = bundle.id.clone();
        let expand_id = bundle.id.clone();
        children.push(crate::widget::bundle_expander(
            bundle.icon.as_str(),
            bundle.name.as_str(),
            subtitle,
            crate::fl!("bundles-package-count", count = bundle.packages.len()),
            enabled,
            expanded,
            sensitive,
            compat_icon,
            move |enabled| Message::ToggleBundle {
                id: id.clone(),
                enabled,
            },
            Message::ExpandBundle {
                id: expand_id,
                expanded: !expanded,
            },
            body,
        ));
    }

    let mut selected_ids = BTreeSet::new();
    for packages in app.state.bundle_packages.values() {
        selected_ids.extend(packages.iter().cloned());
    }
    let summary = if selected_ids.is_empty() {
        crate::fl!("bundles-none-selected")
    } else {
        selected_ids.into_iter().collect::<Vec<_>>().join(", ")
    };

    children.push(
        settings::section()
            .title(crate::fl!("bundles-selected"))
            .add(
                container(widget::text::body(summary).width(Length::Fill))
                    .class(theme::Container::Card)
                    .padding(spacing.space_s)
                    .width(Length::Fill),
            )
            .into(),
    );

    settings::view_column(children).into()
}

fn bundle_subtitle(bundle: &common::BundleDef, is_arm: bool) -> String {
    if is_arm && bundle.arm_compat != ArmCompat::Full {
        let note = bundle
            .arm_note
            .as_deref()
            .unwrap_or_else(|| bundle.arm_compat.display_name());
        format!("{} | ⚠️ {note}", bundle.description)
    } else {
        bundle.description.clone()
    }
}
