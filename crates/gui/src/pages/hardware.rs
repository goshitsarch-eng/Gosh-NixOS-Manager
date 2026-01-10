//! Hardware configuration page (GPU, Audio, Bluetooth, Power Management)

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::CpuArch;
use gtk::glib;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct HardwarePage {
        // GPU settings
        pub gpu_vendor: RefCell<Option<String>>,
        pub nvidia_driver: RefCell<Option<adw::ComboRow>>,
        pub nvidia_modesetting: RefCell<Option<adw::SwitchRow>>,
        pub nvidia_powermanagement: RefCell<Option<adw::SwitchRow>>,
        pub nvidia_open: RefCell<Option<adw::SwitchRow>>,
        // Audio settings
        pub audio_server: RefCell<Option<adw::ComboRow>>,
        pub audio_lowlatency: RefCell<Option<adw::SwitchRow>>,
        // Bluetooth settings
        pub bluetooth_enabled: RefCell<Option<adw::SwitchRow>>,
        pub bluetooth_autopower: RefCell<Option<adw::SwitchRow>>,
        // Power management
        pub power_profile: RefCell<Option<adw::ComboRow>>,
        pub tlp_enabled: RefCell<Option<adw::SwitchRow>>,
        pub thermald_enabled: RefCell<Option<adw::SwitchRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HardwarePage {
        const NAME: &'static str = "NixosToolkitHardwarePage";
        type Type = super::HardwarePage;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for HardwarePage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for HardwarePage {}
    impl BoxImpl for HardwarePage {}
}

glib::wrapper! {
    pub struct HardwarePage(ObjectSubclass<imp::HardwarePage>)
        @extends gtk::Box, gtk::Widget;
}

impl HardwarePage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("orientation", gtk::Orientation::Vertical)
            .property("spacing", 0)
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();
        let is_arm = CpuArch::detect().is_arm();

        // Wrap everything in a scrolled window
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_start(24)
            .margin_end(24)
            .margin_top(24)
            .margin_bottom(24)
            .build();

        // Title
        let title = gtk::Label::builder()
            .label("Hardware Configuration")
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build();
        content.append(&title);

        // Description
        let desc = gtk::Label::builder()
            .label("Configure graphics drivers, audio, bluetooth, and power management settings.")
            .wrap(true)
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        content.append(&desc);

        // ARM warning banner
        if is_arm {
            let arm_banner = adw::Banner::builder()
                .title("ARM64: NVIDIA drivers and Intel Thermald are not available")
                .revealed(true)
                .build();
            arm_banner.add_css_class("warning");
            content.append(&arm_banner);
        }

        // === GPU Configuration ===
        let gpu_group = adw::PreferencesGroup::builder()
            .title("Graphics Drivers")
            .description("Configure GPU drivers for your system")
            .build();

        // Detect GPU
        let detected_gpu = self.detect_gpu();
        *imp.gpu_vendor.borrow_mut() = Some(detected_gpu.clone());

        let gpu_info_row = adw::ActionRow::builder()
            .title("Detected GPU")
            .subtitle(&detected_gpu)
            .build();
        gpu_info_row.add_prefix(&gtk::Image::from_icon_name("video-display-symbolic"));
        gpu_group.add(&gpu_info_row);

        // NVIDIA-specific options (not available on ARM)
        if detected_gpu.to_lowercase().contains("nvidia") && !is_arm {
            // Driver selection
            let nvidia_driver = adw::ComboRow::builder()
                .title("NVIDIA Driver")
                .subtitle("Select which NVIDIA driver package to use")
                .build();
            nvidia_driver.set_model(Some(&gtk::StringList::new(&[
                "Stable (nvidia)",
                "Beta (nvidia-beta)",
                "Open Source (nvidia-open)",
                "Nouveau (open-source, limited)",
            ])));
            nvidia_driver.set_selected(0);
            nvidia_driver.add_prefix(&gtk::Image::from_icon_name("application-x-firmware-symbolic"));
            gpu_group.add(&nvidia_driver);
            *imp.nvidia_driver.borrow_mut() = Some(nvidia_driver);

            // Modesetting
            let modesetting = adw::SwitchRow::builder()
                .title("Modesetting")
                .subtitle("Enable kernel modesetting (recommended for Wayland)")
                .active(true)
                .build();
            modesetting.add_prefix(&gtk::Image::from_icon_name("preferences-desktop-display-symbolic"));
            gpu_group.add(&modesetting);
            *imp.nvidia_modesetting.borrow_mut() = Some(modesetting);

            // Power Management
            let powermgmt = adw::SwitchRow::builder()
                .title("Power Management")
                .subtitle("Enable experimental power management features")
                .active(false)
                .build();
            powermgmt.add_prefix(&gtk::Image::from_icon_name("battery-symbolic"));
            gpu_group.add(&powermgmt);
            *imp.nvidia_powermanagement.borrow_mut() = Some(powermgmt);

            // Open kernel modules
            let open_modules = adw::SwitchRow::builder()
                .title("Open Kernel Modules")
                .subtitle("Use open-source NVIDIA kernel modules (Turing+ GPUs)")
                .active(false)
                .build();
            open_modules.add_prefix(&gtk::Image::from_icon_name("emblem-system-symbolic"));
            gpu_group.add(&open_modules);
            *imp.nvidia_open.borrow_mut() = Some(open_modules);
        }

        content.append(&gpu_group);

        // === Audio Configuration ===
        let audio_group = adw::PreferencesGroup::builder()
            .title("Audio")
            .description("Configure audio server and settings")
            .build();

        let audio_server = adw::ComboRow::builder()
            .title("Audio Server")
            .subtitle("Select the audio server for your system")
            .build();
        audio_server.set_model(Some(&gtk::StringList::new(&[
            "PipeWire (recommended)",
            "PulseAudio",
            "None",
        ])));
        audio_server.set_selected(0);
        audio_server.add_prefix(&gtk::Image::from_icon_name("audio-speakers-symbolic"));
        audio_group.add(&audio_server);
        *imp.audio_server.borrow_mut() = Some(audio_server);

        let lowlatency = adw::SwitchRow::builder()
            .title("Low Latency Audio")
            .subtitle("Enable low-latency settings for professional audio work")
            .active(false)
            .build();
        lowlatency.add_prefix(&gtk::Image::from_icon_name("audio-input-microphone-symbolic"));
        audio_group.add(&lowlatency);
        *imp.audio_lowlatency.borrow_mut() = Some(lowlatency);

        content.append(&audio_group);

        // === Bluetooth Configuration ===
        let bluetooth_group = adw::PreferencesGroup::builder()
            .title("Bluetooth")
            .description("Configure Bluetooth hardware and settings")
            .build();

        let bt_enabled = adw::SwitchRow::builder()
            .title("Enable Bluetooth")
            .subtitle("Enable Bluetooth hardware and services")
            .active(false)
            .build();
        bt_enabled.add_prefix(&gtk::Image::from_icon_name("bluetooth-symbolic"));
        bluetooth_group.add(&bt_enabled);
        *imp.bluetooth_enabled.borrow_mut() = Some(bt_enabled);

        let bt_autopower = adw::SwitchRow::builder()
            .title("Power on at Boot")
            .subtitle("Automatically power on Bluetooth adapter at system startup")
            .active(false)
            .build();
        bt_autopower.add_prefix(&gtk::Image::from_icon_name("system-restart-symbolic"));
        bluetooth_group.add(&bt_autopower);
        *imp.bluetooth_autopower.borrow_mut() = Some(bt_autopower);

        content.append(&bluetooth_group);

        // === Power Management ===
        let power_group = adw::PreferencesGroup::builder()
            .title("Power Management")
            .description("Configure power settings for laptops and desktops")
            .build();

        let power_profile = adw::ComboRow::builder()
            .title("Power Profile")
            .subtitle("Select power/performance balance")
            .build();
        power_profile.set_model(Some(&gtk::StringList::new(&[
            "Balanced",
            "Performance",
            "Power Saver",
        ])));
        power_profile.set_selected(0);
        power_profile.add_prefix(&gtk::Image::from_icon_name("power-profile-balanced-symbolic"));
        power_group.add(&power_profile);
        *imp.power_profile.borrow_mut() = Some(power_profile);

        let tlp = adw::SwitchRow::builder()
            .title("TLP Power Management")
            .subtitle("Advanced power management for laptops (recommended)")
            .active(false)
            .build();
        tlp.add_prefix(&gtk::Image::from_icon_name("battery-full-symbolic"));
        power_group.add(&tlp);
        *imp.tlp_enabled.borrow_mut() = Some(tlp);

        let thermald = adw::SwitchRow::builder()
            .title("Thermald")
            .subtitle(if is_arm {
                "Intel-only - not available on ARM"
            } else {
                "Thermal management daemon for Intel CPUs"
            })
            .active(false)
            .sensitive(!is_arm)
            .build();
        thermald.add_prefix(&gtk::Image::from_icon_name("sensors-temperature-symbolic"));
        power_group.add(&thermald);
        *imp.thermald_enabled.borrow_mut() = Some(thermald);

        content.append(&power_group);

        // Note about changes
        let note_group = adw::PreferencesGroup::new();
        let note = adw::ActionRow::builder()
            .title("Note")
            .subtitle("Hardware changes require a system rebuild. Some changes may require a reboot.")
            .build();
        note.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        note_group.add(&note);
        content.append(&note_group);

        scroll.set_child(Some(&content));
        self.append(&scroll);
    }

    fn detect_gpu(&self) -> String {
        // Try to detect GPU using lspci
        let output = std::process::Command::new("lspci")
            .output()
            .ok();

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let lower = line.to_lowercase();
                if lower.contains("vga") || lower.contains("3d") || lower.contains("display") {
                    // Extract vendor and device name
                    if lower.contains("nvidia") {
                        return format!("NVIDIA: {}", line.split(':').last().unwrap_or("Unknown").trim());
                    } else if lower.contains("amd") || lower.contains("radeon") {
                        return format!("AMD: {}", line.split(':').last().unwrap_or("Unknown").trim());
                    } else if lower.contains("intel") {
                        return format!("Intel: {}", line.split(':').last().unwrap_or("Unknown").trim());
                    }
                }
            }
        }

        "Unknown GPU (lspci not available)".to_string()
    }

    /// Get the current hardware configuration for Nix generation
    pub fn get_hardware_config(&self) -> HardwareConfig {
        let imp = self.imp();

        HardwareConfig {
            // GPU
            nvidia_driver: imp.nvidia_driver.borrow().as_ref().map(|c| c.selected() as u8),
            nvidia_modesetting: imp.nvidia_modesetting.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            nvidia_powermanagement: imp.nvidia_powermanagement.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            nvidia_open: imp.nvidia_open.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            // Audio
            audio_server: imp.audio_server.borrow().as_ref().map(|c| c.selected() as u8).unwrap_or(0),
            audio_lowlatency: imp.audio_lowlatency.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            // Bluetooth
            bluetooth_enabled: imp.bluetooth_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            bluetooth_autopower: imp.bluetooth_autopower.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            // Power
            power_profile: imp.power_profile.borrow().as_ref().map(|c| c.selected() as u8).unwrap_or(0),
            tlp_enabled: imp.tlp_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
            thermald_enabled: imp.thermald_enabled.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false),
        }
    }
}

impl Default for HardwarePage {
    fn default() -> Self {
        Self::new()
    }
}

/// Hardware configuration state
#[derive(Debug, Clone, Default)]
pub struct HardwareConfig {
    pub nvidia_driver: Option<u8>,
    pub nvidia_modesetting: bool,
    pub nvidia_powermanagement: bool,
    pub nvidia_open: bool,
    pub audio_server: u8,
    pub audio_lowlatency: bool,
    pub bluetooth_enabled: bool,
    pub bluetooth_autopower: bool,
    pub power_profile: u8,
    pub tlp_enabled: bool,
    pub thermald_enabled: bool,
}
