//! Flat application message enum (frozen contract from PLAN.md / DECISIONS.md).

use crate::config::{ColorSchemePreference, UserPreferences};
use common::config::SystemInfo;
use common::ipc::{
    AppState as IpcAppState, DiskUsageInfo, Generation, HelperRequest, HelperResponse, RebuildType,
};
use std::path::PathBuf;

/// Top-level application message. Every user action and every helper event.
#[derive(Debug, Clone)]
pub enum Message {
    // ── Chrome / nav / app lifecycle ──────────────────────────────────────
    NavSelect(Page),
    ToggleAbout,
    LaunchUrl(String),
    Quit,
    /// Re-run local `detect_system()` (Ctrl+R / F5). Does not ReadState.
    RefreshSystem,
    SystemDetected(SystemInfo),
    DismissToast(cosmic::widget::ToastId),
    DismissDialog,
    /// cosmic-config / prefs file changed on disk.
    UpdateConfig(UserPreferences),
    ClipboardCopied {
        ok: bool,
    },

    // ── Onboarding ────────────────────────────────────────────────────────
    CopyIntegrationSnippet,
    OpenEtcNixos,
    VerifyIntegration,

    // ── Profiles ──────────────────────────────────────────────────────────
    SelectProfile(String),
    /// Kept for tests; no UI control (GTK has none).
    ClearProfile,
    RefreshProfilePreview,

    // ── Bundles ───────────────────────────────────────────────────────────
    ToggleBundle {
        id: String,
        enabled: bool,
    },
    ToggleBundlePackage {
        bundle_id: String,
        package: String,
        enabled: bool,
    },
    ExpandBundle {
        id: String,
        expanded: bool,
    },

    // ── Custom packages ───────────────────────────────────────────────────
    PackageInputChanged(String),
    AddPackagesFromInput,
    AddCustomPackages(Vec<String>),
    RemoveCustomPackage(String),

    // ── System settings ───────────────────────────────────────────────────
    HostnameChanged(String),
    DnsServersChanged(String),
    UsernameChanged(String),
    ToggleUserGroup {
        group: String,
        enabled: bool,
    },
    ColorSchemeChanged(ColorSchemePreference),

    // ── Hardware ──────────────────────────────────────────────────────────
    SetNvidiaDriver(Option<u8>),
    SetNvidiaModesetting(bool),
    SetNvidiaPowerManagement(bool),
    SetNvidiaOpen(bool),
    SetAudioServer(u8),
    SetAudioLowLatency(bool),
    SetBluetoothEnabled(bool),
    SetBluetoothAutoPower(bool),
    SetPowerProfile(u8),
    SetTlpEnabled(bool),
    SetThermaldEnabled(bool),
    GpuDetected(String),

    // ── Network ───────────────────────────────────────────────────────────
    SetFirewallEnabled(bool),
    ToggleTcpPort {
        port: u16,
        enabled: bool,
    },
    CustomTcpPortsChanged(String),
    ToggleUdpPort {
        port: u16,
        enabled: bool,
    },
    CustomUdpPortsChanged(String),
    SetSshEnabled(bool),
    SetSshPort(u16),
    SetSshPasswordAuth(bool),
    /// Combo index: 0 = no, 1 = prohibit-password, 2 = yes.
    SetSshRootLogin(u8),
    SetFail2banEnabled(bool),
    SetTailscaleEnabled(bool),
    SetWireguardEnabled(bool),
    SetWireguardListenPort(u16),

    // ── Services ──────────────────────────────────────────────────────────
    ToggleService {
        id: String,
        enabled: bool,
    },

    // ── Apply / preview / dry-run ─────────────────────────────────────────
    RefreshPreview,
    PreviewReady(String),
    RequestApply,
    ConfirmApply,
    CancelApply,
    RequestDryRun,
    SetRebuildType(RebuildType),
    ApplyFinished {
        success: bool,
        message: String,
    },

    // ── Generations ───────────────────────────────────────────────────────
    LoadGenerations,
    GenerationsLoaded(Vec<Generation>),
    RequestRollback {
        generation: u32,
    },
    ConfirmRollback {
        generation: u32,
        mode: RollbackMode,
    },
    RequestDeleteGeneration {
        generation: u32,
    },
    ConfirmDeleteGeneration {
        generation: u32,
    },
    RollbackToPrevious,

    // ── Maintenance ───────────────────────────────────────────────────────
    LoadDiskUsage,
    DiskUsageLoaded(DiskUsageInfo),
    RequestMaintenance {
        id: String,
    },
    ConfirmMaintenance {
        id: String,
    },

