use std::sync::atomic::{AtomicU32, Ordering};

use super::{
    command_ids, Command, CommandId, CommandPayload, NativeTextInputDialogRequest, UiRect,
};

pub(crate) const SETTINGS_W: i32 = 1100;
pub(crate) const SETTINGS_H: i32 = 740;
pub(crate) const SETTINGS_NAV_W: i32 = 236;
pub(crate) const SETTINGS_TOP_H: i32 = 84;
pub(crate) const SETTINGS_NAV_Y: i32 = 72;
pub(crate) const SETTINGS_CONTENT_X: i32 = SETTINGS_NAV_W + 28;
pub(crate) const SETTINGS_PAGE_LABELS: [&str; 6] =
    ["常规", "快捷键", "插件", "分组", "多端同步", "关于"];
pub(crate) const SETTINGS_NATIVE_TAB_GENERAL_SECTIONS: [&str; 2] =
    ["settings_summary", "settings_controls"];
pub(crate) const SETTINGS_NATIVE_TAB_GROUPS_SECTIONS: [&str; 2] =
    ["group_selector", "group_actions"];
pub(crate) const SETTINGS_NATIVE_TAB_ACTIONS_SECTIONS: [&str; 3] =
    ["settings_actions", "platform_actions", "dialog_actions"];

static SETTINGS_UI_DPI: AtomicU32 = AtomicU32::new(96);

pub(crate) fn set_settings_ui_dpi(dpi: u32) {
    SETTINGS_UI_DPI.store(dpi.max(96), Ordering::Relaxed);
}

pub(crate) fn settings_ui_dpi() -> u32 {
    SETTINGS_UI_DPI.load(Ordering::Relaxed).max(96)
}

pub(crate) fn settings_scale(value: i32) -> i32 {
    let dpi = settings_ui_dpi() as i64;
    (((value as i64) * dpi) + 48) as i32 / 96
}

pub(crate) fn settings_w_scaled() -> i32 {
    settings_scale(SETTINGS_W)
}

pub(crate) fn settings_h_scaled() -> i32 {
    settings_scale(SETTINGS_H)
}

pub(crate) fn settings_nav_w_scaled() -> i32 {
    settings_scale(SETTINGS_NAV_W)
}

pub(crate) fn settings_top_h_scaled() -> i32 {
    settings_scale(SETTINGS_TOP_H)
}

pub(crate) fn settings_content_x_scaled() -> i32 {
    settings_scale(SETTINGS_CONTENT_X)
}

pub(crate) fn settings_content_w_scaled() -> i32 {
    settings_w_scaled() - settings_content_x_scaled() - settings_scale(28)
}

pub(crate) fn settings_content_y_scaled() -> i32 {
    settings_top_h_scaled()
}

