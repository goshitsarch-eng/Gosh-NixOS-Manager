//! Hardware page.

use crate::app::{AppModel, BannerKind};
use crate::message::Message;
use cosmic::iced::Length;
use cosmic::widget::{self, settings};
use cosmic::Element;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let is_arm = app.cpu_arch().is_arm();
    let hw = &app.state.hardware_config;
    let gpu_label = app
        .gpu_vendor
        .as_deref()
        .map_or_else(|| crate::fl!("hardware-gpu-unknown"), str::to_owned);
    let show_nvidia = !is_arm
        && app
            .gpu_vendor
            .as_deref()
            .is_some_and(|gpu| gpu.to_ascii_lowercase().contains("nvidia"));

    let mut children = vec![
        widget::text::title2(crate::fl!("page-hardware-title"))
            .width(Length::Fill)
            .into(),
        widget::text::body(crate::fl!("page-hardware-desc"))
            .width(Length::Fill)
            .into(),
    ];

    if is_arm {
        children.push(crate::widget::status_banner(
            BannerKind::Warning,
            crate::fl!("banner-arm-hardware"),
        ));
    }

    let mut graphics = settings::section()
        .header(super::section_header(
            crate::fl!("hardware-graphics"),
            Some(crate::fl!("hardware-graphics-desc")),
        ))
        .add(super::info_item(
            crate::fl!("hardware-gpu"),
            gpu_label,
            "video-display-symbolic",
        ));

    if show_nvidia {
        let nvidia_idx = usize::from(hw.nvidia_driver.unwrap_or(0).min(3));
        let nvidia_options = vec![
            crate::fl!("nvidia-stable"),
            crate::fl!("nvidia-beta"),
            crate::fl!("nvidia-open"),
            crate::fl!("nvidia-nouveau"),
        ];
        graphics = graphics
            .add(
                settings::item::builder(crate::fl!("hardware-nvidia-driver"))
                    .description(crate::fl!("hardware-nvidia-driver-desc"))
                    .icon(super::icon("application-x-firmware-symbolic"))
                    .control(widget::dropdown(
                        nvidia_options,
                        Some(nvidia_idx),
                        |index| Message::SetNvidiaDriver(Some(index as u8)),
                    )),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-nvidia-modesetting"))
                    .description(crate::fl!("hardware-nvidia-modesetting-desc"))
                    .icon(super::icon("preferences-desktop-display-symbolic"))
                    .toggler(hw.nvidia_modesetting, Message::SetNvidiaModesetting),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-nvidia-power"))
                    .description(crate::fl!("hardware-nvidia-power-desc"))
                    .icon(super::icon("battery-symbolic"))
                    .toggler(hw.nvidia_powermanagement, Message::SetNvidiaPowerManagement),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-nvidia-open"))
                    .description(crate::fl!("hardware-nvidia-open-desc"))
                    .icon(super::icon("emblem-system-symbolic"))
                    .toggler(hw.nvidia_open, Message::SetNvidiaOpen),
            );
    }

    let audio_idx = usize::from(hw.audio_server.min(2));
    let audio_options = vec![
        crate::fl!("audio-pipewire"),
        crate::fl!("audio-pulseaudio"),
        crate::fl!("audio-none"),
    ];
    let power_idx = usize::from(hw.power_profile.min(2));
    let power_options = vec![
        crate::fl!("power-balanced"),
        crate::fl!("power-performance"),
        crate::fl!("power-saver"),
    ];
    let thermald_desc = if is_arm {
        crate::fl!("hardware-thermald-arm")
    } else {
        crate::fl!("hardware-thermald-desc")
    };

    children.push(graphics.into());
    children.push(
        settings::section()
            .header(super::section_header(
                crate::fl!("hardware-audio"),
                Some(crate::fl!("hardware-audio-desc")),
            ))
            .add(
                settings::item::builder(crate::fl!("hardware-audio-server"))
                    .description(crate::fl!("hardware-audio-server-desc"))
                    .icon(super::icon("audio-speakers-symbolic"))
                    .control(widget::dropdown(audio_options, Some(audio_idx), |index| {
                        Message::SetAudioServer(index as u8)
                    })),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-audio-lowlatency"))
                    .description(crate::fl!("hardware-audio-lowlatency-desc"))
                    .icon(super::icon("audio-input-microphone-symbolic"))
                    .toggler(hw.audio_lowlatency, Message::SetAudioLowLatency),
            )
            .into(),
    );
    children.push(
        settings::section()
            .header(super::section_header(
                crate::fl!("hardware-bluetooth"),
                Some(crate::fl!("hardware-bluetooth-desc")),
            ))
            .add(
                settings::item::builder(crate::fl!("hardware-bluetooth-enable"))
                    .description(crate::fl!("hardware-bluetooth-enable-desc"))
                    .icon(super::icon("bluetooth-symbolic"))
                    .toggler(hw.bluetooth_enabled, Message::SetBluetoothEnabled),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-bluetooth-autopower"))
                    .description(crate::fl!("hardware-bluetooth-autopower-desc"))
                    .icon(super::icon("system-restart-symbolic"))
                    .toggler(hw.bluetooth_autopower, Message::SetBluetoothAutoPower),
            )
            .into(),
    );
    children.push(
        settings::section()
            .header(super::section_header(
                crate::fl!("hardware-power"),
                Some(crate::fl!("hardware-power-desc")),
            ))
            .add(
                settings::item::builder(crate::fl!("hardware-power-profile"))
                    .description(if hw.tlp_enabled {
                        crate::fl!("hardware-power-profile-tlp")
                    } else {
                        crate::fl!("hardware-power-profile-desc")
                    })
                    .icon(super::icon("power-profile-balanced-symbolic"))
                    .control(widget::dropdown(power_options, Some(power_idx), |index| {
                        Message::SetPowerProfile(index as u8)
                    })),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-tlp"))
                    .description(crate::fl!("hardware-tlp-desc"))
                    .icon(super::icon("battery-full-symbolic"))
                    .toggler(hw.tlp_enabled, Message::SetTlpEnabled),
            )
            .add(
                settings::item::builder(crate::fl!("hardware-thermald"))
                    .description(thermald_desc)
                    .icon(super::icon("sensors-temperature-symbolic"))
                    .toggler_maybe(
                        hw.thermald_enabled,
                        (!is_arm).then_some(Message::SetThermaldEnabled),
                    ),
            )
            .into(),
    );
    children.push(
        settings::section()
            .add(super::info_item(
                crate::fl!("hardware-note"),
                crate::fl!("hardware-note-desc"),
                "dialog-information-symbolic",
            ))
            .into(),
    );

    settings::view_column(children).into()
}