    // ── Helper session ────────────────────────────────────────────────────
    Helper(HelperEvent),
}

/// How generation activation should run after switching generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackMode {
    SwitchNow,
    SetForNextBoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Onboarding,
    Profiles,
    Bundles,
    Packages,
    System,
    Hardware,
    Network,
    Services,
    Generations,
    Maintenance,
    Apply,
}

impl Page {
    pub const ALL: [Page; 11] = [
        Page::Onboarding,
        Page::Profiles,
        Page::Bundles,
        Page::Packages,
        Page::System,
        Page::Hardware,
        Page::Network,
        Page::Services,
        Page::Generations,
        Page::Maintenance,
        Page::Apply,
    ];

    #[must_use]
    pub fn title(self) -> String {
        match self {
            Self::Onboarding => crate::fl!("nav-onboarding"),
            Self::Profiles => crate::fl!("nav-profiles"),
            Self::Bundles => crate::fl!("nav-bundles"),
            Self::Packages => crate::fl!("nav-packages"),
            Self::System => crate::fl!("nav-system"),
            Self::Hardware => crate::fl!("nav-hardware"),
            Self::Network => crate::fl!("nav-network"),
            Self::Services => crate::fl!("nav-services"),
            Self::Generations => crate::fl!("nav-generations"),
            Self::Maintenance => crate::fl!("nav-maintenance"),
            Self::Apply => crate::fl!("nav-apply"),
        }
    }

    #[must_use]
    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Onboarding => "go-home-symbolic",
            Self::Profiles => "user-desktop-symbolic",
            Self::Bundles => "package-x-generic-symbolic",
            Self::Packages => "list-add-symbolic",
            Self::System => "preferences-system-symbolic",
            Self::Hardware => "video-display-symbolic",
            Self::Network => "network-workgroup-symbolic",
            Self::Services => "system-run-symbolic",
            Self::Generations => "document-open-recent-symbolic",
            Self::Maintenance => "user-trash-symbolic",
            Self::Apply => "emblem-synchronizing-symbolic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextPage {
    #[default]
    About,
}

/// Events from a live helper process.
#[derive(Debug, Clone)]
pub enum HelperEvent {
    SpawnFailed {
        op: HelperOp,
        error: String,
    },
    Spawned {
        op: HelperOp,
    },
    Response {
        op: HelperOp,
        response: Box<HelperResponse>,
    },
    Closed {
        op: HelperOp,
        error: Option<String>,
    },
    Timeout {
        op: HelperOp,
    },
}

/// Why a helper was spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum HelperOp {
    ReadState,
    WriteState,
    CheckPermissions,
    GetSystemInfo,
    EnsureDirectories,
    Validate,
    Generate,
    /// `then_write_state` is true for Switch/Boot/Test/Build (files persist);
    /// false for DryBuild. `save` is written on the same session after a successful Apply.
    Apply {
        rebuild: RebuildType,
        then_write_state: bool,
        save: Option<Box<IpcAppState>>,
    },
    ListGenerations,
    Rollback {
        generation: u32,
        mode: RollbackMode,
    },
    DeleteGenerations {
        generations: Vec<u32>,
    },
    RunMaintenance {
        command: String,
    },
    GetDiskUsage,
}

/// Modal dialogs rendered by the application chrome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    ConfirmApply {
        heading: String,
        body: String,
        destructive: bool,
    },
    ConfirmRollback {
        generation: u32,
    },
    ConfirmDeleteGeneration {
        generation: u32,
    },
    ConfirmMaintenance {
        id: String,
        name: String,
        warning: String,
    },
}

/// Side effects `apply()` returns. Tests inspect these instead of spawning.
///
/// `HelperRequest` does not implement `PartialEq`, so this enum does not either.
#[derive(Debug, Clone)]
pub enum Intent {
    None,
    SpawnHelper {
        op: HelperOp,
        request: HelperRequest,
    },
    SendOnSession {
        request: HelperRequest,
    },
    CloseSession,
    DetectSystem,
    DetectGpu,
    LocalPreview,
    LocalProfilePreview {
        id: String,
    },
    CopyClipboard(String),
    OpenPath(PathBuf),
    OpenUrl(String),
    SavePrefs,
    ApplyTheme,
    SetWindowTitle(String),
    ShowToast {
        text: String,
        timeout_ms: u64,
    },
    Exit,
}

impl Intent {
    #[must_use]
    pub fn is_exit(&self) -> bool {
        matches!(self, Self::Exit)
    }
}