pub(crate) fn settings_nav_item_rect(index: usize) -> UiRect {
    let x = settings_scale(10);
    let y = settings_scale(SETTINGS_NAV_Y + 8 + (index as i32) * 44);
    UiRect::new(
        x,
        y,
        settings_nav_w_scaled() - settings_scale(10),
        y + settings_scale(36),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsControlRole {
    Save,
    Close,
    OpenConfig,
    Dropdown,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeSettingsPageTabKind {
    General,
    Groups,
    Actions,
}

impl NativeSettingsPageTabKind {
    pub(crate) const fn tab_id(self) -> &'static str {
        match self {
            Self::General => "settings.tab.general",
            Self::Groups => "settings.tab.groups",
            Self::Actions => "settings.tab.actions",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Groups => "Groups",
            Self::Actions => "Actions",
        }
    }

    pub(crate) const fn section_names(self) -> &'static [&'static str] {
        match self {
            Self::General => &SETTINGS_NATIVE_TAB_GENERAL_SECTIONS,
            Self::Groups => &SETTINGS_NATIVE_TAB_GROUPS_SECTIONS,
            Self::Actions => &SETTINGS_NATIVE_TAB_ACTIONS_SECTIONS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSettingsPageTabSpec {
    pub(crate) kind: NativeSettingsPageTabKind,
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) section_names: &'static [&'static str],
}

impl NativeSettingsPageTabSpec {
    pub(crate) const fn new(kind: NativeSettingsPageTabKind) -> Self {
        Self {
            kind,
            id: kind.tab_id(),
            label: kind.label(),
            section_names: kind.section_names(),
        }
    }
}

pub(crate) const fn native_host_settings_page_tab_specs() -> [NativeSettingsPageTabSpec; 3] {
    [
        NativeSettingsPageTabSpec::new(NativeSettingsPageTabKind::General),
        NativeSettingsPageTabSpec::new(NativeSettingsPageTabKind::Groups),
        NativeSettingsPageTabSpec::new(NativeSettingsPageTabKind::Actions),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSettingsSectionSpec {
    pub(crate) section_name: &'static str,
    pub(crate) tab_kind: NativeSettingsPageTabKind,
    pub(crate) label: &'static str,
}

pub(crate) const fn native_host_settings_section_specs() -> [NativeSettingsSectionSpec; 7] {
    [
        NativeSettingsSectionSpec {
            section_name: "settings_summary",
            tab_kind: NativeSettingsPageTabKind::General,
            label: "Settings Summary",
        },
        NativeSettingsSectionSpec {
            section_name: "settings_controls",
            tab_kind: NativeSettingsPageTabKind::General,
            label: "Shared Controls",
        },
        NativeSettingsSectionSpec {
            section_name: "group_selector",
            tab_kind: NativeSettingsPageTabKind::Groups,
            label: "Group Management",
        },
        NativeSettingsSectionSpec {
            section_name: "group_actions",
            tab_kind: NativeSettingsPageTabKind::Groups,
            label: "Group Actions",
        },
        NativeSettingsSectionSpec {
            section_name: "settings_actions",
            tab_kind: NativeSettingsPageTabKind::Actions,
            label: "Settings Actions",
        },
        NativeSettingsSectionSpec {
            section_name: "platform_actions",
            tab_kind: NativeSettingsPageTabKind::Actions,
            label: "Platform Actions",
        },
        NativeSettingsSectionSpec {
            section_name: "dialog_actions",
            tab_kind: NativeSettingsPageTabKind::Actions,
            label: "Dialog Actions",
        },
    ]
}

pub(crate) fn native_host_settings_section_label(section_name: &str) -> Option<&'static str> {
    native_host_settings_section_specs()
        .into_iter()
        .find(|spec| spec.section_name == section_name)
        .map(|spec| spec.label)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsAction {
    ToggleHotkeyRecording,
    AddGroup,
    RenameGroup,
    DeleteGroup,
    MoveGroupUp,
    MoveGroupDown,
    GroupSelectionChanged,
    ShowRecordGroups,
    ShowPhraseGroups,
    PickPasteSound,
    CaptureSkippedWindowClass,
    RestoreSearchEnginePreset,
    DetectOcrRuntime,
    OpenMailMerge,
    OpenWpsTaskpaneDocs,
    OpenSourceRepository,
    CheckForUpdates,
    DisableSystemClipboardHistory,
    EnableSystemClipboardHistory,
    RestartSystemShell,
    SyncWebDavNow,
    UploadWebDavConfig,
    ApplyWebDavConfig,
    RestoreWebDavBackup,
    RefreshLanDevices,
    PairLanDevice,
    AcceptLanPairing,
    RejectLanPairing,
    CopyLanPairUrl,
    CopyLanSetupUrl,
    OpenLanSetupPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsActionRoute {
    Sync,
    Group,
    Platform,
}

pub(crate) const SETTINGS_LAN_SYNC_FEATURE_NAME: &str = "sync_lan";
pub(crate) const SETTINGS_LAN_SYNC_RUNTIME_GAP: &str =
    "LAN sync needs a platform service runtime boundary before this settings action can run";
pub(crate) const SETTINGS_LAN_TCP_PORT_DEFAULT: u16 = 38473;
pub(crate) const SETTINGS_LAN_DESKTOP_CAPABILITIES: [&str; 5] =
    ["text", "image", "latest", "manual_file", "receive_clip"];
pub(crate) const SETTINGS_LAN_SYNC_ACTIONS: [SettingsAction; 7] = [
    SettingsAction::RefreshLanDevices,
    SettingsAction::PairLanDevice,
    SettingsAction::AcceptLanPairing,
    SettingsAction::RejectLanPairing,
    SettingsAction::CopyLanPairUrl,
    SettingsAction::CopyLanSetupUrl,
    SettingsAction::OpenLanSetupPage,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsLanSyncRuntimeBoundary {
    ServiceDiscovery,
    PairingService,
    PairApprovalStore,
    MobileLinkProjection,
}

impl SettingsLanSyncRuntimeBoundary {
    pub(crate) const fn boundary_name(self) -> &'static str {
        match self {
            Self::ServiceDiscovery => "service_discovery",
            Self::PairingService => "pairing_service",
            Self::PairApprovalStore => "pair_approval_store",
            Self::MobileLinkProjection => "mobile_link_projection",
        }
    }

    pub(crate) const fn missing_requirement(self) -> &'static str {
        match self {
            Self::ServiceDiscovery => {
                "platform LAN service lifecycle, UDP discovery, and device cache refresh"
            }
            Self::PairingService => {
                "platform LAN pairing client/server runtime and manual host dispatch"
            }
            Self::PairApprovalStore => {
                "platform pending-pair store projection and accept/reject mutation"
            }
            Self::MobileLinkProjection => {
                "platform LAN mobile setup URL projection plus clipboard/shell-open handoff"
            }
        }
    }

    pub(crate) const fn required_host_capability_names(self) -> &'static [&'static str] {
        match self {
            Self::ServiceDiscovery => &["lan_service_lifecycle", "lan_udp_discovery"],
            Self::PairingService => &["lan_service_lifecycle", "lan_pairing_client"],
            Self::PairApprovalStore => &["lan_pending_pair_store", "lan_device_book_store"],
            Self::MobileLinkProjection => &["lan_mobile_url_projection", "clipboard_or_shell_open"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanSyncActionSupportPlan {
    pub(crate) action: SettingsAction,
    pub(crate) action_name: &'static str,
    pub(crate) feature_name: &'static str,
    pub(crate) runtime_boundary: SettingsLanSyncRuntimeBoundary,
    pub(crate) runtime_boundary_name: &'static str,
    pub(crate) required_host_capability_names: &'static [&'static str],
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
    pub(crate) missing_runtime_boundary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsLanMobileLinkProjectionStatus {
    Ready,
    LanDisabled,
    MissingHost,
    UnsupportedAction,
}

impl SettingsLanMobileLinkProjectionStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::Ready => "mobile_link_projected",
            Self::LanDisabled => "lan_disabled",
            Self::MissingHost => "missing_lan_host",
            Self::UnsupportedAction => "unsupported_lan_mobile_link_action",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanMobileLinkProjection {
    pub(crate) action: SettingsAction,
    pub(crate) action_name: &'static str,
    pub(crate) status: SettingsLanMobileLinkProjectionStatus,
    pub(crate) status_name: &'static str,
    pub(crate) host: Option<String>,
    pub(crate) setup_url: Option<String>,
    pub(crate) pair_url: Option<String>,
    pub(crate) target_url: Option<String>,
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanDeviceProjection {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) endpoint: String,
    pub(crate) last_seen_ms: u64,
    pub(crate) trusted: bool,
    pub(crate) can_receive_clip: bool,
    pub(crate) capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanDeviceBookProjection {
    pub(crate) action: SettingsAction,
    pub(crate) action_name: &'static str,
    pub(crate) device_count: usize,
    pub(crate) trusted_device_count: usize,
    pub(crate) receivable_device_count: usize,
    pub(crate) devices: Vec<SettingsLanDeviceProjection>,
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsLanPairRequestProjectionStatus {
    Ready,
    LanDisabled,
    MissingHost,
    MissingDeviceIdentity,
    UnsupportedAction,
}

impl SettingsLanPairRequestProjectionStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::Ready => "pair_request_ready",
            Self::LanDisabled => "lan_disabled",
            Self::MissingHost => "missing_lan_host",
            Self::MissingDeviceIdentity => "missing_device_identity",
            Self::UnsupportedAction => "unsupported_lan_pair_action",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanPairRequestProjection {
    pub(crate) action: SettingsAction,
    pub(crate) action_name: &'static str,
    pub(crate) status: SettingsLanPairRequestProjectionStatus,
    pub(crate) status_name: &'static str,
    pub(crate) host: Option<String>,
    pub(crate) device_id: Option<String>,
    pub(crate) device_name: Option<String>,
    pub(crate) tcp_port: u16,
    pub(crate) request_body_json: Option<String>,
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsLanPairRequestResponseStatus {
    Sent,
    InvalidResponse,
}

impl SettingsLanPairRequestResponseStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::InvalidResponse => "invalid_response",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanPairRequestResponseProjection {
    pub(crate) status: SettingsLanPairRequestResponseStatus,
    pub(crate) status_name: &'static str,
    pub(crate) pair_id: Option<String>,
    pub(crate) code: Option<String>,
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsLanPairStatusProjectionStatus {
    Pending,
    Accepted,
    Rejected,
    Missing,
    InvalidResponse,
}

impl SettingsLanPairStatusProjectionStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Missing => "missing",
            Self::InvalidResponse => "invalid_response",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanAcceptedDeviceProjection {
    pub(crate) device: SettingsLanDeviceProjection,
    pub(crate) token: String,
    pub(crate) addr: String,
    pub(crate) tcp_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsLanPairStatusProjection {
    pub(crate) status: SettingsLanPairStatusProjectionStatus,
    pub(crate) status_name: &'static str,
    pub(crate) accepted_device: Option<SettingsLanAcceptedDeviceProjection>,
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
}

pub(crate) const fn settings_lan_sync_action_name(action: SettingsAction) -> Option<&'static str> {
    match action {
        SettingsAction::RefreshLanDevices => Some("refresh_lan_devices"),
        SettingsAction::PairLanDevice => Some("pair_lan_device"),
        SettingsAction::AcceptLanPairing => Some("accept_lan_pairing"),
        SettingsAction::RejectLanPairing => Some("reject_lan_pairing"),
        SettingsAction::CopyLanPairUrl => Some("copy_lan_pair_url"),
        SettingsAction::CopyLanSetupUrl => Some("copy_lan_setup_url"),
        SettingsAction::OpenLanSetupPage => Some("open_lan_setup_page"),
        _ => None,
    }
}

pub(crate) const fn settings_lan_sync_runtime_boundary(
    action: SettingsAction,
) -> Option<SettingsLanSyncRuntimeBoundary> {
    match action {
        SettingsAction::RefreshLanDevices => Some(SettingsLanSyncRuntimeBoundary::ServiceDiscovery),
        SettingsAction::PairLanDevice => Some(SettingsLanSyncRuntimeBoundary::PairingService),
        SettingsAction::AcceptLanPairing | SettingsAction::RejectLanPairing => {
            Some(SettingsLanSyncRuntimeBoundary::PairApprovalStore)
        }
        SettingsAction::CopyLanPairUrl
        | SettingsAction::CopyLanSetupUrl
        | SettingsAction::OpenLanSetupPage => {
            Some(SettingsLanSyncRuntimeBoundary::MobileLinkProjection)
        }
        _ => None,
    }
}

pub(crate) fn settings_lan_sync_action_support_plan(
    action: SettingsAction,
    platform_host_name: &str,
    support_status_name: &str,
) -> Option<SettingsLanSyncActionSupportPlan> {
    let action_name = settings_lan_sync_action_name(action)?;
    let runtime_boundary = settings_lan_sync_runtime_boundary(action)?;
    Some(SettingsLanSyncActionSupportPlan {
        action,
        action_name,
        feature_name: SETTINGS_LAN_SYNC_FEATURE_NAME,
        runtime_boundary,
        runtime_boundary_name: runtime_boundary.boundary_name(),
        required_host_capability_names: runtime_boundary.required_host_capability_names(),
        accepted: false,
        result_name: format!(
            "zsclip.settings_sync.{action_name}.{support_status_name}_on_{platform_host_name}"
        ),
        missing_runtime_boundary: runtime_boundary.missing_requirement(),
    })
}

pub(crate) fn settings_lan_mobile_link_projection_from_json(
    action: SettingsAction,
    platform_host_name: &str,
    settings_json: &serde_json::Value,
) -> Option<SettingsLanMobileLinkProjection> {
    let action_name = settings_lan_sync_action_name(action)?;
    let status = if !settings_lan_json_bool(settings_json, "lan_sync_enabled") {
        SettingsLanMobileLinkProjectionStatus::LanDisabled
    } else if !matches!(
        action,
        SettingsAction::CopyLanPairUrl
            | SettingsAction::CopyLanSetupUrl
            | SettingsAction::OpenLanSetupPage
    ) {
        SettingsLanMobileLinkProjectionStatus::UnsupportedAction
    } else if settings_lan_json_string(settings_json, "lan_manual_host")
        .map(|host| settings_lan_normalize_host(&host, SETTINGS_LAN_TCP_PORT_DEFAULT))
        .filter(|host| !host.trim().is_empty())
        .is_none()
    {
        SettingsLanMobileLinkProjectionStatus::MissingHost
    } else {
        SettingsLanMobileLinkProjectionStatus::Ready
    };
    let status_name = status.status_name();
    let host = if status == SettingsLanMobileLinkProjectionStatus::Ready {
        settings_lan_json_string(settings_json, "lan_manual_host")
            .map(|host| {
                settings_lan_normalize_host(
                    &host,
                    settings_lan_json_u16(settings_json, "lan_tcp_port")
                        .unwrap_or(SETTINGS_LAN_TCP_PORT_DEFAULT),
                )
            })
            .filter(|host| !host.trim().is_empty())
    } else {
        None
    };
    let setup_url = host
        .as_ref()
        .map(|host| format!("http://{host}/mobile/setup"));
    let pair_url = host
        .as_ref()
        .map(|host| format!("zsclip://pair?host={}", settings_lan_url_encode(host)));
    let target_url = match action {
        SettingsAction::CopyLanPairUrl => pair_url.clone(),
        SettingsAction::CopyLanSetupUrl | SettingsAction::OpenLanSetupPage => setup_url.clone(),
        _ => None,
    };

    Some(SettingsLanMobileLinkProjection {
        action,
        action_name,
        status,
        status_name,
        host,
        setup_url,
        pair_url,
        target_url,
        accepted: status == SettingsLanMobileLinkProjectionStatus::Ready,
        result_name: format!(
            "zsclip.settings_sync.{action_name}.{status_name}_on_{platform_host_name}"
        ),
    })
}

pub(crate) fn settings_lan_device_book_projection(
    platform_host_name: &str,
    devices: Vec<SettingsLanDeviceProjection>,
) -> SettingsLanDeviceBookProjection {
    let trusted_device_count = devices.iter().filter(|device| device.trusted).count();
    let receivable_device_count = devices
        .iter()
        .filter(|device| device.can_receive_clip)
        .count();
    let device_count = devices.len();
    SettingsLanDeviceBookProjection {
        action: SettingsAction::RefreshLanDevices,
        action_name: "refresh_lan_devices",
        device_count,
        trusted_device_count,
        receivable_device_count,
        devices,
        accepted: true,
        result_name: format!(
            "zsclip.settings_sync.refresh_lan_devices.device_book_projected_{device_count}_on_{platform_host_name}"
        ),
    }
}

pub(crate) fn settings_lan_device_projection(
    device_id: impl Into<String>,
    name: impl Into<String>,
    addr: impl AsRef<str>,
    tcp_port: u16,
    last_seen_ms: u64,
    trusted: bool,
    capabilities: Vec<String>,
) -> SettingsLanDeviceProjection {
    let addr = addr.as_ref().trim();
    let endpoint = if addr.is_empty() || tcp_port == 0 {
        addr.to_string()
    } else {
        format!("{addr}:{tcp_port}")
    };
    let can_receive_clip = trusted
        && tcp_port > 0
        && !capabilities.iter().any(|capability| {
            capability.eq_ignore_ascii_case("client_only")
                || capability.eq_ignore_ascii_case("pull_only")
        });
    SettingsLanDeviceProjection {
        device_id: device_id.into(),
        name: name.into(),
        endpoint,
        last_seen_ms,
        trusted,
        can_receive_clip,
        capabilities,
    }
}

pub(crate) fn settings_lan_pair_request_projection_from_json(
    action: SettingsAction,
    platform_host_name: &str,
    settings_json: &serde_json::Value,
) -> Option<SettingsLanPairRequestProjection> {
    let action_name = settings_lan_sync_action_name(action)?;
    let tcp_port = settings_lan_json_u16(settings_json, "lan_tcp_port")
        .unwrap_or(SETTINGS_LAN_TCP_PORT_DEFAULT);
    let host = settings_lan_json_string(settings_json, "lan_manual_host")
        .map(|host| settings_lan_normalize_host(&host, tcp_port))
        .filter(|host| !host.trim().is_empty());
    let device_id = settings_lan_json_string(settings_json, "lan_device_id");
    let device_name = settings_lan_json_string(settings_json, "lan_device_name");
    let status = if action != SettingsAction::PairLanDevice {
        SettingsLanPairRequestProjectionStatus::UnsupportedAction
    } else if !settings_lan_json_bool(settings_json, "lan_sync_enabled") {
        SettingsLanPairRequestProjectionStatus::LanDisabled
    } else if host.is_none() {
        SettingsLanPairRequestProjectionStatus::MissingHost
    } else if device_id.is_none() || device_name.is_none() {
        SettingsLanPairRequestProjectionStatus::MissingDeviceIdentity
    } else {
        SettingsLanPairRequestProjectionStatus::Ready
    };
    let request_body_json = if status == SettingsLanPairRequestProjectionStatus::Ready {
        Some(
            serde_json::json!({
                "device_id": device_id.as_deref().unwrap_or_default(),
                "name": device_name.as_deref().unwrap_or_default(),
                "tcp_port": tcp_port,
                "capabilities": SETTINGS_LAN_DESKTOP_CAPABILITIES,
            })
            .to_string(),
        )
    } else {
        None
    };
    let status_name = status.status_name();
    Some(SettingsLanPairRequestProjection {
        action,
        action_name,
        status,
        status_name,
        host,
        device_id,
        device_name,
        tcp_port,
        request_body_json,
        accepted: status == SettingsLanPairRequestProjectionStatus::Ready,
        result_name: format!(
            "zsclip.settings_sync.{action_name}.{status_name}_on_{platform_host_name}"
        ),
    })
}

pub(crate) fn settings_lan_pair_request_response_projection(
    result_prefix: &str,
    response: &[u8],
) -> SettingsLanPairRequestResponseProjection {
    let response_json = serde_json::from_slice::<serde_json::Value>(response).ok();
    let pair_id = response_json
        .as_ref()
        .and_then(|value| value.get("pair_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let code = response_json
        .as_ref()
        .and_then(|value| value.get("code"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let status = if pair_id.is_some() && code.is_some() {
        SettingsLanPairRequestResponseStatus::Sent
    } else {
        SettingsLanPairRequestResponseStatus::InvalidResponse
    };
    let status_name = status.status_name();
    SettingsLanPairRequestResponseProjection {
        status,
        status_name,
        pair_id,
        code,
        accepted: status == SettingsLanPairRequestResponseStatus::Sent,
        result_name: format!("{result_prefix}.{status_name}"),
    }
}

pub(crate) fn settings_lan_pair_status_projection(
    result_prefix: &str,
    host: &str,
    fallback_tcp_port: u16,
    last_seen_ms: u64,
    response: &[u8],
) -> SettingsLanPairStatusProjection {
    let response_json = serde_json::from_slice::<serde_json::Value>(response).ok();
    let status_text = response_json
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let status = match status_text {
        "pending" => SettingsLanPairStatusProjectionStatus::Pending,
        "accepted" => SettingsLanPairStatusProjectionStatus::Accepted,
        "rejected" => SettingsLanPairStatusProjectionStatus::Rejected,
        "missing" => SettingsLanPairStatusProjectionStatus::Missing,
        _ => SettingsLanPairStatusProjectionStatus::InvalidResponse,
    };
    let accepted_device = if status == SettingsLanPairStatusProjectionStatus::Accepted {
        settings_lan_accepted_device_projection_from_json(
            host,
            fallback_tcp_port,
            last_seen_ms,
            response_json.as_ref(),
        )
    } else {
        None
    };
    let final_status =
        if status == SettingsLanPairStatusProjectionStatus::Accepted && accepted_device.is_none() {
            SettingsLanPairStatusProjectionStatus::InvalidResponse
        } else {
            status
        };
    let status_name = final_status.status_name();
    SettingsLanPairStatusProjection {
        status: final_status,
        status_name,
        accepted: final_status == SettingsLanPairStatusProjectionStatus::Accepted,
        accepted_device,
        result_name: format!("{result_prefix}.{status_name}"),
    }
}

fn settings_lan_accepted_device_projection_from_json(
    host: &str,
    fallback_tcp_port: u16,
    last_seen_ms: u64,
    response_json: Option<&serde_json::Value>,
) -> Option<SettingsLanAcceptedDeviceProjection> {
    let value = response_json?;
    let device_id = value
        .get("device_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let token = value
        .get("token")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("ZSClip");
    let tcp_port = value
        .get("tcp_port")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback_tcp_port);
    let capabilities = value
        .get("capabilities")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let addr = host.split(':').next().unwrap_or_default().to_string();
    let device = settings_lan_device_projection(
        device_id,
        name,
        addr.as_str(),
        tcp_port,
        last_seen_ms,
        true,
        capabilities,
    );
    Some(SettingsLanAcceptedDeviceProjection {
        device,
        token: token.to_string(),
        addr,
        tcp_port,
    })
}

fn settings_lan_json_bool(settings_json: &serde_json::Value, key: &str) -> bool {
    settings_json
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or_default()
}

fn settings_lan_json_string(settings_json: &serde_json::Value, key: &str) -> Option<String> {
    settings_json
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn settings_lan_json_u16(settings_json: &serde_json::Value, key: &str) -> Option<u16> {
    settings_json
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn settings_lan_normalize_host(raw: &str, default_port: u16) -> String {
    let mut host = raw.trim().to_string();
    if let Some(rest) = host.strip_prefix("http://") {
        host = rest.to_string();
    }
    if let Some(rest) = host.strip_prefix("https://") {
        host = rest.to_string();
    }
    if let Some((value, _path)) = host.split_once('/') {
        host = value.to_string();
    }
    if host.is_empty() {
        return String::new();
    }
    if host.contains(':') {
        host
    } else {
        format!("{host}:{default_port}")
    }
}

fn settings_lan_url_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

pub(crate) const fn settings_action_route(action: SettingsAction) -> SettingsActionRoute {
    match action {
        SettingsAction::SyncWebDavNow
        | SettingsAction::UploadWebDavConfig
        | SettingsAction::ApplyWebDavConfig
        | SettingsAction::RestoreWebDavBackup
        | SettingsAction::RefreshLanDevices
        | SettingsAction::PairLanDevice
        | SettingsAction::AcceptLanPairing
        | SettingsAction::RejectLanPairing
        | SettingsAction::CopyLanPairUrl
        | SettingsAction::CopyLanSetupUrl
        | SettingsAction::OpenLanSetupPage => SettingsActionRoute::Sync,
        SettingsAction::AddGroup
        | SettingsAction::RenameGroup
        | SettingsAction::DeleteGroup
        | SettingsAction::MoveGroupUp
        | SettingsAction::MoveGroupDown
        | SettingsAction::GroupSelectionChanged
        | SettingsAction::ShowRecordGroups
        | SettingsAction::ShowPhraseGroups => SettingsActionRoute::Group,
        SettingsAction::ToggleHotkeyRecording
        | SettingsAction::PickPasteSound
        | SettingsAction::CaptureSkippedWindowClass
        | SettingsAction::RestoreSearchEnginePreset
        | SettingsAction::DetectOcrRuntime
        | SettingsAction::OpenMailMerge
        | SettingsAction::OpenWpsTaskpaneDocs
        | SettingsAction::OpenSourceRepository
        | SettingsAction::CheckForUpdates
        | SettingsAction::DisableSystemClipboardHistory
        | SettingsAction::EnableSystemClipboardHistory
        | SettingsAction::RestartSystemShell => SettingsActionRoute::Platform,
    }
}

pub(crate) fn settings_action_for_route(
    route_name: &str,
    action_name: &str,
) -> Option<SettingsAction> {
    let action = match (route_name, action_name) {
        ("settings_sync", "sync_webdav_now") => SettingsAction::SyncWebDavNow,
        ("settings_sync", "upload_webdav_config") => SettingsAction::UploadWebDavConfig,
        ("settings_sync", "apply_webdav_config") => SettingsAction::ApplyWebDavConfig,
        ("settings_sync", "restore_webdav_backup") => SettingsAction::RestoreWebDavBackup,
        ("settings_sync", "refresh_lan_devices") => SettingsAction::RefreshLanDevices,
        ("settings_sync", "pair_lan_device") => SettingsAction::PairLanDevice,
        ("settings_sync", "accept_lan_pairing") => SettingsAction::AcceptLanPairing,
        ("settings_sync", "reject_lan_pairing") => SettingsAction::RejectLanPairing,
        ("settings_sync", "copy_lan_pair_url") => SettingsAction::CopyLanPairUrl,
        ("settings_sync", "copy_lan_setup_url") => SettingsAction::CopyLanSetupUrl,
        ("settings_sync", "open_lan_setup_page") => SettingsAction::OpenLanSetupPage,
        ("settings_group", "show_record_groups") => SettingsAction::ShowRecordGroups,
        ("settings_group", "show_phrase_groups") => SettingsAction::ShowPhraseGroups,
        ("settings_group", "add_group") => SettingsAction::AddGroup,
        ("settings_group", "rename_group") => SettingsAction::RenameGroup,
        ("settings_group", "delete_group") => SettingsAction::DeleteGroup,
        ("settings_group", "move_group_up") => SettingsAction::MoveGroupUp,
        ("settings_group", "move_group_down") => SettingsAction::MoveGroupDown,
        ("settings_platform", "toggle_hotkey_recording") => SettingsAction::ToggleHotkeyRecording,
        ("settings_platform", "pick_paste_sound") => SettingsAction::PickPasteSound,
        ("settings_platform", "capture_skipped_window_class") => {
            SettingsAction::CaptureSkippedWindowClass
        }
        ("settings_platform", "restore_search_engine_preset") => {
            SettingsAction::RestoreSearchEnginePreset
        }
        ("settings_platform", "detect_ocr_runtime") => SettingsAction::DetectOcrRuntime,
        ("settings_platform", "open_mail_merge") => SettingsAction::OpenMailMerge,
        ("settings_platform", "open_wps_taskpane_docs") => SettingsAction::OpenWpsTaskpaneDocs,
        ("settings_platform", "open_source_repository") => SettingsAction::OpenSourceRepository,
        ("settings_platform", "check_for_updates") => SettingsAction::CheckForUpdates,
        ("settings_platform", "disable_system_clipboard_history") => {
            SettingsAction::DisableSystemClipboardHistory
        }
        ("settings_platform", "enable_system_clipboard_history") => {
            SettingsAction::EnableSystemClipboardHistory
        }
        ("settings_platform", "restart_system_shell") => SettingsAction::RestartSystemShell,
        _ => return None,
    };
    Some(action)
}

pub(crate) trait SettingsActionExecutor {
    type Context;

    fn execute_sync(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool;

    fn execute_group(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool;

    fn execute_platform(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool;
}

pub(crate) fn dispatch_settings_action<E: SettingsActionExecutor>(
    executor: &mut E,
    context: &mut E::Context,
    action: SettingsAction,
) -> bool {
    match settings_action_route(action) {
        SettingsActionRoute::Sync => executor.execute_sync(context, action),
        SettingsActionRoute::Group => executor.execute_group(context, action),
        SettingsActionRoute::Platform => executor.execute_platform(context, action),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsGroupTextInputKind {
    Add,
    Rename,
}

pub(crate) fn settings_group_text_input_request<'a>(
    kind: SettingsGroupTextInputKind,
    current_name: &'a str,
) -> NativeTextInputDialogRequest<'a> {
    match kind {
        SettingsGroupTextInputKind::Add => NativeTextInputDialogRequest {
            title: "新建分组",
            label: "请输入分组名称：",
            initial: "新分组",
        },
        SettingsGroupTextInputKind::Rename => NativeTextInputDialogRequest {
            title: "重命名分组",
            label: "请输入新名称：",
            initial: current_name,
        },
    }
}

pub(crate) fn settings_command_id_for_role(role: SettingsControlRole) -> CommandId {
    match role {
        SettingsControlRole::Save => command_ids::SAVE_SETTINGS,
        SettingsControlRole::Close => command_ids::CLOSE_SETTINGS,
        SettingsControlRole::OpenConfig => command_ids::OPEN_SETTINGS_CONFIG,
        SettingsControlRole::Dropdown => command_ids::OPEN_SETTINGS_DROPDOWN,
        SettingsControlRole::Toggle => command_ids::TOGGLE_SETTINGS_CONTROL,
    }
}

pub(crate) fn settings_command_for_control_role(
    role: SettingsControlRole,
    control_id: i64,
) -> Command {
    let id = settings_command_id_for_role(role);
    match role {
        SettingsControlRole::Dropdown | SettingsControlRole::Toggle => {
            Command::window_with_payload(id, CommandPayload::ControlId(control_id))
        }
        SettingsControlRole::Save
        | SettingsControlRole::Close
        | SettingsControlRole::OpenConfig => Command::window(id),
    }
}
