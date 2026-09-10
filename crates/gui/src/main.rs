//! NixOS Toolkit — libcosmic entry point.

use nixos_toolkit_gui::app::AppModel;
use nixos_toolkit_gui::config::UserPreferences;
use nixos_toolkit_gui::{i18n, Flags};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> cosmic::iced::Result {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("nixos_toolkit_gui=info".parse().expect("directive")),
        )
        .init();

    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    tracing::info!("Starting NixOS Toolkit");

    let flags = Flags::from_env();
    let prefs = UserPreferences::load(flags.prefs_path.as_deref());

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(1000.0, 700.0))
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(640.0)
                .min_height(480.0),
        )
        .theme(prefs.color_scheme.to_theme());

    // Do not enable single-instance (DECISIONS C22).
    cosmic::app::run::<AppModel>(settings, flags)
}
