use super::layout_protocol::{SharedUiProtocol, SHARED_NON_HOST_UI_PROTOCOLS};
use super::native_adapter_manifest::{
    native_ui_adapter_parity_report, native_ui_backend_capability_matrix_for_platform,
    NativeUiAdapterCapability, NativeUiAdapterParityReport, NativeUiAdapterReusePackage,
    NativeUiBackendDescriptor, NativeUiBackendStatus, NativeUiPlatform, NativeUiToolkit,
    REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES, SUPPORTED_NATIVE_UI_BACKENDS,
    SUPPORTED_NATIVE_UI_PLATFORMS, SUPPORTED_NATIVE_UI_TOOLKITS,
};
use super::native_component_protocol::{
    native_component_family_descriptors, native_ui_protocol_surfaces,
    NativeComponentFamilyDescriptor, NativeUiProtocolSurface, NativeUiProtocolSurfaceKind,
};
use super::native_hosts::required_native_runtime_driver_operation_names;
use super::product_adapter::{
    product_adapter_execution_pipeline, product_adapter_function_flows,
    product_adapter_integration_contract, product_adapter_reuse_checklist,
    product_ai_integration_manifest, required_product_adapter_host_method_names,
    ProductAdapterFunctionFlow, ProductAdapterIntegrationContract, ProductAdapterIntegrationTask,
    ProductAdapterPipelineStage, ProductAiExecutorBoundary, ProductAiIntegrationManifest,
};
use super::runtime_protocol::zsui_reusable_runtime_harness_stage_names;
use super::settings_protocol::SETTINGS_LAN_SYNC_RUNTIME_GAP;
use super::ui_surface_protocol::{UiHostSurface, REQUIRED_UI_HOST_SURFACES};
use super::zsui::{ApiVersion, APP_CORE_API_VERSION, ZSUI_FRAMEWORK_NAME, ZSUI_FRAMEWORK_TAGLINE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiFrameworkManifest {
    pub(crate) name: &'static str,
    pub(crate) tagline: &'static str,
    pub(crate) api_version: ApiVersion,
    pub(crate) framework_layers: Vec<ZsuiFrameworkLayer>,
    pub(crate) boundary_rules: Vec<ZsuiFrameworkBoundaryRule>,
    pub(crate) native_feature_parity: Vec<ZsuiNativeFeatureParityStatus>,
    pub(crate) clipboard_mvp_feature_matrix: Vec<ZsuiClipboardMvpFeatureStatus>,
    pub(crate) lan_sync_capability_matrix: Vec<ZsuiLanSyncCapabilityStatus>,
    pub(crate) window_system_capability_matrix: Vec<ZsuiWindowSystemCapabilityStatus>,
    pub(crate) window_system_backend_work_items: Vec<ZsuiWindowSystemBackendWorkItem>,
    pub(crate) user_feature_platform_statuses: Vec<ZsuiUserFeaturePlatformStatus>,
    pub(crate) user_feature_completion_summaries: Vec<ZsuiUserFeatureCompletionSummary>,
    pub(crate) user_feature_cross_platform_summaries: Vec<ZsuiUserFeatureCrossPlatformSummary>,
    pub(crate) user_feature_release_progress: ZsuiUserFeatureReleaseProgress,
    pub(crate) ui_ingress_classifications: Vec<ZsuiUiIngressClassification>,
    pub(crate) ui_extension_recipes: Vec<ZsuiUiExtensionRecipe>,
    pub(crate) native_component_families: Vec<NativeComponentFamilyDescriptor>,
    pub(crate) native_feature_ui_ingress_requirements: Vec<ZsuiNativeFeatureUiIngressRequirement>,
    pub(crate) ui_protocol_convergence: Vec<ZsuiUiProtocolConvergenceStatus>,
    pub(crate) native_ui_protocol_surfaces: Vec<NativeUiProtocolSurface>,
    pub(crate) native_ui_protocol_host_statuses: Vec<ZsuiNativeUiProtocolHostStatus>,
    pub(crate) host_private_ui_ingress_audits: Vec<ZsuiHostPrivateUiIngressAudit>,
    pub(crate) native_ui_host_translation_work_items: Vec<ZsuiNativeUiHostTranslationWorkItem>,
    pub(crate) native_target_smoke_work_items: Vec<ZsuiNativeTargetSmokeWorkItem>,
    pub(crate) native_platforms: Vec<NativeUiPlatform>,
    pub(crate) native_toolkits: Vec<NativeUiToolkit>,
    pub(crate) native_backends: Vec<NativeUiBackendDescriptor>,
    pub(crate) native_adapter_capabilities: Vec<NativeUiAdapterCapability>,
    pub(crate) required_host_surfaces: Vec<UiHostSurface>,
    pub(crate) shared_non_host_protocols: Vec<SharedUiProtocol>,
    pub(crate) product_adapter: ProductAdapterIntegrationContract,
    pub(crate) ai: ProductAiIntegrationManifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiFrameworkLayer {
    CoreContracts,
    LayoutAndRenderPlans,
    AdapterBoundary,
    NativeHost,
    ProductAdapter,
}

impl ZsuiFrameworkLayer {
    pub(crate) const fn layer_name(self) -> &'static str {
        match self {
            Self::CoreContracts => "core_contracts",
            Self::LayoutAndRenderPlans => "layout_and_render_plans",
            Self::AdapterBoundary => "adapter_boundary",
            Self::NativeHost => "native_host",
            Self::ProductAdapter => "product_adapter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiFrameworkBoundaryRule {
    pub(crate) layer: ZsuiFrameworkLayer,
    pub(crate) owner_name: &'static str,
    pub(crate) allowed_modules: Vec<&'static str>,
    pub(crate) owns: Vec<&'static str>,
    pub(crate) must_not_own: Vec<&'static str>,
    pub(crate) handoff_to: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeFeatureParityStatus {
    pub(crate) feature_name: &'static str,
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) support_status: ZsuiNativeFeatureSupportStatus,
    pub(crate) support_status_name: &'static str,
    pub(crate) code_level_ready: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) target_smoke_verified: bool,
    pub(crate) system_complete: bool,
    pub(crate) missing_system_requirements: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiNativeFeatureSupportStatus {
    CodeLevelReadyPendingTargetSmoke,
    PartiallyCodeReadyPendingTargetSmoke,
    PlannedNotImplemented,
    UnsupportedByDesign,
    TargetSmokeVerified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiUserFeatureHostMaturity {
    ProtocolReadyPendingHost,
    HostPartialPendingWork,
    HostUsablePendingTargetSmoke,
    TargetSmokeVerified,
    UnsupportedByDesign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiClipboardMvpPhase {
    P0SameClipboardLoop,
    P1SystemIntegration,
    P2AdvancedInteraction,
}

impl ZsuiClipboardMvpPhase {
    pub(crate) const fn phase_name(self) -> &'static str {
        match self {
            Self::P0SameClipboardLoop => "p0_same_clipboard_loop",
            Self::P1SystemIntegration => "p1_system_integration",
            Self::P2AdvancedInteraction => "p2_advanced_interaction",
        }
    }

    pub(crate) const fn blocks_cross_platform_mvp(self) -> bool {
        matches!(self, Self::P0SameClipboardLoop)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiClipboardMvpFeatureStatus {
    pub(crate) feature_name: &'static str,
    pub(crate) phase: ZsuiClipboardMvpPhase,
    pub(crate) phase_name: &'static str,
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) required_native_feature_names: Vec<&'static str>,
    pub(crate) code_level_ready: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) target_smoke_verified: bool,
    pub(crate) system_complete: bool,
    pub(crate) blocks_cross_platform_mvp: bool,
    pub(crate) next_missing_requirement: Option<&'static str>,
    pub(crate) missing_system_requirements: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiLanSyncCapabilityStatus {
    pub(crate) capability_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) runtime_boundary_name: &'static str,
    pub(crate) support_status: ZsuiNativeFeatureSupportStatus,
    pub(crate) support_status_name: &'static str,
    pub(crate) code_level_ready: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) system_complete: bool,
    pub(crate) blocks_sync_lan_release: bool,
    pub(crate) evidence_names: Vec<&'static str>,
    pub(crate) missing_system_requirements: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiWindowSystemCapabilityStatus {
    pub(crate) capability_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) support_status: ZsuiNativeFeatureSupportStatus,
    pub(crate) support_status_name: &'static str,
    pub(crate) code_level_ready: bool,
    pub(crate) partial_code_level_ready: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) system_complete: bool,
    pub(crate) evidence_names: Vec<&'static str>,
    pub(crate) implemented_backend_names: Vec<&'static str>,
    pub(crate) missing_backend_requirements: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUserFeaturePlatformStatus {
    pub(crate) user_feature_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) support_status_name: &'static str,
    pub(crate) required_native_feature_names: Vec<&'static str>,
    pub(crate) ui_ingress_names: Vec<&'static str>,
    pub(crate) native_component_family_names: Vec<&'static str>,
    pub(crate) typed_component_spec_names: Vec<&'static str>,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) code_level_ready: bool,
    pub(crate) host_maturity_name: &'static str,
    pub(crate) host_maturity_percent: u8,
    pub(crate) host_usable: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) target_smoke_verified: bool,
    pub(crate) system_complete: bool,
    pub(crate) next_missing_requirement: Option<&'static str>,
    pub(crate) missing_system_requirements: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUserFeatureCompletionSummary {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) total_user_feature_count: usize,
    pub(crate) code_level_ready_count: usize,
    pub(crate) host_usable_count: usize,
    pub(crate) planned_not_implemented_count: usize,
    pub(crate) target_smoke_required_count: usize,
    pub(crate) system_complete_count: usize,
    pub(crate) code_level_ready_percent: u8,
    pub(crate) host_usable_percent: u8,
    pub(crate) system_complete_percent: u8,
    pub(crate) next_user_feature_name: Option<&'static str>,
    pub(crate) next_missing_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUserFeatureCrossPlatformSummary {
    pub(crate) user_feature_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) total_platform_count: usize,
    pub(crate) code_level_ready_count: usize,
    pub(crate) host_usable_count: usize,
    pub(crate) planned_not_implemented_count: usize,
    pub(crate) target_smoke_required_count: usize,
    pub(crate) system_complete_count: usize,
    pub(crate) code_level_ready_percent: u8,
    pub(crate) host_usable_percent: u8,
    pub(crate) system_complete_percent: u8,
    pub(crate) code_level_ready_platform_names: Vec<&'static str>,
    pub(crate) host_usable_platform_names: Vec<&'static str>,
    pub(crate) planned_platform_names: Vec<&'static str>,
    pub(crate) target_smoke_required_platform_names: Vec<&'static str>,
    pub(crate) system_complete_platform_names: Vec<&'static str>,
    pub(crate) next_platform_name: Option<&'static str>,
    pub(crate) next_missing_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUserFeatureProgressReport {
    pub(crate) user_feature_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) cross_platform_summary: ZsuiUserFeatureCrossPlatformSummary,
    pub(crate) platform_statuses: Vec<ZsuiUserFeaturePlatformStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUserFeatureReleaseProgress {
    pub(crate) total_platform_feature_slots: usize,
    pub(crate) code_level_ready_slots: usize,
    pub(crate) host_usable_slots: usize,
    pub(crate) planned_not_implemented_slots: usize,
    pub(crate) target_smoke_required_slots: usize,
    pub(crate) system_complete_slots: usize,
    pub(crate) non_windows_host_slots: usize,
    pub(crate) non_windows_host_code_level_ready_slots: usize,
    pub(crate) non_windows_host_usable_slots: usize,
    pub(crate) non_windows_host_code_gap_slots: usize,
    pub(crate) non_windows_host_system_complete_slots: usize,
    pub(crate) code_level_ready_percent: u8,
    pub(crate) host_usable_percent: u8,
    pub(crate) non_windows_host_usable_percent: u8,
    pub(crate) system_complete_percent: u8,
    pub(crate) next_platform_name: Option<&'static str>,
    pub(crate) next_user_feature_name: Option<&'static str>,
    pub(crate) next_display_name: Option<&'static str>,
    pub(crate) next_ui_ingress_names: Vec<&'static str>,
    pub(crate) next_native_component_family_names: Vec<&'static str>,
    pub(crate) next_typed_component_spec_names: Vec<&'static str>,
    pub(crate) next_preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) next_platform_host_module_paths: Vec<&'static str>,
    pub(crate) next_missing_requirement: Option<&'static str>,
    pub(crate) next_host_platform_name: Option<&'static str>,
    pub(crate) next_host_user_feature_name: Option<&'static str>,
    pub(crate) next_host_display_name: Option<&'static str>,
    pub(crate) next_host_ui_ingress_names: Vec<&'static str>,
    pub(crate) next_host_native_component_family_names: Vec<&'static str>,
    pub(crate) next_host_typed_component_spec_names: Vec<&'static str>,
    pub(crate) next_host_preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) next_host_module_paths: Vec<&'static str>,
    pub(crate) next_host_missing_requirement: Option<&'static str>,
    pub(crate) next_host_code_gap_platform_name: Option<&'static str>,
    pub(crate) next_host_code_gap_user_feature_name: Option<&'static str>,
    pub(crate) next_host_code_gap_display_name: Option<&'static str>,
    pub(crate) next_host_code_gap_ui_ingress_names: Vec<&'static str>,
    pub(crate) next_host_code_gap_native_component_family_names: Vec<&'static str>,
    pub(crate) next_host_code_gap_typed_component_spec_names: Vec<&'static str>,
    pub(crate) next_host_code_gap_preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) next_host_code_gap_module_paths: Vec<&'static str>,
    pub(crate) next_host_code_gap_missing_requirement: Option<&'static str>,
}

impl ZsuiNativeFeatureSupportStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::CodeLevelReadyPendingTargetSmoke => "code_level_ready_pending_target_smoke",
            Self::PartiallyCodeReadyPendingTargetSmoke => {
                "partially_code_ready_pending_target_smoke"
            }
            Self::PlannedNotImplemented => "planned_not_implemented",
            Self::UnsupportedByDesign => "unsupported_by_design",
            Self::TargetSmokeVerified => "target_smoke_verified",
        }
    }

    pub(crate) const fn code_level_ready(self) -> bool {
        matches!(
            self,
            Self::CodeLevelReadyPendingTargetSmoke | Self::TargetSmokeVerified
        )
    }

    pub(crate) const fn target_smoke_required(self) -> bool {
        matches!(
            self,
            Self::CodeLevelReadyPendingTargetSmoke | Self::PartiallyCodeReadyPendingTargetSmoke
        )
    }

    pub(crate) const fn target_smoke_verified(self) -> bool {
        matches!(self, Self::TargetSmokeVerified)
    }

    pub(crate) const fn system_complete(self) -> bool {
        matches!(self, Self::TargetSmokeVerified)
    }

    pub(crate) const fn is_explicitly_unsupported(self) -> bool {
        matches!(self, Self::UnsupportedByDesign)
    }
}

impl ZsuiUserFeatureHostMaturity {
    pub(crate) const fn maturity_name(self) -> &'static str {
        match self {
            Self::ProtocolReadyPendingHost => "protocol_ready_pending_host",
            Self::HostPartialPendingWork => "host_partial_pending_work",
            Self::HostUsablePendingTargetSmoke => "host_usable_pending_target_smoke",
            Self::TargetSmokeVerified => "target_smoke_verified",
            Self::UnsupportedByDesign => "unsupported_by_design",
        }
    }

    pub(crate) const fn maturity_percent(self) -> u8 {
        match self {
            Self::ProtocolReadyPendingHost => 35,
            Self::HostPartialPendingWork => 60,
            Self::HostUsablePendingTargetSmoke => 80,
            Self::TargetSmokeVerified => 100,
            Self::UnsupportedByDesign => 0,
        }
    }

    pub(crate) const fn host_usable(self) -> bool {
        matches!(
            self,
            Self::HostUsablePendingTargetSmoke | Self::TargetSmokeVerified
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeUiProtocolHostStatus {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) backend_status_name: &'static str,
    pub(crate) host_module_path: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) protocol_builder_names: Vec<&'static str>,
    pub(crate) dynamic_protocol_builder_names: Vec<&'static str>,
    pub(crate) action_family_names: Vec<&'static str>,
    pub(crate) source_guard_required: bool,
    pub(crate) source_coverage_verified: bool,
    pub(crate) missing_protocol_builder_names: Vec<&'static str>,
    pub(crate) target_smoke_required: bool,
    pub(crate) target_smoke_verified: bool,
    pub(crate) system_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeUiHostTranslationWorkItem {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) host_module_path: &'static str,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) platform_host_module_paths: Vec<&'static str>,
    pub(crate) protocol_builder_names: Vec<&'static str>,
    pub(crate) dynamic_protocol_builder_names: Vec<&'static str>,
    pub(crate) action_family_names: Vec<&'static str>,
    pub(crate) source_coverage_verified: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) system_complete: bool,
    pub(crate) next_missing_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiHostPrivateUiIngressAuditStatus {
    ProtocolAnchoredHostChromeOnly,
    ProtocolAnchoredNeedsExtraction,
    PrivateIngressNeedsProtocol,
}

impl ZsuiHostPrivateUiIngressAuditStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::ProtocolAnchoredHostChromeOnly => "protocol_anchored_host_chrome_only",
            Self::ProtocolAnchoredNeedsExtraction => "protocol_anchored_needs_extraction",
            Self::PrivateIngressNeedsProtocol => "private_ingress_needs_protocol",
        }
    }

    pub(crate) const fn needs_protocol_work(self) -> bool {
        matches!(
            self,
            Self::ProtocolAnchoredNeedsExtraction | Self::PrivateIngressNeedsProtocol
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiHostPrivateUiIngressAudit {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) host_module_path: &'static str,
    pub(crate) native_entry_markers: Vec<&'static str>,
    pub(crate) protocol_anchor_names: Vec<&'static str>,
    pub(crate) private_native_entry_names: Vec<&'static str>,
    pub(crate) audit_status: ZsuiHostPrivateUiIngressAuditStatus,
    pub(crate) audit_status_name: &'static str,
    pub(crate) next_protocolization_step: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeTargetSmokeWorkItem {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) target_environment_name: &'static str,
    pub(crate) user_feature_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) support_status_name: &'static str,
    pub(crate) ui_ingress_names: Vec<&'static str>,
    pub(crate) native_component_family_names: Vec<&'static str>,
    pub(crate) typed_component_spec_names: Vec<&'static str>,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) platform_host_module_paths: Vec<&'static str>,
    pub(crate) required_native_feature_names: Vec<&'static str>,
    pub(crate) target_smoke_name: &'static str,
    pub(crate) target_smoke_steps: Vec<&'static str>,
    pub(crate) code_level_ready: bool,
    pub(crate) target_smoke_required: bool,
    pub(crate) target_smoke_verified: bool,
    pub(crate) system_complete: bool,
    pub(crate) next_missing_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUiProtocolConvergenceStatus {
    pub(crate) ingress_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) category_name: &'static str,
    pub(crate) protocolized: bool,
    pub(crate) total_builder_count: usize,
    pub(crate) static_builder_count: usize,
    pub(crate) dynamic_builder_count: usize,
    pub(crate) action_family_count: usize,
    pub(crate) platform_host_count: usize,
    pub(crate) source_covered_platform_count: usize,
    pub(crate) source_gap_platform_count: usize,
    pub(crate) target_smoke_required_platform_count: usize,
    pub(crate) system_complete_platform_count: usize,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) platform_adapter_touchpoints: Vec<&'static str>,
    pub(crate) next_platform_name: Option<&'static str>,
    pub(crate) next_missing_protocol_builder_name: Option<&'static str>,
    pub(crate) next_missing_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUiIngressClassification {
    pub(crate) ingress_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) category_name: &'static str,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) protocol_builder_names: Vec<&'static str>,
    pub(crate) dynamic_protocol_builder_names: Vec<&'static str>,
    pub(crate) action_family_names: Vec<&'static str>,
    pub(crate) platform_host_scope: &'static str,
    pub(crate) platform_adapter_touchpoints: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUiExtensionRecipe {
    pub(crate) recipe_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) action_family_name: &'static str,
    pub(crate) primary_app_core_modules: Vec<&'static str>,
    pub(crate) action_enum_names: Vec<&'static str>,
    pub(crate) spec_builder_names: Vec<&'static str>,
    pub(crate) dynamic_spec_builder_names: Vec<&'static str>,
    pub(crate) platform_host_touchpoints: Vec<&'static str>,
    pub(crate) expected_edit_order: Vec<&'static str>,
    pub(crate) required_test_focus: Vec<&'static str>,
    pub(crate) must_not_edit_first: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeFeatureUiIngressRequirement {
    pub(crate) feature_name: &'static str,
    pub(crate) ingress_names: Vec<&'static str>,
    pub(crate) native_component_family_names: Vec<&'static str>,
    pub(crate) typed_component_spec_names: Vec<&'static str>,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) protocol_builder_names: Vec<&'static str>,
    pub(crate) dynamic_protocol_builder_names: Vec<&'static str>,
    pub(crate) action_family_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiReuseReadinessReport {
    pub(crate) platform_names: Vec<&'static str>,
    pub(crate) native_runtime_ready_platforms: Vec<&'static str>,
    pub(crate) first_pass_native_host_platforms: Vec<&'static str>,
    pub(crate) scaffold_platforms: Vec<&'static str>,
    pub(crate) native_adapter_capability_names: Vec<&'static str>,
    pub(crate) product_adapter_surface_names: Vec<&'static str>,
    pub(crate) product_adapter_task_names: Vec<&'static str>,
    pub(crate) ai_provider_names: Vec<&'static str>,
    pub(crate) ai_executor_boundary_names: Vec<&'static str>,
    pub(crate) adapter_parity: Option<NativeUiAdapterParityReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeRuntimeGateCapabilityPlan {
    pub(crate) gate_name: &'static str,
    pub(crate) required_adapter_capability_names: Vec<&'static str>,
    pub(crate) required_product_adapter_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeRuntimeGatePlatformBindingPlan {
    pub(crate) gate_name: &'static str,
    pub(crate) required_adapter_capability_names: Vec<&'static str>,
    pub(crate) required_platform_binding_names: Vec<&'static str>,
    pub(crate) required_product_adapter_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiNativeRuntimeGateCompletionReport {
    pub(crate) total_gate_count: usize,
    pub(crate) completed_gate_count: usize,
    pub(crate) missing_gate_count: usize,
    pub(crate) completion_percent: u8,
    pub(crate) completed_gate_names: Vec<&'static str>,
    pub(crate) missing_gate_names: Vec<&'static str>,
    pub(crate) next_gate_name: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAdapterReusePackageGateBindingSummary {
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) status_name: &'static str,
    pub(crate) adapter_boundary: &'static str,
    pub(crate) gate_names: Vec<&'static str>,
    pub(crate) gate_binding_counts: Vec<usize>,
    pub(crate) missing_gate_names: Vec<&'static str>,
    pub(crate) next_gate_name: Option<&'static str>,
    pub(crate) completion_percent: u8,
    pub(crate) all_gate_bindings_present_in_adapter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAdapterPortingWorkItem {
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) status_name: &'static str,
    pub(crate) adapter_boundary: &'static str,
    pub(crate) adapter_module_path: &'static str,
    pub(crate) gate_name: &'static str,
    pub(crate) required_adapter_capability_names: Vec<&'static str>,
    pub(crate) required_platform_binding_names: Vec<&'static str>,
    pub(crate) required_product_adapter_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiWindowSystemBackendWorkItem {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) capability_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) support_status_name: &'static str,
    pub(crate) backend_trait_name: &'static str,
    pub(crate) default_backend_name: &'static str,
    pub(crate) platform_host_module_paths: Vec<&'static str>,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) implemented_backend_names: Vec<&'static str>,
    pub(crate) backend_option_names: Vec<&'static str>,
    pub(crate) missing_backend_requirements: Vec<&'static str>,
    pub(crate) next_backend_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiUserFeatureWorkItem {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) user_feature_name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) support_status_name: &'static str,
    pub(crate) ui_ingress_names: Vec<&'static str>,
    pub(crate) native_component_family_names: Vec<&'static str>,
    pub(crate) typed_component_spec_names: Vec<&'static str>,
    pub(crate) preferred_app_core_edit_modules: Vec<&'static str>,
    pub(crate) platform_host_module_paths: Vec<&'static str>,
    pub(crate) required_native_feature_names: Vec<&'static str>,
    pub(crate) next_missing_requirement: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiReusableAppFeatureRequirement {
    pub(crate) feature_name: &'static str,
    pub(crate) required_runtime_gate_names: Vec<&'static str>,
    pub(crate) required_adapter_capability_names: Vec<&'static str>,
    pub(crate) required_product_adapter_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiReusableAppFeaturePlatformStatus {
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) backend_status_name: &'static str,
    pub(crate) adapter_module_path: &'static str,
    pub(crate) feature_name: &'static str,
    pub(crate) runtime_status_name: &'static str,
    pub(crate) runtime_ready: bool,
    pub(crate) required_runtime_gate_names: Vec<&'static str>,
    pub(crate) blocking_runtime_gate_names: Vec<&'static str>,
    pub(crate) required_platform_binding_names: Vec<&'static str>,
    pub(crate) required_product_adapter_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiReusableAppBlueprint {
    pub(crate) framework_name: &'static str,
    pub(crate) api_version: ApiVersion,
    pub(crate) rust_ui_language_name: &'static str,
    pub(crate) native_platform_names: Vec<&'static str>,
    pub(crate) native_runtime_driver_operation_names: Vec<&'static str>,
    pub(crate) runtime_harness_stage_names: Vec<&'static str>,
    pub(crate) reusable_feature_names: Vec<&'static str>,
    pub(crate) product_adapter_surface_names: Vec<&'static str>,
    pub(crate) product_adapter_task_names: Vec<&'static str>,
    pub(crate) product_adapter_method_names: Vec<&'static str>,
    pub(crate) product_function_flows: Vec<ProductAdapterFunctionFlow>,
    pub(crate) product_execution_pipeline: Vec<ProductAdapterPipelineStage>,
    pub(crate) ai_executor_boundary_names: Vec<&'static str>,
    pub(crate) feature_statuses: Vec<ZsuiReusableAppFeaturePlatformStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiReuseBootstrapPlan {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) backend_status: NativeUiBackendStatus,
    pub(crate) backend_status_name: &'static str,
    pub(crate) adapter_boundary: &'static str,
    pub(crate) adapter_module_path: &'static str,
    pub(crate) native_adapter_capability_names: Vec<&'static str>,
    pub(crate) product_adapter_surface_names: Vec<&'static str>,
    pub(crate) product_adapter_task_names: Vec<&'static str>,
    pub(crate) ai_provider_names: Vec<&'static str>,
    pub(crate) ai_executor_boundary_names: Vec<&'static str>,
    pub(crate) native_runtime_gate_names: Vec<&'static str>,
    pub(crate) missing_native_runtime_gate_names: Vec<&'static str>,
    pub(crate) next_native_runtime_gate_name: Option<&'static str>,
    pub(crate) native_runtime_gate_plans: Vec<ZsuiNativeRuntimeGateCapabilityPlan>,
    pub(crate) native_runtime_gate_binding_plans: Vec<ZsuiNativeRuntimeGatePlatformBindingPlan>,
    pub(crate) native_runtime_gate_completion: ZsuiNativeRuntimeGateCompletionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAgentAiRouteSummary {
    pub(crate) capability_id: &'static str,
    pub(crate) provider_name: &'static str,
    pub(crate) executor_boundary_name: &'static str,
    pub(crate) executor_task_name: &'static str,
    pub(crate) action_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) input_context_names: Vec<&'static str>,
    pub(crate) result_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAgentPlatformBootstrapSummary {
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) backend_status_name: &'static str,
    pub(crate) adapter_boundary: &'static str,
    pub(crate) adapter_module_path: &'static str,
    pub(crate) native_adapter_capability_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAgentPlatformRuntimeGate {
    pub(crate) platform_name: &'static str,
    pub(crate) toolkit_name: &'static str,
    pub(crate) backend_status_name: &'static str,
    pub(crate) native_runtime_ready: bool,
    pub(crate) gate_names: Vec<&'static str>,
    pub(crate) missing_gate_names: Vec<&'static str>,
    pub(crate) next_gate_name: Option<&'static str>,
    pub(crate) gate_plans: Vec<ZsuiNativeRuntimeGateCapabilityPlan>,
    pub(crate) gate_binding_plans: Vec<ZsuiNativeRuntimeGatePlatformBindingPlan>,
    pub(crate) completion: ZsuiNativeRuntimeGateCompletionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAgentIntegrationStep {
    pub(crate) step_name: &'static str,
    pub(crate) owner_name: &'static str,
    pub(crate) required_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsuiAgentContext {
    pub(crate) framework_name: &'static str,
    pub(crate) api_version: ApiVersion,
    pub(crate) framework_layers: Vec<ZsuiFrameworkLayer>,
    pub(crate) boundary_rules: Vec<ZsuiFrameworkBoundaryRule>,
    pub(crate) native_feature_parity: Vec<ZsuiNativeFeatureParityStatus>,
    pub(crate) clipboard_mvp_feature_matrix: Vec<ZsuiClipboardMvpFeatureStatus>,
    pub(crate) lan_sync_capability_matrix: Vec<ZsuiLanSyncCapabilityStatus>,
    pub(crate) window_system_capability_matrix: Vec<ZsuiWindowSystemCapabilityStatus>,
    pub(crate) window_system_backend_work_items: Vec<ZsuiWindowSystemBackendWorkItem>,
    pub(crate) user_feature_platform_statuses: Vec<ZsuiUserFeaturePlatformStatus>,
    pub(crate) user_feature_completion_summaries: Vec<ZsuiUserFeatureCompletionSummary>,
    pub(crate) user_feature_cross_platform_summaries: Vec<ZsuiUserFeatureCrossPlatformSummary>,
    pub(crate) user_feature_release_progress: ZsuiUserFeatureReleaseProgress,
    pub(crate) ui_ingress_classifications: Vec<ZsuiUiIngressClassification>,
    pub(crate) ui_extension_recipes: Vec<ZsuiUiExtensionRecipe>,
    pub(crate) native_component_families: Vec<NativeComponentFamilyDescriptor>,
    pub(crate) native_feature_ui_ingress_requirements: Vec<ZsuiNativeFeatureUiIngressRequirement>,
    pub(crate) ui_protocol_convergence: Vec<ZsuiUiProtocolConvergenceStatus>,
    pub(crate) native_ui_protocol_surfaces: Vec<NativeUiProtocolSurface>,
    pub(crate) native_ui_protocol_host_statuses: Vec<ZsuiNativeUiProtocolHostStatus>,
    pub(crate) host_private_ui_ingress_audits: Vec<ZsuiHostPrivateUiIngressAudit>,
    pub(crate) native_ui_host_translation_work_items: Vec<ZsuiNativeUiHostTranslationWorkItem>,
    pub(crate) native_target_smoke_work_items: Vec<ZsuiNativeTargetSmokeWorkItem>,
    pub(crate) readiness: ZsuiReuseReadinessReport,
    pub(crate) platform_bootstrap: Vec<ZsuiAgentPlatformBootstrapSummary>,
    pub(crate) platform_runtime_gates: Vec<ZsuiAgentPlatformRuntimeGate>,
    pub(crate) porting_work_items: Vec<ZsuiAdapterPortingWorkItem>,
    pub(crate) user_feature_work_items: Vec<ZsuiUserFeatureWorkItem>,
    pub(crate) reusable_app_blueprint: ZsuiReusableAppBlueprint,
    pub(crate) integration_steps: Vec<ZsuiAgentIntegrationStep>,
    pub(crate) ai_routes: Vec<ZsuiAgentAiRouteSummary>,
}

impl ZsuiReuseBootstrapPlan {
    pub(crate) fn native_runtime_ready(&self) -> bool {
        self.backend_status.is_native_runtime_ready()
    }

    pub(crate) fn scaffolded(&self) -> bool {
        self.backend_status.is_scaffold()
    }
}

pub(crate) fn zsui_framework_layers() -> Vec<ZsuiFrameworkLayer> {
    vec![
        ZsuiFrameworkLayer::CoreContracts,
        ZsuiFrameworkLayer::LayoutAndRenderPlans,
        ZsuiFrameworkLayer::AdapterBoundary,
        ZsuiFrameworkLayer::NativeHost,
        ZsuiFrameworkLayer::ProductAdapter,
    ]
}

pub(crate) fn zsui_framework_boundary_rules() -> Vec<ZsuiFrameworkBoundaryRule> {
    use ZsuiFrameworkLayer::{
        AdapterBoundary, CoreContracts, LayoutAndRenderPlans, NativeHost, ProductAdapter,
    };

    vec![
        ZsuiFrameworkBoundaryRule {
            layer: CoreContracts,
            owner_name: "app_core_contracts",
            allowed_modules: vec![
                "src/app_core.rs",
                "src/app_core/command_protocol.rs",
                "src/app_core/component_protocol.rs",
                "src/app_core/control_protocol.rs",
                "src/app_core/event_protocol.rs",
                "src/app_core/host_protocol.rs",
                "src/app_core/native_hosts.rs",
                "src/app_core/runtime_protocol.rs",
                "src/app_core/ui_surface_protocol.rs",
                "src/app_core/zsui.rs",
            ],
            owns: vec![
                "stable command ids",
                "platform-neutral events",
                "host traits",
                "required host surfaces",
                "runtime driver contract",
            ],
            must_not_own: vec![
                "AppKit objects",
                "GTK widgets",
                "Win32 handles",
                "ZSClip database access",
                "clipboard history side effects",
            ],
            handoff_to: vec![
                "layout_and_render_plans",
                "adapter_boundary",
                "product_adapter",
            ],
        },
        ZsuiFrameworkBoundaryRule {
            layer: LayoutAndRenderPlans,
            owner_name: "shared_ui_plans",
            allowed_modules: vec![
                "src/app_core/layout_protocol.rs",
                "src/app_core/main_window.rs",
                "src/app_core/main_window_protocol.rs",
                "src/app_core/render_protocol.rs",
                "src/app_core/settings_protocol.rs",
                "src/app_core/timer_protocol.rs",
                "src/app_core/native_host_actions.rs",
            ],
            owns: vec![
                "pure geometry",
                "hit testing",
                "semantic render commands",
                "settings layout plans",
                "row and VV action plans",
            ],
            must_not_own: vec![
                "native window creation",
                "native menu presentation",
                "native clipboard writes",
                "database mutations",
                "network sync",
            ],
            handoff_to: vec!["native_host", "product_adapter"],
        },
        ZsuiFrameworkBoundaryRule {
            layer: AdapterBoundary,
            owner_name: "native_adapter_boundary",
            allowed_modules: vec![
                "src/app_core/framework_manifest.rs",
                "src/app_core/native_adapter_manifest.rs",
                "src/windows_win32_adapter.rs",
                "src/macos_appkit_adapter.rs",
                "src/linux_gtk_adapter.rs",
            ],
            owns: vec![
                "backend descriptors",
                "platform binding names",
                "capability parity",
                "runtime gate binding plans",
                "reuse bootstrap packages",
            ],
            must_not_own: vec![
                "product command execution",
                "clipboard item storage",
                "settings persistence",
                "AppKit event-loop side effects",
                "GTK event-loop side effects",
            ],
            handoff_to: vec!["native_host", "product_adapter"],
        },
        ZsuiFrameworkBoundaryRule {
            layer: NativeHost,
            owner_name: "platform_native_host",
            allowed_modules: vec![
                "src/app",
                "src/platform",
                "src/windows_win32_adapter.rs",
                "src/macos_app.rs",
                "src/macos_native_host.rs",
                "src/linux_app.rs",
                "src/linux_native_host.rs",
            ],
            owns: vec![
                "native windows",
                "native controls",
                "native menus",
                "native dialogs",
                "native clipboard services",
                "native rendering",
                "platform event loop",
            ],
            must_not_own: vec![
                "shared layout math",
                "product data model",
                "row command semantics",
                "AI provider clients",
                "sync transport logic",
            ],
            handoff_to: vec!["core_contracts", "product_adapter"],
        },
        ZsuiFrameworkBoundaryRule {
            layer: ProductAdapter,
            owner_name: "zsclip_product_adapter",
            allowed_modules: vec![
                "src/app_core/product_adapter.rs",
                "src/zsclip_product_adapter.rs",
                "src/app/main_row_commands.rs",
                "src/app/settings_actions.rs",
                "src/app/settings_sync_actions.rs",
                "src/app/settings_group_actions.rs",
            ],
            owns: vec![
                "product identity",
                "state projection",
                "product command execution",
                "settings persistence",
                "async event bridge",
                "AI capability catalog",
            ],
            must_not_own: vec![
                "native widget construction",
                "native menu presentation",
                "platform event-loop ownership",
                "renderer implementation",
                "platform hotkey registration",
            ],
            handoff_to: vec!["core_contracts", "adapter_boundary", "native_host"],
        },
    ]
}

pub(crate) fn zsui_native_feature_parity_statuses() -> Vec<ZsuiNativeFeatureParityStatus> {
    let ready_pending_smoke = ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke;
    let rows = [
        (
            "main_window_db_rows",
            ready_pending_smoke,
            &[
                "target visual and interaction screenshots",
                "native window chrome, visual effect background, and titlebar integration smoke",
                "native deactivation hide and status item restore smoke",
                "native accessibility labels for primary controls and row surfaces",
            ][..],
        ),
        (
            "window_system_integration",
            ready_pending_smoke,
            &[
                "target always-on-top smoke",
                "target dark appearance detection smoke",
                "target DPI scale factor smoke",
                "target cursor-follow positioning smoke",
            ][..],
        ),
        (
            "startup_autostart",
            ready_pending_smoke,
            &[
                "target startup/autostart install smoke",
                "native settings auto_start control toggles platform autostart host",
                "macOS LaunchAgent or SMAppService verification",
                "Linux XDG autostart desktop entry verification",
            ][..],
        ),
        (
            "clip_row_presentation_plan",
            ready_pending_smoke,
            &[
                "target row visual parity smoke for text, image, file, folder, phrase, and pinned rows",
                "host source proves row labels, kind icons, pin badges, and accessibility text consume NativeHostClipRowPresentation",
            ][..],
        ),
        (
            "search_text_route",
            ready_pending_smoke,
            &[
                "target keyboard and focus smoke",
                "native search shortcut, Escape clear/hide, and list focus restore smoke",
            ][..],
        ),
        (
            "right_click_edit_save",
            ready_pending_smoke,
            &["target multiline native edit window, save refresh, and unsaved-change smoke"][..],
        ),
        (
            "right_click_copy",
            ready_pending_smoke,
            &["target row-menu copy smoke with text, image, and file payloads"][..],
        ),
        (
            "right_click_paste",
            ready_pending_smoke,
            &[
                "target row-menu paste smoke",
                "real OS paste shortcut delivery",
            ][..],
        ),
        (
            "right_click_delete",
            ready_pending_smoke,
            &["target row-menu delete smoke and list refresh proof"][..],
        ),
        (
            "right_click_pin",
            ready_pending_smoke,
            &["target row-menu pin toggle smoke and sorted refresh proof"][..],
        ),
        (
            "right_click_group_assign_remove",
            ready_pending_smoke,
            &["target row-menu group assign/remove smoke"][..],
        ),
        (
            "group_create_rename_delete_reorder_filter",
            ready_pending_smoke,
            &["target popup refresh smoke after every group mutation"][..],
        ),
        (
            "vv_popup_select",
            ready_pending_smoke,
            &[
                "target global input permission smoke",
                "external key consumption proof",
            ][..],
        ),
        (
            "vv_paste",
            ready_pending_smoke,
            &[
                "real OS paste shortcut delivery",
                "focus restoration proof",
                "target-window identity",
            ][..],
        ),
        (
            "clipboard_text_payload",
            ready_pending_smoke,
            &["target text capture, copy, paste, and monitor smoke"][..],
        ),
        (
            "clipboard_image_payload",
            ready_pending_smoke,
            &[
                "target image capture, copy, paste, and monitor smoke",
                "alpha and format normalization proof",
            ][..],
        ),
        (
            "clipboard_file_path_payload",
            ready_pending_smoke,
            &[
                "Linux GDK or portal file URL support",
                "target file path copy and paste smoke",
                "long-running source identity",
                "monitor filtering",
            ][..],
        ),
        (
            "sync_webdav",
            ready_pending_smoke,
            &[
                "target settings sync toggle smoke",
                "WebDAV credential and transfer smoke",
            ][..],
        ),
        (
            "sync_lan",
            ready_pending_smoke,
            &[
                "target settings sync toggle smoke",
                "LAN sync loopback smoke",
            ][..],
        ),
        (
            "status_menu",
            ready_pending_smoke,
            &["target StatusNotifierHost smoke"][..],
        ),
        (
            "dialog_input_confirm_edit",
            ready_pending_smoke,
            &["target input, confirm, info, and edit dialog smoke"][..],
        ),
        (
            "shell_open",
            ready_pending_smoke,
            &["safe target smoke for trusted URL and file handoff"][..],
        ),
        (
            "file_picker",
            ready_pending_smoke,
            &["target file picker smoke"][..],
        ),
        (
            "settings_pages",
            ready_pending_smoke,
            &[
                "native tab and scroll containers for settings pages",
                "native controls for every shared settings section",
                "target screenshots for every settings page",
            ][..],
        ),
        (
            "window_paste_target_identity",
            ready_pending_smoke,
            &[
                "macOS Accessibility identity",
                "Linux AT-SPI or Wayland/X11 identity",
            ][..],
        ),
    ];

    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .flat_map(|platform| {
            rows.iter()
                .map(move |(feature_name, support_status, missing)| {
                    let support_status =
                        zsui_native_feature_support_status_for_platform(
                            platform,
                            feature_name,
                            *support_status,
                        );
                    let mut missing_system_requirements = missing.to_vec();
                    if *feature_name == "sync_lan" {
                        match support_status {
                            ZsuiNativeFeatureSupportStatus::PlannedNotImplemented => {
                                missing_system_requirements.push(SETTINGS_LAN_SYNC_RUNTIME_GAP);
                            }
                            ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke => {
                                missing_system_requirements.push(
                                    "LAN sync code-level progress is partial; inspect lan_sync_capability_matrix for remaining capability blockers",
                                );
                                missing_system_requirements.push(SETTINGS_LAN_SYNC_RUNTIME_GAP);
                            }
                            _ => {}
                        }
                    }
                    if platform == NativeUiPlatform::Linux
                        && *feature_name == "window_system_integration"
                    {
                        missing_system_requirements.push(
                            "target GTK X11 command backend smoke for keep-above and cursor-follow; Wayland layer-shell follow-up if compositor blocks window moves",
                        );
                    }
                    missing_system_requirements.push(match platform {
                        NativeUiPlatform::Windows => {
                            "Windows local release build and native smoke verification"
                        }
                        NativeUiPlatform::Macos => "real macOS AppKit build/run smoke verification",
                        NativeUiPlatform::Linux => "real Ubuntu GTK build/run smoke verification",
                    });
                    ZsuiNativeFeatureParityStatus {
                        feature_name,
                        platform,
                        platform_name: platform.platform_name(),
                        support_status,
                        support_status_name: support_status.status_name(),
                        code_level_ready: support_status.code_level_ready(),
                        target_smoke_required: support_status.target_smoke_required(),
                        target_smoke_verified: support_status.target_smoke_verified(),
                        system_complete: support_status.system_complete(),
                        missing_system_requirements,
                    }
                })
        })
        .collect()
}

fn zsui_native_feature_support_status_for_platform(
    platform: NativeUiPlatform,
    feature_name: &str,
    default_status: ZsuiNativeFeatureSupportStatus,
) -> ZsuiNativeFeatureSupportStatus {
    match (platform, feature_name) {
        (NativeUiPlatform::Macos | NativeUiPlatform::Linux, "sync_lan") => {
            let statuses = zsui_lan_sync_capabilities_for_platform(platform);
            if statuses.iter().all(|status| status.code_level_ready) {
                default_status
            } else if zsui_lan_sync_capability_partial_code_ready(platform) {
                ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke
            } else {
                ZsuiNativeFeatureSupportStatus::PlannedNotImplemented
            }
        }
        (NativeUiPlatform::Linux, "window_system_integration") => {
            let capabilities = zsui_window_system_capabilities_for_platform(platform);
            if capabilities.iter().all(|status| status.code_level_ready) {
                default_status
            } else if capabilities
                .iter()
                .any(|status| status.partial_code_level_ready)
            {
                ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke
            } else {
                ZsuiNativeFeatureSupportStatus::PlannedNotImplemented
            }
        }
        _ => default_status,
    }
}

pub(crate) fn zsui_native_feature_status_for(
    platform: NativeUiPlatform,
    feature_name: &str,
) -> Option<ZsuiNativeFeatureParityStatus> {
    zsui_native_feature_parity_statuses()
        .into_iter()
        .find(|status| status.platform == platform && status.feature_name == feature_name)
}

pub(crate) fn zsui_ui_ingress_classifications() -> Vec<ZsuiUiIngressClassification> {
    native_ui_protocol_surfaces()
        .into_iter()
        .map(|surface| {
            let (
                category_name,
                preferred_app_core_edit_modules,
                platform_host_scope,
                platform_adapter_touchpoints,
            ) = match surface.kind {
                NativeUiProtocolSurfaceKind::MainWindow => (
                    "main_window",
                    vec![
                        "src/app_core/native_host_actions.rs",
                        "src/app_core/native_component_protocol.rs",
                        "src/app_core/main_window_protocol.rs",
                    ],
                    "translate shared main-window actions, search specs, and layout plans into native window controls",
                    vec![
                        "src/windows_win32_adapter.rs",
                        "src/macos_appkit_adapter.rs",
                        "src/linux_gtk_adapter.rs",
                    ],
                ),
                NativeUiProtocolSurfaceKind::Menu => (
                    "menu",
                    vec![
                        "src/app_core/native_host_actions.rs",
                        "src/app_core/native_component_protocol.rs",
                        "src/app_core/main_window_protocol.rs",
                    ],
                    "translate shared row, group, and status menu specs into native menu items",
                    vec![
                        "src/windows_win32_adapter.rs",
                        "src/macos_appkit_adapter.rs",
                        "src/linux_gtk_adapter.rs",
                    ],
                ),
                NativeUiProtocolSurfaceKind::SettingsPage => (
                    "settings_page",
                    vec![
                        "src/app_core/native_host_actions.rs",
                        "src/app_core/native_component_protocol.rs",
                        "src/app_core/settings_protocol.rs",
                    ],
                    "translate shared settings specs and settings model projections into native settings controls",
                    vec![
                        "src/settings_ui_host.rs",
                        "src/macos_native_host.rs",
                        "src/linux_native_host.rs",
                    ],
                ),
                NativeUiProtocolSurfaceKind::Dialog => (
                    "dialog",
                    vec![
                        "src/app_core/native_host_actions.rs",
                        "src/app_core/native_component_protocol.rs",
                        "src/app_core/host_protocol.rs",
                    ],
                    "translate shared dialog actions and edit specs into native input, confirm, and edit dialogs",
                    vec![
                        "src/windows_edit_text_dialog.rs",
                        "src/windows_text_input_dialog.rs",
                        "src/macos_native_host.rs",
                        "src/linux_native_host.rs",
                    ],
                ),
                NativeUiProtocolSurfaceKind::DynamicControls => (
                    "dynamic_controls",
                    vec![
                        "src/app_core/native_host_actions.rs",
                        "src/app_core/native_component_protocol.rs",
                        "src/app_core/render_protocol.rs",
                    ],
                    "translate shared per-row and VV instance specs without inventing platform-local control identity",
                    vec![
                        "src/app/main_renderer.rs",
                        "src/macos_native_host.rs",
                        "src/linux_native_host.rs",
                    ],
                ),
            };

            ZsuiUiIngressClassification {
                ingress_name: surface.surface_name(),
                surface_name: surface.surface_name(),
                category_name,
                preferred_app_core_edit_modules,
                protocol_builder_names: surface.protocol_builder_names.to_vec(),
                dynamic_protocol_builder_names: surface.dynamic_protocol_builder_names.to_vec(),
                action_family_names: surface.action_family_names.to_vec(),
                platform_host_scope,
                platform_adapter_touchpoints,
            }
        })
        .collect()
}

pub(crate) fn zsui_ui_ingress_for_protocol_builder(
    builder_name: &str,
) -> Option<ZsuiUiIngressClassification> {
    zsui_ui_ingress_classifications().into_iter().find(|entry| {
        entry.protocol_builder_names.contains(&builder_name)
            || entry.dynamic_protocol_builder_names.contains(&builder_name)
    })
}

pub(crate) fn zsui_ui_ingress_for_action_family(
    action_family_name: &str,
) -> Option<ZsuiUiIngressClassification> {
    zsui_ui_ingress_classifications()
        .into_iter()
        .find(|entry| entry.action_family_names.contains(&action_family_name))
}

pub(crate) fn zsui_ui_extension_recipes() -> Vec<ZsuiUiExtensionRecipe> {
    vec![
        ZsuiUiExtensionRecipe {
            recipe_name: "add_main_window_command",
            surface_name: "main_window",
            action_family_name: "HostUi",
            primary_app_core_modules: vec![
                "src/app_core/native_host_actions.rs",
                "src/app_core/native_component_protocol.rs",
                "src/app_core/main_window_protocol.rs",
            ],
            action_enum_names: vec!["NativeHostUiAction", "NativeHostSearchControlAction"],
            spec_builder_names: vec![
                "native_host_main_action_button_specs",
                "native_host_search_input_specs",
            ],
            dynamic_spec_builder_names: Vec::new(),
            platform_host_touchpoints: vec![
                "src/app/main_events.rs",
                "src/app/main_search_host.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
            expected_edit_order: vec![
                "define or extend the action enum in app_core",
                "add the typed spec builder entry in app_core",
                "route the command through product/app logic",
                "translate the spec in each native host",
            ],
            required_test_focus: vec![
                "typed spec exposes the action",
                "Windows/macOS/Linux host source consumes the typed builder",
                "feature matrix maps the command to a user feature when needed",
            ],
            must_not_edit_first: vec![
                "src/app/main_events.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
        },
        ZsuiUiExtensionRecipe {
            recipe_name: "add_row_or_status_menu_command",
            surface_name: "menu",
            action_family_name: "Row",
            primary_app_core_modules: vec![
                "src/app_core/native_host_actions.rs",
                "src/app_core/native_component_protocol.rs",
                "src/app_core/main_window.rs",
            ],
            action_enum_names: vec!["NativeHostRowAction", "NativeHostStatusMenuAction"],
            spec_builder_names: vec![
                "native_host_row_action_button_specs",
                "native_host_status_menu_item_specs",
                "native_host_full_row_popup_menu_entries_for_groups",
                "native_host_group_filter_popup_menu_entries_for_groups",
            ],
            dynamic_spec_builder_names: Vec::new(),
            platform_host_touchpoints: vec![
                "src/tray.rs",
                "src/platform/menu.rs",
                "src/app/main_popup_menus.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
            expected_edit_order: vec![
                "define the row/status action in app_core",
                "add the typed menu spec or popup plan entry",
                "route command ids through app_core menu id helpers",
                "translate existing NativePopupMenuEntry/NativeMenuItemSpec in hosts",
            ],
            required_test_focus: vec![
                "menu ids remain stable and non-overlapping",
                "popup entries round-trip to row/status actions",
                "hosts consume shared builders instead of erased component specs",
            ],
            must_not_edit_first: vec![
                "src/platform/menu.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
        },
        ZsuiUiExtensionRecipe {
            recipe_name: "add_settings_control",
            surface_name: "settings_page",
            action_family_name: "SettingsControl",
            primary_app_core_modules: vec![
                "src/app_core/native_host_actions.rs",
                "src/app_core/native_component_protocol.rs",
                "src/app_core/settings_protocol.rs",
                "src/settings_model.rs",
            ],
            action_enum_names: vec![
                "NativeHostSettingsAction",
                "NativeHostSettingsControlAction",
                "NativeHostSettingsGroupAction",
                "NativeHostSettingsPlatformAction",
            ],
            spec_builder_names: vec![
                "native_host_settings_action_button_specs",
                "native_host_settings_control_button_specs",
                "native_host_settings_toggle_specs",
                "native_host_settings_dropdown_specs",
                "native_host_settings_group_button_specs",
                "native_host_settings_platform_button_specs",
            ],
            dynamic_spec_builder_names: Vec::new(),
            platform_host_touchpoints: vec![
                "src/settings_ui_host.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
            expected_edit_order: vec![
                "add the settings model/control summary first",
                "add the settings action/control enum if interaction is needed",
                "add the typed app_core spec builder entry",
                "bind native controls to existing settings submission/apply flow",
            ],
            required_test_focus: vec![
                "settings model exposes the control summary",
                "settings collect/apply handles the new key",
                "three native hosts consume the typed settings builders",
            ],
            must_not_edit_first: vec![
                "src/settings_ui_host.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
        },
        ZsuiUiExtensionRecipe {
            recipe_name: "add_dialog_or_edit_surface",
            surface_name: "dialog",
            action_family_name: "Dialog",
            primary_app_core_modules: vec![
                "src/app_core/native_host_actions.rs",
                "src/app_core/native_component_protocol.rs",
                "src/app_core/host_protocol.rs",
            ],
            action_enum_names: vec!["NativeHostDialogAction", "NativeHostEditTextAction"],
            spec_builder_names: vec![
                "native_host_dialog_button_specs",
                "native_host_edit_text_button_specs",
            ],
            dynamic_spec_builder_names: Vec::new(),
            platform_host_touchpoints: vec![
                "src/platform/dialog.rs",
                "src/windows_edit_text_dialog.rs",
                "src/windows_text_input_dialog.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
            expected_edit_order: vec![
                "define request/result data in app_core host_protocol",
                "add action/spec builders in app_core",
                "route save/confirm semantics through product logic",
                "translate to modal/sheet/dialog in native hosts",
            ],
            required_test_focus: vec![
                "request/result data is platform-neutral",
                "edit close/save semantics are shared",
                "target smoke verifies native modality and focus",
            ],
            must_not_edit_first: vec![
                "src/windows_edit_text_dialog.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
        },
        ZsuiUiExtensionRecipe {
            recipe_name: "add_dynamic_row_or_vv_control",
            surface_name: "dynamic_controls",
            action_family_name: "ClipRow",
            primary_app_core_modules: vec![
                "src/app_core/native_host_actions.rs",
                "src/app_core/native_component_protocol.rs",
                "src/app_core/render_protocol.rs",
            ],
            action_enum_names: vec!["NativeHostClipRowAction", "NativeHostVvSelectAction"],
            spec_builder_names: Vec::new(),
            dynamic_spec_builder_names: vec![
                "native_host_clip_row_specs",
                "native_host_vv_select_specs",
            ],
            platform_host_touchpoints: vec![
                "src/app/main_renderer.rs",
                "src/app/main_row_tools.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
            expected_edit_order: vec![
                "extend the platform-neutral projection/render plan",
                "add or adjust the dynamic component spec builder",
                "keep item identity in app_core action structs",
                "translate rows/VV controls in native hosts without local identity rules",
            ],
            required_test_focus: vec![
                "projection exposes the required row/VV data",
                "dynamic specs preserve item ids and indexes",
                "hosts refresh native rows from app_core projection",
            ],
            must_not_edit_first: vec![
                "src/app/main_renderer.rs",
                "src/macos_native_host.rs",
                "src/linux_native_host.rs",
            ],
        },
    ]
}

pub(crate) fn zsui_ui_extension_recipe_for_action_family(
    action_family_name: &str,
) -> Option<ZsuiUiExtensionRecipe> {
    zsui_ui_extension_recipes()
        .into_iter()
        .find(|recipe| recipe.action_family_name == action_family_name)
}

pub(crate) fn zsui_ui_extension_recipes_for_surface(
    surface_name: &str,
) -> Vec<ZsuiUiExtensionRecipe> {
    zsui_ui_extension_recipes()
        .into_iter()
        .filter(|recipe| recipe.surface_name == surface_name)
        .collect()
}

pub(crate) fn zsui_ui_protocol_convergence() -> Vec<ZsuiUiProtocolConvergenceStatus> {
    let classifications = zsui_ui_ingress_classifications();
    let host_statuses = zsui_native_ui_protocol_host_statuses();
    classifications
        .into_iter()
        .map(|classification| {
            let platform_statuses = host_statuses
                .iter()
                .filter(|status| status.surface_name == classification.surface_name)
                .collect::<Vec<_>>();
            let next_gap = platform_statuses
                .iter()
                .find(|status| !status.source_coverage_verified || !status.system_complete);
            let next_missing_protocol_builder_name =
                next_gap.and_then(|status| status.missing_protocol_builder_names.first().copied());
            let next_missing_requirement = if next_missing_protocol_builder_name.is_some() {
                Some("host source must consume every app_core protocol builder for this UI ingress")
            } else if next_gap.is_some() {
                Some("target native smoke verification")
            } else {
                None
            };
            let static_builder_count = classification.protocol_builder_names.len();
            let dynamic_builder_count = classification.dynamic_protocol_builder_names.len();
            let total_builder_count = static_builder_count + dynamic_builder_count;

            ZsuiUiProtocolConvergenceStatus {
                ingress_name: classification.ingress_name,
                surface_name: classification.surface_name,
                category_name: classification.category_name,
                protocolized: total_builder_count > 0
                    && !classification.action_family_names.is_empty(),
                total_builder_count,
                static_builder_count,
                dynamic_builder_count,
                action_family_count: classification.action_family_names.len(),
                platform_host_count: platform_statuses.len(),
                source_covered_platform_count: platform_statuses
                    .iter()
                    .filter(|status| status.source_coverage_verified)
                    .count(),
                source_gap_platform_count: platform_statuses
                    .iter()
                    .filter(|status| !status.source_coverage_verified)
                    .count(),
                target_smoke_required_platform_count: platform_statuses
                    .iter()
                    .filter(|status| status.target_smoke_required)
                    .count(),
                system_complete_platform_count: platform_statuses
                    .iter()
                    .filter(|status| status.system_complete)
                    .count(),
                preferred_app_core_edit_modules: classification.preferred_app_core_edit_modules,
                platform_adapter_touchpoints: classification.platform_adapter_touchpoints,
                next_platform_name: next_gap.map(|status| status.platform_name),
                next_missing_protocol_builder_name,
                next_missing_requirement,
            }
        })
        .collect()
}

pub(crate) fn zsui_ui_protocol_convergence_for_ingress(
    ingress_name: &str,
) -> Option<ZsuiUiProtocolConvergenceStatus> {
    zsui_ui_protocol_convergence()
        .into_iter()
        .find(|status| status.ingress_name == ingress_name)
}

pub(crate) fn zsui_native_feature_ui_ingress_requirements(
) -> Vec<ZsuiNativeFeatureUiIngressRequirement> {
    [
        (
            "main_window_db_rows",
            &["main_window", "dynamic_controls"][..],
        ),
        ("search_text_route", &["main_window"][..]),
        ("right_click_edit_save", &["menu", "dialog"][..]),
        ("right_click_copy", &["menu"][..]),
        ("right_click_paste", &["menu"][..]),
        ("right_click_delete", &["menu", "dynamic_controls"][..]),
        ("right_click_pin", &["menu", "dynamic_controls"][..]),
        ("right_click_group_assign_remove", &["menu"][..]),
        (
            "group_create_rename_delete_reorder_filter",
            &["menu", "settings_page"][..],
        ),
        ("vv_popup_select", &["main_window", "dynamic_controls"][..]),
        ("vv_paste", &["main_window", "dynamic_controls"][..]),
        (
            "clipboard_text_payload",
            &["main_window", "dynamic_controls"][..],
        ),
        (
            "clipboard_image_payload",
            &["main_window", "dynamic_controls"][..],
        ),
        (
            "clipboard_file_path_payload",
            &["main_window", "dynamic_controls"][..],
        ),
        (
            "clip_row_presentation_plan",
            &["main_window", "dynamic_controls"][..],
        ),
        ("window_system_integration", &["main_window"][..]),
        ("startup_autostart", &["settings_page"][..]),
        ("sync_webdav", &["settings_page"][..]),
        ("sync_lan", &["settings_page"][..]),
        ("status_menu", &["menu"][..]),
        ("dialog_input_confirm_edit", &["dialog"][..]),
        ("shell_open", &["settings_page", "dialog"][..]),
        ("file_picker", &["settings_page", "dialog"][..]),
        ("settings_pages", &["settings_page"][..]),
        ("window_paste_target_identity", &["main_window"][..]),
    ]
    .into_iter()
    .map(|(feature_name, ingress_names)| {
        zsui_feature_ui_ingress_requirement(feature_name, ingress_names)
    })
    .collect()
}

fn zsui_feature_ui_ingress_requirement(
    feature_name: &'static str,
    ingress_names: &[&'static str],
) -> ZsuiNativeFeatureUiIngressRequirement {
    let classifications = zsui_ui_ingress_classifications();
    let component_families = native_component_family_descriptors();
    let mut preferred_app_core_edit_modules = Vec::new();
    let mut protocol_builder_names = Vec::new();
    let mut dynamic_protocol_builder_names = Vec::new();
    let mut action_family_names = Vec::new();
    let mut native_component_family_names = Vec::new();
    let mut typed_component_spec_names = Vec::new();

    for ingress_name in ingress_names {
        if let Some(entry) = classifications
            .iter()
            .find(|entry| entry.ingress_name == *ingress_name)
        {
            push_unique_names(
                &mut preferred_app_core_edit_modules,
                &entry.preferred_app_core_edit_modules,
            );
            push_unique_names(&mut protocol_builder_names, &entry.protocol_builder_names);
            push_unique_names(
                &mut dynamic_protocol_builder_names,
                &entry.dynamic_protocol_builder_names,
            );
            push_unique_names(&mut action_family_names, &entry.action_family_names);
        }
    }

    for family_name in zsui_native_feature_component_family_names(feature_name) {
        if let Some(family) = component_families
            .iter()
            .find(|family| family.family_name == family_name)
        {
            push_unique_names(&mut native_component_family_names, &[family.family_name]);
            push_unique_names(&mut typed_component_spec_names, &[family.typed_spec_name]);
        }
    }

    ZsuiNativeFeatureUiIngressRequirement {
        feature_name,
        ingress_names: ingress_names.to_vec(),
        native_component_family_names,
        typed_component_spec_names,
        preferred_app_core_edit_modules,
        protocol_builder_names,
        dynamic_protocol_builder_names,
        action_family_names,
    }
}

pub(crate) fn zsui_native_feature_component_family_names(feature_name: &str) -> Vec<&'static str> {
    let names = match feature_name {
        "main_window_db_rows" => vec!["main_action_button", "clip_row_instance"],
        "search_text_route" => vec!["search_input"],
        "right_click_edit_save" => vec!["row_action_button", "edit_text_button"],
        "right_click_copy"
        | "right_click_paste"
        | "right_click_delete"
        | "right_click_pin"
        | "right_click_group_assign_remove" => vec!["row_action_button"],
        "group_create_rename_delete_reorder_filter" => {
            vec![
                "main_tool_button",
                "row_action_button",
                "settings_group_button",
            ]
        }
        "vv_popup_select" | "vv_paste" => vec!["main_tool_button"],
        "clipboard_text_payload"
        | "clipboard_image_payload"
        | "clipboard_file_path_payload"
        | "clip_row_presentation_plan" => vec!["clip_row_instance"],
        "window_system_integration" | "window_paste_target_identity" => {
            vec!["main_action_button"]
        }
        "startup_autostart" | "sync_webdav" => vec!["settings_toggle"],
        "sync_lan" => vec!["settings_toggle", "settings_dropdown"],
        "status_menu" => vec!["status_menu_item"],
        "dialog_input_confirm_edit" => vec!["dialog_button", "edit_text_button"],
        "shell_open" | "file_picker" => vec!["settings_platform_button", "dialog_button"],
        "settings_pages" => vec![
            "settings_action_button",
            "settings_control_button",
            "settings_toggle",
            "settings_dropdown",
            "settings_group_button",
            "settings_platform_button",
        ],
        _ => Vec::new(),
    };

    #[cfg(feature = "vv-paste")]
    {
        let mut names = names;
        if matches!(feature_name, "vv_popup_select" | "vv_paste") {
            names.push("vv_select_instance");
        }
        names
    }

    #[cfg(not(feature = "vv-paste"))]
    names
}

fn push_unique_names(target: &mut Vec<&'static str>, source: &[&'static str]) {
    for name in source {
        if !target.contains(name) {
            target.push(*name);
        }
    }
}

pub(crate) fn zsui_clipboard_mvp_feature_matrix() -> Vec<ZsuiClipboardMvpFeatureStatus> {
    let parity = zsui_native_feature_parity_statuses();
    let rows = [
        (
            "main_window_show_hide_and_restore",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &["main_window_db_rows", "status_menu"][..],
        ),
        (
            "clipboard_history_list_and_search",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &[
                "main_window_db_rows",
                "clip_row_presentation_plan",
                "search_text_route",
            ][..],
        ),
        (
            "row_context_copy_paste_delete_pin",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &[
                "right_click_copy",
                "right_click_paste",
                "right_click_delete",
                "right_click_pin",
            ][..],
        ),
        (
            "row_edit_save",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &["right_click_edit_save", "dialog_input_confirm_edit"][..],
        ),
        (
            "group_filter_and_assignment",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &[
                "right_click_group_assign_remove",
                "group_create_rename_delete_reorder_filter",
            ][..],
        ),
        (
            "vv_popup_select_and_paste",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &["vv_popup_select", "vv_paste"][..],
        ),
        (
            "settings_open_edit_save",
            ZsuiClipboardMvpPhase::P0SameClipboardLoop,
            &["settings_pages", "startup_autostart"][..],
        ),
        (
            "text_image_file_payloads",
            ZsuiClipboardMvpPhase::P1SystemIntegration,
            &[
                "clipboard_text_payload",
                "clipboard_image_payload",
                "clipboard_file_path_payload",
                "clip_row_presentation_plan",
            ][..],
        ),
        (
            "sync_webdav",
            ZsuiClipboardMvpPhase::P1SystemIntegration,
            &["sync_webdav"][..],
        ),
        (
            "sync_lan",
            ZsuiClipboardMvpPhase::P1SystemIntegration,
            &["sync_lan"][..],
        ),
        (
            "window_identity_and_paste_delivery",
            ZsuiClipboardMvpPhase::P1SystemIntegration,
            &["window_paste_target_identity", "vv_paste"][..],
        ),
        (
            "shell_file_picker_and_dialogs",
            ZsuiClipboardMvpPhase::P2AdvancedInteraction,
            &["shell_open", "file_picker", "dialog_input_confirm_edit"][..],
        ),
    ];

    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .flat_map(|platform| {
            let parity = &parity;
            rows.iter()
                .map(move |(feature_name, phase, required_features)| {
                    let matching_statuses = required_features
                        .iter()
                        .filter_map(|required_feature| {
                            parity.iter().find(|status| {
                                status.platform == platform
                                    && status.feature_name == *required_feature
                            })
                        })
                        .collect::<Vec<_>>();
                    let all_required_features_present =
                        matching_statuses.len() == required_features.len();
                    let code_level_ready = all_required_features_present
                        && matching_statuses
                            .iter()
                            .all(|status| status.code_level_ready);
                    let target_smoke_required = matching_statuses
                        .iter()
                        .any(|status| status.target_smoke_required);
                    let target_smoke_verified = all_required_features_present
                        && matching_statuses
                            .iter()
                            .all(|status| status.target_smoke_verified);
                    let system_complete = all_required_features_present
                        && matching_statuses
                            .iter()
                            .all(|status| status.system_complete);
                    let missing_system_requirements = matching_statuses
                        .iter()
                        .flat_map(|status| status.missing_system_requirements.iter().copied())
                        .collect::<Vec<_>>();
                    let next_missing_requirement = missing_system_requirements.first().copied();
                    ZsuiClipboardMvpFeatureStatus {
                        feature_name: *feature_name,
                        phase: *phase,
                        phase_name: phase.phase_name(),
                        platform,
                        platform_name: platform.platform_name(),
                        required_native_feature_names: required_features.to_vec(),
                        code_level_ready,
                        target_smoke_required,
                        target_smoke_verified,
                        system_complete,
                        blocks_cross_platform_mvp: phase.blocks_cross_platform_mvp()
                            && !system_complete,
                        next_missing_requirement,
                        missing_system_requirements,
                    }
                })
        })
        .collect()
}

pub(crate) fn zsui_lan_sync_capability_matrix() -> Vec<ZsuiLanSyncCapabilityStatus> {
    let rows = [
        (
            "mobile_link_projection",
            "移动端配对/设置链接投影",
            "mobile_link_projection",
            &[
                "settings_lan_mobile_link_projection_from_json",
                "copy_lan_pair_url",
                "copy_lan_setup_url",
                "open_lan_setup_page",
            ][..],
            &[][..],
            false,
        ),
        (
            "device_book_projection",
            "可信设备簿读取与展示投影",
            "service_discovery",
            &[
                "settings_lan_device_book_projection",
                "load_lan_devices_from_store",
                "lan_devices.json",
            ][..],
            &[][..],
            false,
        ),
        (
            "manual_pair_request",
            "手动主机配对请求",
            "pairing_service",
            &[
                "settings_lan_pair_request_projection_from_json",
                "/v1/pair/request",
            ][..],
            &[][..],
            false,
        ),
        (
            "pair_status_accepted_device_save",
            "配对状态 accepted 解析并写入设备簿",
            "pairing_service",
            &[
                "settings_lan_pair_status_projection",
                "/v1/pair/status",
                "upsert_lan_device_in_store",
            ][..],
            &[][..],
            false,
        ),
        (
            "pair_approval_prompt",
            "待配对请求审批",
            "pair_approval_store",
            &["accept_lan_pairing", "reject_lan_pairing"][..],
            &["platform pending-pair store projection and accept/reject mutation"][..],
            true,
        ),
        (
            "service_discovery_runtime",
            "LAN 服务生命周期与 UDP 发现",
            "service_discovery",
            &[
                "lan_service_lifecycle",
                "lan_udp_discovery",
                "probe_lan_discovery_once",
                "lan_discovered_devices.json",
            ][..],
            &[][..],
            true,
        ),
        (
            "background_clip_sync_loop",
            "后台剪贴板同步循环",
            "pairing_service",
            &[
                "execute_lan_background_clip_sync_once",
                "lan_background_clip_sync_plan",
                "trusted device push/pull loop",
            ][..],
            &[][..],
            true,
        ),
        (
            "image_payload_transfer",
            "图片载荷传输",
            "pairing_service",
            &[
                "lan_clip_envelope_from_native_clip_item",
                "image_png_base64",
                "LAN_IMAGE_MAX_BYTES",
            ][..],
            &["cross-platform LAN image payload transfer smoke"][..],
            true,
        ),
        (
            "file_payload_transfer",
            "文件载荷传输",
            "pairing_service",
            &[
                "execute_lan_file_payload_transfer_once",
                "push_lan_file_payload_to_device",
                "LAN_FILE_CHUNK_BYTES",
                "manual_file",
            ][..],
            &["cross-platform LAN file payload transfer smoke"][..],
            true,
        ),
    ];

    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .flat_map(|platform| {
            rows.iter().map(
                move |(
                    capability_name,
                    display_name,
                    runtime_boundary_name,
                    evidence_names,
                    missing_requirements,
                    release_blocker,
                )| {
                    let code_level_ready =
                        zsui_lan_sync_capability_code_ready(platform, capability_name);
                    let support_status = if code_level_ready {
                        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
                    } else {
                        ZsuiNativeFeatureSupportStatus::PlannedNotImplemented
                    };
                    let mut missing_system_requirements = missing_requirements.to_vec();
                    if code_level_ready {
                        missing_system_requirements.push("target LAN sync capability smoke");
                    } else {
                        missing_system_requirements.push(SETTINGS_LAN_SYNC_RUNTIME_GAP);
                    }
                    missing_system_requirements.push(match platform {
                        NativeUiPlatform::Windows => "Windows LAN sync release smoke",
                        NativeUiPlatform::Macos => "real macOS AppKit LAN sync smoke",
                        NativeUiPlatform::Linux => "real Ubuntu GTK LAN sync smoke",
                    });

                    ZsuiLanSyncCapabilityStatus {
                        capability_name,
                        display_name,
                        platform,
                        platform_name: platform.platform_name(),
                        runtime_boundary_name,
                        support_status,
                        support_status_name: support_status.status_name(),
                        code_level_ready,
                        target_smoke_required: support_status.target_smoke_required(),
                        system_complete: support_status.system_complete(),
                        blocks_sync_lan_release: *release_blocker && !code_level_ready,
                        evidence_names: evidence_names.to_vec(),
                        missing_system_requirements,
                    }
                },
            )
        })
        .collect()
}

fn zsui_lan_sync_capability_code_ready(platform: NativeUiPlatform, capability_name: &str) -> bool {
    match platform {
        NativeUiPlatform::Windows => true,
        NativeUiPlatform::Macos | NativeUiPlatform::Linux => matches!(
            capability_name,
            "mobile_link_projection"
                | "device_book_projection"
                | "manual_pair_request"
                | "pair_status_accepted_device_save"
                | "pair_approval_prompt"
                | "service_discovery_runtime"
                | "background_clip_sync_loop"
                | "image_payload_transfer"
                | "file_payload_transfer"
        ),
    }
}

fn zsui_lan_sync_capability_partial_code_ready(platform: NativeUiPlatform) -> bool {
    let capability_names = [
        "mobile_link_projection",
        "device_book_projection",
        "manual_pair_request",
        "pair_status_accepted_device_save",
        "pair_approval_prompt",
        "service_discovery_runtime",
        "background_clip_sync_loop",
        "image_payload_transfer",
        "file_payload_transfer",
    ];
    let ready_count = capability_names
        .iter()
        .filter(|capability_name| zsui_lan_sync_capability_code_ready(platform, capability_name))
        .count();
    ready_count > 0 && ready_count < capability_names.len()
}

fn zsui_user_feature_host_maturity(
    planned_or_missing: bool,
    partially_code_ready: bool,
    code_level_ready: bool,
    system_complete: bool,
) -> ZsuiUserFeatureHostMaturity {
    if system_complete {
        ZsuiUserFeatureHostMaturity::TargetSmokeVerified
    } else if planned_or_missing || !code_level_ready {
        if partially_code_ready {
            ZsuiUserFeatureHostMaturity::HostPartialPendingWork
        } else {
            ZsuiUserFeatureHostMaturity::ProtocolReadyPendingHost
        }
    } else {
        ZsuiUserFeatureHostMaturity::HostUsablePendingTargetSmoke
    }
}

pub(crate) fn zsui_lan_sync_capabilities_for_platform(
    platform: NativeUiPlatform,
) -> Vec<ZsuiLanSyncCapabilityStatus> {
    zsui_lan_sync_capability_matrix()
        .into_iter()
        .filter(|status| status.platform == platform)
        .collect()
}

pub(crate) fn zsui_window_system_capability_matrix() -> Vec<ZsuiWindowSystemCapabilityStatus> {
    let rows = [
        (
            "window_show_hide_restore",
            "窗口显隐与恢复",
            &[
                "main window toggle action",
                "status menu restore action",
                "native present/hide route",
            ][..],
        ),
        (
            "always_on_top",
            "窗口置顶",
            &[
                "native floating window level or topmost window style",
                "target always-on-top smoke",
            ][..],
        ),
        (
            "dark_theme_detection",
            "暗色主题检测",
            &[
                "native system appearance query",
                "theme-aware native style path",
            ][..],
        ),
        (
            "dpi_scale_factor",
            "DPI 缩放",
            &[
                "native monitor or window scale-factor query",
                "target high-DPI smoke",
            ][..],
        ),
        (
            "cursor_follow_positioning",
            "光标跟随定位",
            &[
                "native pointer location query",
                "native window move near cursor",
            ][..],
        ),
    ];

    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .flat_map(|platform| {
            rows.iter()
                .map(move |(capability_name, display_name, evidence_names)| {
                    let missing_backend_requirements =
                        zsui_window_system_missing_backend_requirements(platform, capability_name);
                    let implemented_backend_names =
                        zsui_window_system_implemented_backend_names(platform, capability_name);
                    let support_status = if missing_backend_requirements.is_empty() {
                        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
                    } else if !implemented_backend_names.is_empty() {
                        ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke
                    } else {
                        ZsuiNativeFeatureSupportStatus::PlannedNotImplemented
                    };
                    ZsuiWindowSystemCapabilityStatus {
                        capability_name,
                        display_name,
                        platform,
                        platform_name: platform.platform_name(),
                        support_status,
                        support_status_name: support_status.status_name(),
                        code_level_ready: support_status.code_level_ready(),
                        partial_code_level_ready: matches!(
                            support_status,
                            ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke
                        ),
                        target_smoke_required: support_status.target_smoke_required(),
                        system_complete: support_status.system_complete(),
                        evidence_names: evidence_names.to_vec(),
                        implemented_backend_names,
                        missing_backend_requirements,
                    }
                })
        })
        .collect()
}

fn zsui_window_system_missing_backend_requirements(
    platform: NativeUiPlatform,
    capability_name: &str,
) -> Vec<&'static str> {
    match (platform, capability_name) {
        (NativeUiPlatform::Linux, "always_on_top")
        | (NativeUiPlatform::Linux, "cursor_follow_positioning") => Vec::new(),
        _ => Vec::new(),
    }
}

fn zsui_window_system_implemented_backend_names(
    platform: NativeUiPlatform,
    capability_name: &str,
) -> Vec<&'static str> {
    match (platform, capability_name) {
        (NativeUiPlatform::Linux, "always_on_top")
        | (NativeUiPlatform::Linux, "cursor_follow_positioning") => vec!["x11_command_tools"],
        _ => Vec::new(),
    }
}

pub(crate) fn zsui_window_system_capabilities_for_platform(
    platform: NativeUiPlatform,
) -> Vec<ZsuiWindowSystemCapabilityStatus> {
    zsui_window_system_capability_matrix()
        .into_iter()
        .filter(|status| status.platform == platform)
        .collect()
}

pub(crate) fn zsui_window_system_backend_work_items() -> Vec<ZsuiWindowSystemBackendWorkItem> {
    zsui_window_system_capability_matrix()
        .into_iter()
        .filter(|status| !status.code_level_ready)
        .map(|status| {
            let platform_host_module_paths = match status.platform {
                NativeUiPlatform::Windows => vec!["src/app", "src/platform"],
                NativeUiPlatform::Macos => vec!["src/macos_native_host.rs", "src/macos_app.rs"],
                NativeUiPlatform::Linux => vec!["src/linux_native_host.rs", "src/linux_app.rs"],
            };
            let (backend_trait_name, default_backend_name, backend_option_names) =
                match status.platform {
                    NativeUiPlatform::Windows => (
                        "Win32WindowSystemBackend",
                        "Win32GdiWindowSystemBackend",
                        vec!["win32_topmost", "win32_monitor_dpi"],
                    ),
                    NativeUiPlatform::Macos => (
                        "AppKitWindowSystemBackend",
                        "AppKitNativeWindowSystemBackend",
                        vec!["NSFloatingWindowLevel", "NSEvent.mouseLocation"],
                    ),
                    NativeUiPlatform::Linux => (
                        "GtkWindowSystemBackend",
                        "Gtk4WindowSystemBackend",
                        vec![
                            "x11_command_tools",
                            "x11_gdk",
                            "gtk_layer_shell",
                            "compositor_specific_adapter",
                        ],
                    ),
                };
            ZsuiWindowSystemBackendWorkItem {
                platform: status.platform,
                platform_name: status.platform_name,
                capability_name: status.capability_name,
                display_name: status.display_name,
                support_status_name: status.support_status_name,
                backend_trait_name,
                default_backend_name,
                platform_host_module_paths,
                preferred_app_core_edit_modules: vec!["src/app_core/framework_manifest.rs"],
                implemented_backend_names: status.implemented_backend_names,
                backend_option_names,
                next_backend_requirement: status.missing_backend_requirements.first().copied(),
                missing_backend_requirements: status.missing_backend_requirements,
            }
        })
        .collect()
}

pub(crate) fn zsui_user_feature_platform_statuses() -> Vec<ZsuiUserFeaturePlatformStatus> {
    let parity = zsui_native_feature_parity_statuses();
    let ingress_requirements = zsui_native_feature_ui_ingress_requirements();
    let rows = [
        (
            "right_click_edit",
            "右键编辑",
            &["right_click_edit_save", "dialog_input_confirm_edit"][..],
        ),
        (
            "right_click_copy",
            "右键复制",
            &[
                "right_click_copy",
                "clipboard_text_payload",
                "clipboard_image_payload",
                "clipboard_file_path_payload",
            ][..],
        ),
        (
            "right_click_paste",
            "右键粘贴",
            &["right_click_paste", "window_paste_target_identity"][..],
        ),
        (
            "right_click_delete",
            "右键删除",
            &["right_click_delete"][..],
        ),
        ("right_click_pin", "右键置顶", &["right_click_pin"][..]),
        (
            "grouping",
            "分组功能",
            &[
                "right_click_group_assign_remove",
                "group_create_rename_delete_reorder_filter",
            ][..],
        ),
        (
            "search",
            "搜索",
            &[
                "main_window_db_rows",
                "clip_row_presentation_plan",
                "search_text_route",
            ][..],
        ),
        (
            "vv_mode",
            "VV 模式",
            &[
                "vv_popup_select",
                "vv_paste",
                "window_paste_target_identity",
            ][..],
        ),
        (
            "settings_pages",
            "设置页",
            &[
                "settings_pages",
                "startup_autostart",
                "dialog_input_confirm_edit",
            ][..],
        ),
        (
            "window_system",
            "窗口系统",
            &["main_window_db_rows", "window_system_integration"][..],
        ),
        ("sync_webdav", "WebDAV 同步", &["sync_webdav"][..]),
        ("sync_lan", "LAN 同步", &["sync_lan"][..]),
        (
            "tray_status_menu",
            "托盘/状态栏",
            &["status_menu", "main_window_db_rows"][..],
        ),
        (
            "popups_dialogs",
            "弹窗",
            &["dialog_input_confirm_edit", "vv_popup_select"][..],
        ),
        (
            "text_payload",
            "文本处理",
            &["clipboard_text_payload", "clip_row_presentation_plan"][..],
        ),
        (
            "image_payload",
            "图片处理",
            &["clipboard_image_payload", "clip_row_presentation_plan"][..],
        ),
        (
            "file_payload",
            "文件处理",
            &["clipboard_file_path_payload", "clip_row_presentation_plan"][..],
        ),
    ];

    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .flat_map(|platform| {
            let parity = &parity;
            let ingress_requirements = &ingress_requirements;
            rows.iter().map(
                move |(user_feature_name, display_name, required_native_feature_names)| {
                    let matching_statuses = required_native_feature_names
                        .iter()
                        .filter_map(|required_feature| {
                            parity.iter().find(|status| {
                                status.platform == platform
                                    && status.feature_name == *required_feature
                            })
                        })
                        .collect::<Vec<_>>();
                    let all_required_features_present =
                        matching_statuses.len() == required_native_feature_names.len();
                    let planned_or_missing = matching_statuses.iter().any(|status| {
                        status.support_status
                            == ZsuiNativeFeatureSupportStatus::PlannedNotImplemented
                    }) || !all_required_features_present;
                    let any_required_feature_code_ready = matching_statuses
                        .iter()
                        .any(|status| status.code_level_ready);
                    let partially_code_ready = matching_statuses.iter().any(|status| {
                        status.support_status
                            == ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke
                    }) || (planned_or_missing
                        && any_required_feature_code_ready);
                    let code_level_ready = all_required_features_present
                        && matching_statuses
                            .iter()
                            .all(|status| status.code_level_ready);
                    let target_smoke_required = !planned_or_missing
                        && matching_statuses
                            .iter()
                            .any(|status| status.target_smoke_required);
                    let target_smoke_verified = !planned_or_missing
                        && matching_statuses
                            .iter()
                            .all(|status| status.target_smoke_verified);
                    let system_complete = !planned_or_missing
                        && matching_statuses
                            .iter()
                            .all(|status| status.system_complete);
                    let support_status_name = if partially_code_ready {
                        ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke
                            .status_name()
                    } else if planned_or_missing {
                        ZsuiNativeFeatureSupportStatus::PlannedNotImplemented.status_name()
                    } else if system_complete {
                        ZsuiNativeFeatureSupportStatus::TargetSmokeVerified.status_name()
                    } else {
                        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
                            .status_name()
                    };
                    let host_maturity = zsui_user_feature_host_maturity(
                        planned_or_missing,
                        partially_code_ready,
                        code_level_ready,
                        system_complete,
                    );
                    let missing_system_requirements = matching_statuses
                        .iter()
                        .flat_map(|status| status.missing_system_requirements.iter().copied())
                        .collect::<Vec<_>>();
                    let mut ui_ingress_names = Vec::new();
                    let mut native_component_family_names = Vec::new();
                    let mut typed_component_spec_names = Vec::new();
                    let mut preferred_app_core_edit_modules = Vec::new();
                    for native_feature_name in *required_native_feature_names {
                        if let Some(requirement) = ingress_requirements
                            .iter()
                            .find(|requirement| requirement.feature_name == *native_feature_name)
                        {
                            push_unique_names(&mut ui_ingress_names, &requirement.ingress_names);
                            push_unique_names(
                                &mut native_component_family_names,
                                &requirement.native_component_family_names,
                            );
                            push_unique_names(
                                &mut typed_component_spec_names,
                                &requirement.typed_component_spec_names,
                            );
                            push_unique_names(
                                &mut preferred_app_core_edit_modules,
                                &requirement.preferred_app_core_edit_modules,
                            );
                        }
                    }

                    ZsuiUserFeaturePlatformStatus {
                        user_feature_name,
                        display_name,
                        platform,
                        platform_name: platform.platform_name(),
                        support_status_name,
                        required_native_feature_names: required_native_feature_names.to_vec(),
                        ui_ingress_names,
                        native_component_family_names,
                        typed_component_spec_names,
                        preferred_app_core_edit_modules,
                        code_level_ready,
                        host_maturity_name: host_maturity.maturity_name(),
                        host_maturity_percent: host_maturity.maturity_percent(),
                        host_usable: host_maturity.host_usable(),
                        target_smoke_required,
                        target_smoke_verified,
                        system_complete,
                        next_missing_requirement: missing_system_requirements.first().copied(),
                        missing_system_requirements,
                    }
                },
            )
        })
        .collect()
}

pub(crate) fn zsui_user_feature_status_for(
    platform: NativeUiPlatform,
    user_feature_name: &str,
) -> Option<ZsuiUserFeaturePlatformStatus> {
    zsui_user_feature_platform_statuses()
        .into_iter()
        .find(|status| status.platform == platform && status.user_feature_name == user_feature_name)
}

pub(crate) fn zsui_user_feature_completion_summaries() -> Vec<ZsuiUserFeatureCompletionSummary> {
    let statuses = zsui_user_feature_platform_statuses();
    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .map(|platform| {
            let platform_rows = statuses
                .iter()
                .filter(|status| status.platform == platform)
                .collect::<Vec<_>>();
            let total_user_feature_count = platform_rows.len();
            let code_level_ready_count = platform_rows
                .iter()
                .filter(|status| status.code_level_ready)
                .count();
            let host_usable_count = platform_rows
                .iter()
                .filter(|status| status.host_usable)
                .count();
            let planned_not_implemented_count = platform_rows
                .iter()
                .filter(|status| {
                    status.support_status_name
                        == ZsuiNativeFeatureSupportStatus::PlannedNotImplemented.status_name()
                })
                .count();
            let target_smoke_required_count = platform_rows
                .iter()
                .filter(|status| status.target_smoke_required)
                .count();
            let system_complete_count = platform_rows
                .iter()
                .filter(|status| status.system_complete)
                .count();
            let next_missing = platform_rows.iter().find(|status| !status.system_complete);

            ZsuiUserFeatureCompletionSummary {
                platform,
                platform_name: platform.platform_name(),
                total_user_feature_count,
                code_level_ready_count,
                host_usable_count,
                planned_not_implemented_count,
                target_smoke_required_count,
                system_complete_count,
                code_level_ready_percent: zsui_percent(
                    code_level_ready_count,
                    total_user_feature_count,
                ),
                host_usable_percent: zsui_percent(host_usable_count, total_user_feature_count),
                system_complete_percent: zsui_percent(
                    system_complete_count,
                    total_user_feature_count,
                ),
                next_user_feature_name: next_missing.map(|status| status.user_feature_name),
                next_missing_requirement: next_missing
                    .and_then(|status| status.next_missing_requirement),
            }
        })
        .collect()
}

pub(crate) fn zsui_user_feature_completion_summary_for(
    platform: NativeUiPlatform,
) -> Option<ZsuiUserFeatureCompletionSummary> {
    zsui_user_feature_completion_summaries()
        .into_iter()
        .find(|summary| summary.platform == platform)
}

pub(crate) fn zsui_user_feature_cross_platform_summaries(
) -> Vec<ZsuiUserFeatureCrossPlatformSummary> {
    let statuses = zsui_user_feature_platform_statuses();
    let mut feature_names = Vec::new();
    for status in &statuses {
        if !feature_names.contains(&status.user_feature_name) {
            feature_names.push(status.user_feature_name);
        }
    }

    feature_names
        .into_iter()
        .filter_map(|user_feature_name| {
            let feature_rows = statuses
                .iter()
                .filter(|status| status.user_feature_name == user_feature_name)
                .collect::<Vec<_>>();
            let first = feature_rows.first()?;
            let total_platform_count = feature_rows.len();
            let code_level_ready_platform_names = feature_rows
                .iter()
                .filter(|status| status.code_level_ready)
                .map(|status| status.platform_name)
                .collect::<Vec<_>>();
            let host_usable_platform_names = feature_rows
                .iter()
                .filter(|status| status.host_usable)
                .map(|status| status.platform_name)
                .collect::<Vec<_>>();
            let planned_platform_names = feature_rows
                .iter()
                .filter(|status| {
                    status.support_status_name
                        == ZsuiNativeFeatureSupportStatus::PlannedNotImplemented.status_name()
                })
                .map(|status| status.platform_name)
                .collect::<Vec<_>>();
            let target_smoke_required_platform_names = feature_rows
                .iter()
                .filter(|status| status.target_smoke_required)
                .map(|status| status.platform_name)
                .collect::<Vec<_>>();
            let system_complete_platform_names = feature_rows
                .iter()
                .filter(|status| status.system_complete)
                .map(|status| status.platform_name)
                .collect::<Vec<_>>();
            let next_missing = feature_rows.iter().find(|status| !status.system_complete);

            Some(ZsuiUserFeatureCrossPlatformSummary {
                user_feature_name,
                display_name: first.display_name,
                total_platform_count,
                code_level_ready_count: code_level_ready_platform_names.len(),
                host_usable_count: host_usable_platform_names.len(),
                planned_not_implemented_count: planned_platform_names.len(),
                target_smoke_required_count: target_smoke_required_platform_names.len(),
                system_complete_count: system_complete_platform_names.len(),
                code_level_ready_percent: zsui_percent(
                    code_level_ready_platform_names.len(),
                    total_platform_count,
                ),
                host_usable_percent: zsui_percent(
                    host_usable_platform_names.len(),
                    total_platform_count,
                ),
                system_complete_percent: zsui_percent(
                    system_complete_platform_names.len(),
                    total_platform_count,
                ),
                code_level_ready_platform_names,
                host_usable_platform_names,
                planned_platform_names,
                target_smoke_required_platform_names,
                system_complete_platform_names,
                next_platform_name: next_missing.map(|status| status.platform_name),
                next_missing_requirement: next_missing
                    .and_then(|status| status.next_missing_requirement),
            })
        })
        .collect()
}

pub(crate) fn zsui_user_feature_cross_platform_summary_for(
    user_feature_name: &str,
) -> Option<ZsuiUserFeatureCrossPlatformSummary> {
    zsui_user_feature_cross_platform_summaries()
        .into_iter()
        .find(|summary| summary.user_feature_name == user_feature_name)
}

pub(crate) fn zsui_user_feature_progress_reports() -> Vec<ZsuiUserFeatureProgressReport> {
    let statuses = zsui_user_feature_platform_statuses();
    zsui_user_feature_cross_platform_summaries()
        .into_iter()
        .map(|summary| {
            let platform_statuses = statuses
                .iter()
                .filter(|status| status.user_feature_name == summary.user_feature_name)
                .cloned()
                .collect::<Vec<_>>();
            ZsuiUserFeatureProgressReport {
                user_feature_name: summary.user_feature_name,
                display_name: summary.display_name,
                cross_platform_summary: summary,
                platform_statuses,
            }
        })
        .collect()
}

pub(crate) fn zsui_user_feature_progress_report_for(
    user_feature_name: &str,
) -> Option<ZsuiUserFeatureProgressReport> {
    zsui_user_feature_progress_reports()
        .into_iter()
        .find(|report| report.user_feature_name == user_feature_name)
}

pub(crate) fn zsui_user_feature_release_progress() -> ZsuiUserFeatureReleaseProgress {
    let statuses = zsui_user_feature_platform_statuses();
    let total_platform_feature_slots = statuses.len();
    let code_level_ready_slots = statuses
        .iter()
        .filter(|status| status.code_level_ready)
        .count();
    let host_usable_slots = statuses.iter().filter(|status| status.host_usable).count();
    let planned_not_implemented_slots = statuses
        .iter()
        .filter(|status| {
            status.support_status_name
                == ZsuiNativeFeatureSupportStatus::PlannedNotImplemented.status_name()
        })
        .count();
    let target_smoke_required_slots = statuses
        .iter()
        .filter(|status| status.target_smoke_required)
        .count();
    let system_complete_slots = statuses
        .iter()
        .filter(|status| status.system_complete)
        .count();
    let non_windows_statuses = statuses
        .iter()
        .filter(|status| status.platform != NativeUiPlatform::Windows)
        .collect::<Vec<_>>();
    let non_windows_host_slots = non_windows_statuses.len();
    let non_windows_host_code_level_ready_slots = non_windows_statuses
        .iter()
        .filter(|status| status.code_level_ready)
        .count();
    let non_windows_host_usable_slots = non_windows_statuses
        .iter()
        .filter(|status| status.host_usable)
        .count();
    let non_windows_host_code_gap_slots =
        non_windows_host_slots.saturating_sub(non_windows_host_code_level_ready_slots);
    let non_windows_host_system_complete_slots = non_windows_statuses
        .iter()
        .filter(|status| status.system_complete)
        .count();
    let work_items = zsui_user_feature_work_items();
    let next_host_work_item = work_items
        .iter()
        .find(|item| item.platform != NativeUiPlatform::Windows);
    let next_work_item = next_host_work_item.or_else(|| work_items.first());
    let next_host_code_gap_status = non_windows_statuses
        .iter()
        .find(|status| !status.code_level_ready);
    let mut next_host_code_gap_module_paths = Vec::new();
    if let Some(status) = next_host_code_gap_status {
        for ingress_name in &status.ui_ingress_names {
            push_unique_names(
                &mut next_host_code_gap_module_paths,
                &zsui_platform_host_modules_for_ingress(status.platform, ingress_name),
            );
        }
    }

    ZsuiUserFeatureReleaseProgress {
        total_platform_feature_slots,
        code_level_ready_slots,
        host_usable_slots,
        planned_not_implemented_slots,
        target_smoke_required_slots,
        system_complete_slots,
        non_windows_host_slots,
        non_windows_host_code_level_ready_slots,
        non_windows_host_usable_slots,
        non_windows_host_code_gap_slots,
        non_windows_host_system_complete_slots,
        code_level_ready_percent: zsui_percent(
            code_level_ready_slots,
            total_platform_feature_slots,
        ),
        host_usable_percent: zsui_percent(host_usable_slots, total_platform_feature_slots),
        non_windows_host_usable_percent: zsui_percent(
            non_windows_host_usable_slots,
            non_windows_host_slots,
        ),
        system_complete_percent: zsui_percent(system_complete_slots, total_platform_feature_slots),
        next_platform_name: next_work_item.as_ref().map(|item| item.platform_name),
        next_user_feature_name: next_work_item.as_ref().map(|item| item.user_feature_name),
        next_display_name: next_work_item.as_ref().map(|item| item.display_name),
        next_ui_ingress_names: next_work_item
            .as_ref()
            .map(|item| item.ui_ingress_names.clone())
            .unwrap_or_default(),
        next_native_component_family_names: next_work_item
            .as_ref()
            .map(|item| item.native_component_family_names.clone())
            .unwrap_or_default(),
        next_typed_component_spec_names: next_work_item
            .as_ref()
            .map(|item| item.typed_component_spec_names.clone())
            .unwrap_or_default(),
        next_preferred_app_core_edit_modules: next_work_item
            .as_ref()
            .map(|item| item.preferred_app_core_edit_modules.clone())
            .unwrap_or_default(),
        next_platform_host_module_paths: next_work_item
            .as_ref()
            .map(|item| item.platform_host_module_paths.clone())
            .unwrap_or_default(),
        next_missing_requirement: next_work_item
            .as_ref()
            .and_then(|item| item.next_missing_requirement),
        next_host_platform_name: next_host_work_item.map(|item| item.platform_name),
        next_host_user_feature_name: next_host_work_item.map(|item| item.user_feature_name),
        next_host_display_name: next_host_work_item.map(|item| item.display_name),
        next_host_ui_ingress_names: next_host_work_item
            .map(|item| item.ui_ingress_names.clone())
            .unwrap_or_default(),
        next_host_native_component_family_names: next_host_work_item
            .map(|item| item.native_component_family_names.clone())
            .unwrap_or_default(),
        next_host_typed_component_spec_names: next_host_work_item
            .map(|item| item.typed_component_spec_names.clone())
            .unwrap_or_default(),
        next_host_preferred_app_core_edit_modules: next_host_work_item
            .map(|item| item.preferred_app_core_edit_modules.clone())
            .unwrap_or_default(),
        next_host_module_paths: next_host_work_item
            .map(|item| item.platform_host_module_paths.clone())
            .unwrap_or_default(),
        next_host_missing_requirement: next_host_work_item
            .and_then(|item| item.next_missing_requirement),
        next_host_code_gap_platform_name: next_host_code_gap_status
            .map(|status| status.platform_name),
        next_host_code_gap_user_feature_name: next_host_code_gap_status
            .map(|status| status.user_feature_name),
        next_host_code_gap_display_name: next_host_code_gap_status
            .map(|status| status.display_name),
        next_host_code_gap_ui_ingress_names: next_host_code_gap_status
            .map(|status| status.ui_ingress_names.clone())
            .unwrap_or_default(),
        next_host_code_gap_native_component_family_names: next_host_code_gap_status
            .map(|status| status.native_component_family_names.clone())
            .unwrap_or_default(),
        next_host_code_gap_typed_component_spec_names: next_host_code_gap_status
            .map(|status| status.typed_component_spec_names.clone())
            .unwrap_or_default(),
        next_host_code_gap_preferred_app_core_edit_modules: next_host_code_gap_status
            .map(|status| status.preferred_app_core_edit_modules.clone())
            .unwrap_or_default(),
        next_host_code_gap_module_paths,
        next_host_code_gap_missing_requirement: next_host_code_gap_status
            .and_then(|status| status.next_missing_requirement),
    }
}

pub(crate) fn zsui_user_feature_work_items() -> Vec<ZsuiUserFeatureWorkItem> {
    zsui_user_feature_platform_statuses()
        .into_iter()
        .filter(|status| !status.system_complete)
        .map(|status| {
            let mut platform_host_module_paths = Vec::new();
            for ingress_name in &status.ui_ingress_names {
                push_unique_names(
                    &mut platform_host_module_paths,
                    &zsui_platform_host_modules_for_ingress(status.platform, ingress_name),
                );
            }

            ZsuiUserFeatureWorkItem {
                platform: status.platform,
                platform_name: status.platform_name,
                user_feature_name: status.user_feature_name,
                display_name: status.display_name,
                support_status_name: status.support_status_name,
                ui_ingress_names: status.ui_ingress_names,
                native_component_family_names: status.native_component_family_names,
                typed_component_spec_names: status.typed_component_spec_names,
                preferred_app_core_edit_modules: status.preferred_app_core_edit_modules,
                platform_host_module_paths,
                required_native_feature_names: status.required_native_feature_names,
                next_missing_requirement: status.next_missing_requirement,
            }
        })
        .collect()
}

fn zsui_platform_host_modules_for_ingress(
    platform: NativeUiPlatform,
    ingress_name: &str,
) -> Vec<&'static str> {
    match (platform, ingress_name) {
        (NativeUiPlatform::Windows, "main_window") => vec!["src/app.rs", "src/app/*"],
        (NativeUiPlatform::Windows, "menu") => vec!["src/app/main_popup_menus.rs"],
        (NativeUiPlatform::Windows, "settings_page") => {
            vec!["src/settings_ui_host.rs", "src/app/settings_*"]
        }
        (NativeUiPlatform::Windows, "dialog") => vec![
            "src/platform/dialog.rs",
            "src/windows_edit_text_dialog.rs",
            "src/windows_text_input_dialog.rs",
        ],
        (NativeUiPlatform::Windows, "dynamic_controls") => {
            vec!["src/app/main_renderer.rs", "src/app/main_row_tools.rs"]
        }
        (NativeUiPlatform::Macos, _) => vec!["src/macos_native_host.rs", "src/macos_app.rs"],
        (NativeUiPlatform::Linux, _) => vec!["src/linux_native_host.rs", "src/linux_app.rs"],
        _ => Vec::new(),
    }
}

pub(crate) fn zsui_native_ui_protocol_host_statuses() -> Vec<ZsuiNativeUiProtocolHostStatus> {
    SUPPORTED_NATIVE_UI_BACKENDS
        .iter()
        .flat_map(|backend| {
            native_ui_protocol_surfaces()
                .into_iter()
                .map(move |surface| {
                    let all_builder_names = surface
                        .protocol_builder_names
                        .iter()
                        .chain(surface.dynamic_protocol_builder_names.iter())
                        .copied()
                        .collect::<Vec<_>>();
                    let covered_builder_names = match backend.platform {
                        NativeUiPlatform::Windows if surface.surface_name() == "main_window" => {
                            vec![
                                "native_host_main_action_button_specs",
                                "native_host_main_tool_button_specs",
                                "native_host_search_input_specs",
                            ]
                        }
                        NativeUiPlatform::Windows if surface.surface_name() == "menu" => {
                            vec![
                                "native_host_row_action_button_specs",
                                "native_host_status_menu_item_specs",
                                "native_host_full_row_popup_menu_entries_for_groups",
                                "native_host_group_filter_popup_menu_entries_for_groups",
                            ]
                        }
                        NativeUiPlatform::Windows if surface.surface_name() == "settings_page" => {
                            vec![
                                "native_host_settings_action_button_specs",
                                "native_host_settings_control_button_specs",
                                "native_host_settings_toggle_specs",
                                "native_host_settings_dropdown_specs",
                                "native_host_settings_group_button_specs",
                                "native_host_settings_platform_button_specs",
                            ]
                        }
                        NativeUiPlatform::Windows if surface.surface_name() == "dialog" => {
                            vec![
                                "native_host_dialog_button_specs",
                                "native_host_edit_text_button_specs",
                            ]
                        }
                        NativeUiPlatform::Windows
                            if surface.surface_name() == "dynamic_controls" =>
                        {
                            #[cfg(feature = "vv-paste")]
                            {
                                vec![
                                    "native_host_clip_row_specs",
                                    "native_host_vv_select_specs",
                                ]
                            }
                            #[cfg(not(feature = "vv-paste"))]
                            {
                                vec!["native_host_clip_row_specs"]
                            }
                        }
                        NativeUiPlatform::Windows => Vec::new(),
                        NativeUiPlatform::Macos | NativeUiPlatform::Linux => {
                            all_builder_names.clone()
                        }
                    };
                    let missing_protocol_builder_names = all_builder_names
                        .into_iter()
                        .filter(|builder_name| !covered_builder_names.contains(builder_name))
                        .collect::<Vec<_>>();
                    let source_coverage_verified = missing_protocol_builder_names.is_empty();
                    ZsuiNativeUiProtocolHostStatus {
                        platform: backend.platform,
                        platform_name: backend.platform_name(),
                        toolkit_name: backend.toolkit_name(),
                        backend_status_name: backend.status_name(),
                        host_module_path: match backend.platform {
                            NativeUiPlatform::Windows => {
                                "src/app.rs + src/app/* + src/tray.rs + src/settings_ui_host.rs + src/platform/dialog.rs + src/windows_edit_text_dialog.rs"
                            }
                            NativeUiPlatform::Macos => "src/macos_native_host.rs",
                            NativeUiPlatform::Linux => "src/linux_native_host.rs",
                        },
                        surface_name: surface.surface_name(),
                        protocol_builder_names: surface.protocol_builder_names.to_vec(),
                        dynamic_protocol_builder_names: surface
                            .dynamic_protocol_builder_names
                            .to_vec(),
                        action_family_names: surface.action_family_names.to_vec(),
                        source_guard_required: true,
                        source_coverage_verified,
                        missing_protocol_builder_names,
                        target_smoke_required: true,
                        target_smoke_verified: false,
                        system_complete: false,
                    }
                })
        })
        .collect()
}

pub(crate) fn zsui_host_private_ui_ingress_audits() -> Vec<ZsuiHostPrivateUiIngressAudit> {
    SUPPORTED_NATIVE_UI_PLATFORMS
        .into_iter()
        .flat_map(|platform| {
            native_ui_protocol_surfaces()
                .into_iter()
                .map(move |surface| zsui_host_private_ui_ingress_audit(platform, surface))
        })
        .collect()
}

pub(crate) fn zsui_host_private_ui_ingress_audits_for_platform(
    platform: NativeUiPlatform,
) -> Vec<ZsuiHostPrivateUiIngressAudit> {
    zsui_host_private_ui_ingress_audits()
        .into_iter()
        .filter(|audit| audit.platform == platform)
        .collect()
}

pub(crate) fn zsui_host_private_ui_ingress_audits_for_surface(
    surface_name: &str,
) -> Vec<ZsuiHostPrivateUiIngressAudit> {
    zsui_host_private_ui_ingress_audits()
        .into_iter()
        .filter(|audit| audit.surface_name == surface_name)
        .collect()
}

pub(crate) fn zsui_host_private_ui_ingress_protocol_work_items(
) -> Vec<ZsuiHostPrivateUiIngressAudit> {
    zsui_host_private_ui_ingress_audits()
        .into_iter()
        .filter(|audit| audit.audit_status.needs_protocol_work())
        .collect()
}

fn zsui_host_private_ui_ingress_audit(
    platform: NativeUiPlatform,
    surface: NativeUiProtocolSurface,
) -> ZsuiHostPrivateUiIngressAudit {
    let (native_entry_markers, private_native_entry_names, audit_status, next_step) =
        zsui_host_private_ui_ingress_audit_payload(platform, surface.kind);
    ZsuiHostPrivateUiIngressAudit {
        platform,
        platform_name: platform.platform_name(),
        surface_name: surface.surface_name(),
        host_module_path: zsui_primary_host_module_for_platform(platform),
        native_entry_markers,
        protocol_anchor_names: surface
            .protocol_builder_names
            .iter()
            .chain(surface.dynamic_protocol_builder_names.iter())
            .copied()
            .collect(),
        private_native_entry_names,
        audit_status,
        audit_status_name: audit_status.status_name(),
        next_protocolization_step: next_step,
    }
}

fn zsui_primary_host_module_for_platform(platform: NativeUiPlatform) -> &'static str {
    match platform {
        NativeUiPlatform::Windows => "src/app/* + src/settings_ui_host.rs",
        NativeUiPlatform::Macos => "src/macos_native_host.rs",
        NativeUiPlatform::Linux => "src/linux_native_host.rs",
    }
}

fn zsui_host_private_ui_ingress_audit_payload(
    platform: NativeUiPlatform,
    surface: NativeUiProtocolSurfaceKind,
) -> (
    Vec<&'static str>,
    Vec<&'static str>,
    ZsuiHostPrivateUiIngressAuditStatus,
    Option<&'static str>,
) {
    use NativeUiPlatform::{Linux, Macos, Windows};
    use NativeUiProtocolSurfaceKind::{Dialog, DynamicControls, MainWindow, Menu, SettingsPage};
    use ZsuiHostPrivateUiIngressAuditStatus::{
        PrivateIngressNeedsProtocol, ProtocolAnchoredHostChromeOnly,
        ProtocolAnchoredNeedsExtraction,
    };

    match (platform, surface) {
        (Windows, MainWindow) => (
            vec!["CreateWindowExW", "draw_main_window", "native_host_main_action_button_specs"],
            vec!["win32_main_window_layout_and_paint"],
            ProtocolAnchoredNeedsExtraction,
            Some("move remaining Win32 main-window layout tokens behind typed app_core render plans"),
        ),
        (Windows, Menu) => (
            vec!["WindowsPopupMenuHost", "NativePopupMenuEntry", "native_host_status_menu_item_specs"],
            Vec::new(),
            ProtocolAnchoredHostChromeOnly,
            None,
        ),
        (Windows, SettingsPage) => (
            vec!["settings_ui_host", "create_settings_button", "native_host_settings_toggle_specs"],
            vec!["win32_settings_page_chrome_and_spacing"],
            ProtocolAnchoredNeedsExtraction,
            Some("turn settings page chrome, grouping, and spacing into app_core settings layout sections"),
        ),
        (Windows, Dialog) => (
            vec!["WindowsDialogHost", "WindowsEditTextDialogHost", "native_host_edit_text_button_specs"],
            Vec::new(),
            ProtocolAnchoredHostChromeOnly,
            None,
        ),
        (Windows, DynamicControls) => (
            vec!["main_renderer", "native_host_clip_row_specs", "native_host_vv_select_specs"],
            vec!["win32_clip_row_paint_metrics", "win32_vv_popup_paint_metrics"],
            ProtocolAnchoredNeedsExtraction,
            Some("move clip row and VV popup paint metrics into typed dynamic render plans"),
        ),
        (Macos, MainWindow) => (
            vec!["NSWindow::initWithContentRect_styleMask_backing_defer", "NSSearchField::new", "NSTableView::initWithFrame"],
            vec!["appkit_main_window_chrome", "appkit_table_container_layout"],
            ProtocolAnchoredNeedsExtraction,
            Some("keep AppKit chrome local but move table/search sizing decisions into app_core main-window layout plans"),
        ),
        (Macos, Menu) => (
            vec!["NSStatusBar::systemStatusBar", "NSMenuItem::initWithTitle_action_keyEquivalent", "NativePopupMenuEntry"],
            vec!["disabled_menu_item_policy"],
            PrivateIngressNeedsProtocol,
            Some("add disabled-state display policy to app_core menu entries"),
        ),
        (Macos, SettingsPage) => (
            vec!["NSWindow::initWithContentRect_styleMask_backing_defer", "NSTabView::initWithFrame", "native_host_settings_action_button_specs"],
            vec!["settings_summary_labels", "settings_group_rows"],
            PrivateIngressNeedsProtocol,
            Some("promote settings summary row layout and group-row presentation into settings_protocol/native_component_protocol"),
        ),
        (Macos, Dialog) => (
            vec!["NSAlert", "NSTextView::initWithFrame", "native_host_edit_text_button_specs"],
            vec!["edit_text_window_layout", "unsaved_changes_alert_copy"],
            ProtocolAnchoredNeedsExtraction,
            Some("move edit dialog title, sizing, and close-confirm copy into app_core edit dialog plan"),
        ),
        (Macos, DynamicControls) => (
            vec!["NSTableView", "native_host_clip_row_specs", "native_host_vv_select_specs"],
            vec!["clip_table_cell_rendering", "vv_popup_window_chrome"],
            ProtocolAnchoredNeedsExtraction,
            Some("move clip cell typography and VV popup chrome metrics into dynamic control render specs"),
        ),
        (Linux, MainWindow) => (
            vec!["ApplicationWindow::builder", "HeaderBar::new", "SearchEntry::new", "ListBox::new"],
            vec!["gtk_headerbar_chrome", "gtk_status_label_copy"],
            ProtocolAnchoredNeedsExtraction,
            Some("keep GTK chrome local but move status/title/search visibility copy into app_core main-window plans"),
        ),
        (Linux, Menu) => (
            vec!["gio::MenuItem::new", "PopoverMenu::from_model", "NativePopupMenuEntry"],
            vec!["disabled_menu_item_policy"],
            PrivateIngressNeedsProtocol,
            Some("add disabled-state display policy to app_core menu entries"),
        ),
        (Linux, SettingsPage) => (
            vec!["ApplicationWindow::builder", "Notebook::new", "native_host_settings_toggle_specs"],
            vec!["settings_group_rows", "settings_button_style_roles"],
            PrivateIngressNeedsProtocol,
            Some("promote settings group rows and action style roles into app_core settings specs"),
        ),
        (Linux, Dialog) => (
            vec!["MessageDialog::builder", "TextView::new", "native_host_edit_text_button_specs"],
            vec!["edit_window_headerbar_policy", "ctrl_enter_save_shortcut"],
            ProtocolAnchoredNeedsExtraction,
            Some("move edit dialog keyboard shortcuts and headerbar policy into app_core edit dialog plan"),
        ),
        (Linux, DynamicControls) => (
            vec!["ListBoxRow::new", "native_host_clip_row_specs", "native_host_vv_select_specs"],
            vec!["clip_row_css_classes", "vv_popup_animation_policy"],
            ProtocolAnchoredNeedsExtraction,
            Some("move list row visual states and VV popup animation policy into dynamic control specs"),
        ),
    }
}

pub(crate) fn zsui_native_ui_host_translation_work_items(
) -> Vec<ZsuiNativeUiHostTranslationWorkItem> {
    let classifications = zsui_ui_ingress_classifications();
    zsui_native_ui_protocol_host_statuses()
        .into_iter()
        .filter(|status| !status.system_complete)
        .map(|status| {
            let classification = classifications
                .iter()
                .find(|classification| classification.surface_name == status.surface_name);
            let preferred_app_core_edit_modules = classification
                .map(|classification| classification.preferred_app_core_edit_modules.clone())
                .unwrap_or_default();
            let platform_host_module_paths =
                zsui_platform_host_modules_for_ingress(status.platform, status.surface_name);
            let next_missing_requirement = if !status.source_coverage_verified {
                Some("host source must consume every app_core protocol builder for this UI surface")
            } else if status.target_smoke_required && !status.target_smoke_verified {
                Some("target native smoke verification")
            } else {
                None
            };

            ZsuiNativeUiHostTranslationWorkItem {
                platform: status.platform,
                platform_name: status.platform_name,
                surface_name: status.surface_name,
                toolkit_name: status.toolkit_name,
                host_module_path: status.host_module_path,
                preferred_app_core_edit_modules,
                platform_host_module_paths,
                protocol_builder_names: status.protocol_builder_names,
                dynamic_protocol_builder_names: status.dynamic_protocol_builder_names,
                action_family_names: status.action_family_names,
                source_coverage_verified: status.source_coverage_verified,
                target_smoke_required: status.target_smoke_required,
                system_complete: status.system_complete,
                next_missing_requirement,
            }
        })
        .collect()
}

pub(crate) fn zsui_native_ui_host_translation_work_items_for_platform(
    platform: NativeUiPlatform,
) -> Vec<ZsuiNativeUiHostTranslationWorkItem> {
    zsui_native_ui_host_translation_work_items()
        .into_iter()
        .filter(|item| item.platform == platform)
        .collect()
}

pub(crate) fn zsui_native_ui_host_translation_work_item_for(
    platform: NativeUiPlatform,
    surface_name: &str,
) -> Option<ZsuiNativeUiHostTranslationWorkItem> {
    zsui_native_ui_host_translation_work_items()
        .into_iter()
        .find(|item| item.platform == platform && item.surface_name == surface_name)
}

pub(crate) fn zsui_next_native_ui_host_translation_work_item_for_platform(
    platform: NativeUiPlatform,
) -> Option<ZsuiNativeUiHostTranslationWorkItem> {
    zsui_native_ui_host_translation_work_items_for_platform(platform)
        .into_iter()
        .find(|item| !item.source_coverage_verified)
        .or_else(|| {
            zsui_native_ui_host_translation_work_items_for_platform(platform)
                .into_iter()
                .find(|item| item.target_smoke_required && !item.system_complete)
        })
}

const fn zsui_native_target_smoke_environment_name(platform: NativeUiPlatform) -> &'static str {
    match platform {
        NativeUiPlatform::Windows => "real Windows Win32/GDI host smoke verification",
        NativeUiPlatform::Macos => "real macOS AppKit host smoke verification",
        NativeUiPlatform::Linux => "real Ubuntu GTK host smoke verification",
    }
}

pub(crate) fn zsui_native_target_smoke_steps(user_feature_name: &str) -> Vec<&'static str> {
    match user_feature_name {
        "right_click_edit" => vec![
            "open native clipboard window with at least one text row",
            "select a row and invoke the shared row edit action",
            "verify the native edit surface uses a multiline text editor",
            "change text, save, and confirm the row refreshes with edited content",
            "change text again, close without saving, and verify the unsaved-change prompt",
        ],
        "right_click_copy" => vec![
            "select a row and invoke the shared row copy action",
            "verify text, image, and file payload copies use the native clipboard host",
        ],
        "right_click_paste" => vec![
            "focus a target text input",
            "select a row and invoke the shared row paste action",
            "verify the native paste target receives the payload",
        ],
        "right_click_delete" => vec![
            "select a row and invoke the shared row delete action",
            "verify the row disappears after the native list refreshes",
        ],
        "right_click_pin" => vec![
            "select a row and invoke the shared row pin action",
            "verify pin state and sorted row position update after refresh",
        ],
        "right_click_group" | "grouping" => vec![
            "create or select a group in the native group surface",
            "assign a selected row to the group from the shared row menu",
            "filter by group and verify the native list projection changes",
        ],
        "search" => vec![
            "focus the native search control from the app shortcut",
            "type a query and verify the native row list filters",
            "clear or hide search and verify list focus returns",
        ],
        "vv_mode" => vec![
            "trigger VV mode from the native shortcut path",
            "verify the native VV popup renders row numbers and previews",
            "select an item and verify the paste target receives it",
        ],
        "settings_pages" => vec![
            "open the native settings window",
            "visit every settings page or tab",
            "toggle representative shared settings controls and verify model refresh",
        ],
        "sync_webdav" | "sync_lan" => vec![
            "open native settings sync controls",
            "toggle the shared sync setting",
            "run a target sync smoke and verify status feedback",
        ],
        "status_menu" => vec![
            "open the native tray or status menu",
            "verify shared status menu entries, separators, and actions",
        ],
        "dialogs" => vec![
            "open native input, confirm, info, and edit dialog paths",
            "verify each dialog returns the shared app_core response",
        ],
        "clipboard_payloads" => vec![
            "capture or create text, image, and file clipboard rows",
            "copy and paste each payload through the native clipboard host",
        ],
        "file_image_text" => vec![
            "capture text, image, and file rows in the native list",
            "verify row presentation, copy, paste, and open-path behavior",
        ],
        _ => vec![
            "run the target native host",
            "exercise the shared app_core feature path",
            "verify native UI output and app_core state refresh",
        ],
    }
}

pub(crate) fn zsui_native_target_smoke_work_items() -> Vec<ZsuiNativeTargetSmokeWorkItem> {
    zsui_user_feature_platform_statuses()
        .into_iter()
        .filter(|status| status.target_smoke_required && !status.target_smoke_verified)
        .map(|status| {
            let mut platform_host_module_paths = Vec::new();
            for ingress_name in &status.ui_ingress_names {
                push_unique_names(
                    &mut platform_host_module_paths,
                    &zsui_platform_host_modules_for_ingress(status.platform, ingress_name),
                );
            }

            ZsuiNativeTargetSmokeWorkItem {
                platform: status.platform,
                platform_name: status.platform_name,
                target_environment_name: zsui_native_target_smoke_environment_name(status.platform),
                user_feature_name: status.user_feature_name,
                display_name: status.display_name,
                support_status_name: status.support_status_name,
                ui_ingress_names: status.ui_ingress_names,
                native_component_family_names: status.native_component_family_names,
                typed_component_spec_names: status.typed_component_spec_names,
                preferred_app_core_edit_modules: status.preferred_app_core_edit_modules,
                platform_host_module_paths,
                required_native_feature_names: status.required_native_feature_names,
                target_smoke_name: status
                    .next_missing_requirement
                    .unwrap_or("target native smoke verification"),
                target_smoke_steps: zsui_native_target_smoke_steps(status.user_feature_name),
                code_level_ready: status.code_level_ready,
                target_smoke_required: status.target_smoke_required,
                target_smoke_verified: status.target_smoke_verified,
                system_complete: status.system_complete,
                next_missing_requirement: status.next_missing_requirement,
            }
        })
        .collect()
}

pub(crate) fn zsui_native_target_smoke_work_items_for_platform(
    platform: NativeUiPlatform,
) -> Vec<ZsuiNativeTargetSmokeWorkItem> {
    zsui_native_target_smoke_work_items()
        .into_iter()
        .filter(|item| item.platform == platform)
        .collect()
}

pub(crate) const ZSUI_NATIVE_TARGET_SMOKE_BATCH_SIZE: usize = 5;

pub(crate) fn zsui_native_target_smoke_batch_for_platform(
    platform: NativeUiPlatform,
) -> Vec<ZsuiNativeTargetSmokeWorkItem> {
    zsui_native_target_smoke_work_items_for_platform(platform)
        .into_iter()
        .take(ZSUI_NATIVE_TARGET_SMOKE_BATCH_SIZE)
        .collect()
}

pub(crate) fn zsui_native_target_smoke_batch_for_macos_and_linux(
) -> Vec<ZsuiNativeTargetSmokeWorkItem> {
    [NativeUiPlatform::Macos, NativeUiPlatform::Linux]
        .into_iter()
        .flat_map(zsui_native_target_smoke_batch_for_platform)
        .collect()
}

pub(crate) fn zsui_native_target_smoke_work_item_for(
    platform: NativeUiPlatform,
    user_feature_name: &str,
) -> Option<ZsuiNativeTargetSmokeWorkItem> {
    zsui_native_target_smoke_work_items()
        .into_iter()
        .find(|item| item.platform == platform && item.user_feature_name == user_feature_name)
}

pub(crate) fn zsui_next_native_target_smoke_work_item_for_platform(
    platform: NativeUiPlatform,
) -> Option<ZsuiNativeTargetSmokeWorkItem> {
    zsui_native_target_smoke_work_items_for_platform(platform)
        .into_iter()
        .find(|item| item.code_level_ready && item.target_smoke_required)
        .or_else(|| {
            zsui_native_target_smoke_work_items_for_platform(platform)
                .into_iter()
                .next()
        })
}

pub(crate) fn zsui_framework_manifest() -> ZsuiFrameworkManifest {
    ZsuiFrameworkManifest {
        name: ZSUI_FRAMEWORK_NAME,
        tagline: ZSUI_FRAMEWORK_TAGLINE,
        api_version: APP_CORE_API_VERSION,
        framework_layers: zsui_framework_layers(),
        boundary_rules: zsui_framework_boundary_rules(),
        native_feature_parity: zsui_native_feature_parity_statuses(),
        clipboard_mvp_feature_matrix: zsui_clipboard_mvp_feature_matrix(),
        lan_sync_capability_matrix: zsui_lan_sync_capability_matrix(),
        window_system_capability_matrix: zsui_window_system_capability_matrix(),
        window_system_backend_work_items: zsui_window_system_backend_work_items(),
        user_feature_platform_statuses: zsui_user_feature_platform_statuses(),
        user_feature_completion_summaries: zsui_user_feature_completion_summaries(),
        user_feature_cross_platform_summaries: zsui_user_feature_cross_platform_summaries(),
        user_feature_release_progress: zsui_user_feature_release_progress(),
        ui_ingress_classifications: zsui_ui_ingress_classifications(),
        ui_extension_recipes: zsui_ui_extension_recipes(),
        native_component_families: native_component_family_descriptors(),
        native_feature_ui_ingress_requirements: zsui_native_feature_ui_ingress_requirements(),
        ui_protocol_convergence: zsui_ui_protocol_convergence(),
        native_ui_protocol_surfaces: native_ui_protocol_surfaces().to_vec(),
        native_ui_protocol_host_statuses: zsui_native_ui_protocol_host_statuses(),
        host_private_ui_ingress_audits: zsui_host_private_ui_ingress_audits(),
        native_ui_host_translation_work_items: zsui_native_ui_host_translation_work_items(),
        native_target_smoke_work_items: zsui_native_target_smoke_work_items(),
        native_platforms: SUPPORTED_NATIVE_UI_PLATFORMS.to_vec(),
        native_toolkits: SUPPORTED_NATIVE_UI_TOOLKITS.to_vec(),
        native_backends: SUPPORTED_NATIVE_UI_BACKENDS.to_vec(),
        native_adapter_capabilities: REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES.to_vec(),
        required_host_surfaces: REQUIRED_UI_HOST_SURFACES.to_vec(),
        shared_non_host_protocols: SHARED_NON_HOST_UI_PROTOCOLS.to_vec(),
        product_adapter: product_adapter_integration_contract(),
        ai: product_ai_integration_manifest(),
    }
}

pub(crate) fn zsui_reuse_readiness_report() -> ZsuiReuseReadinessReport {
    let manifest = zsui_framework_manifest();
    let product_adapter = product_adapter_reuse_checklist();
    ZsuiReuseReadinessReport {
        platform_names: manifest
            .native_platforms
            .iter()
            .map(|platform| platform.platform_name())
            .collect(),
        native_runtime_ready_platforms: manifest
            .native_backends
            .iter()
            .filter(|backend| backend.status.is_native_runtime_ready())
            .map(|backend| backend.platform_name())
            .collect(),
        first_pass_native_host_platforms: manifest
            .native_backends
            .iter()
            .filter(|backend| backend.status.is_first_pass_native_host())
            .map(|backend| backend.platform_name())
            .collect(),
        scaffold_platforms: manifest
            .native_backends
            .iter()
            .filter(|backend| backend.status.is_scaffold())
            .map(|backend| backend.platform_name())
            .collect(),
        native_adapter_capability_names: manifest
            .native_adapter_capabilities
            .iter()
            .map(|capability| capability.capability_name())
            .collect(),
        product_adapter_surface_names: product_adapter.surface_names,
        product_adapter_task_names: product_adapter.task_names,
        ai_provider_names: product_adapter.ai_provider_names,
        ai_executor_boundary_names: product_adapter.ai_executor_boundary_names,
        adapter_parity: None,
    }
}

pub(crate) fn zsui_reuse_readiness_report_with_adapter_parity(
    packages: &[NativeUiAdapterReusePackage<ZsuiReuseBootstrapPlan>],
) -> ZsuiReuseReadinessReport {
    let mut report = zsui_reuse_readiness_report();
    report.adapter_parity = Some(native_ui_adapter_parity_report(packages));
    report
}

pub(crate) fn zsui_reuse_bootstrap_plan(
    platform: NativeUiPlatform,
) -> Option<ZsuiReuseBootstrapPlan> {
    let native = native_ui_backend_capability_matrix_for_platform(platform)?;
    let product_adapter = product_adapter_reuse_checklist();
    let native_runtime_gate_plans = zsui_native_runtime_gate_plans();
    let native_runtime_gate_binding_plans =
        zsui_native_runtime_gate_binding_plans(platform, &native_runtime_gate_plans);
    let native_runtime_gate_names: Vec<&'static str> = native_runtime_gate_plans
        .iter()
        .map(|gate| gate.gate_name)
        .collect();
    let missing_native_runtime_gate_names =
        zsui_missing_native_runtime_gate_names(platform, native.backend.status);
    let next_native_runtime_gate_name = missing_native_runtime_gate_names.first().copied();
    let native_runtime_gate_completion = zsui_native_runtime_gate_completion_report(
        native_runtime_gate_names.clone(),
        missing_native_runtime_gate_names.clone(),
    );
    Some(ZsuiReuseBootstrapPlan {
        platform,
        platform_name: native.backend.platform_name(),
        toolkit_name: native.backend.toolkit_name(),
        backend_status: native.backend.status,
        backend_status_name: native.backend.status_name(),
        adapter_boundary: native.backend.adapter_boundary,
        adapter_module_path: native.backend.module_path,
        native_adapter_capability_names: native.required_capability_names(),
        product_adapter_surface_names: product_adapter.surface_names,
        product_adapter_task_names: product_adapter.task_names,
        ai_provider_names: product_adapter.ai_provider_names,
        ai_executor_boundary_names: product_adapter.ai_executor_boundary_names,
        native_runtime_gate_names,
        missing_native_runtime_gate_names,
        next_native_runtime_gate_name,
        native_runtime_gate_plans,
        native_runtime_gate_binding_plans,
        native_runtime_gate_completion,
    })
}

pub(crate) fn zsui_adapter_reuse_package_gate_binding_summaries(
    packages: &[NativeUiAdapterReusePackage<ZsuiReuseBootstrapPlan>],
) -> Vec<ZsuiAdapterReusePackageGateBindingSummary> {
    packages
        .iter()
        .map(|package| {
            let all_gate_bindings_present_in_adapter = package
                .bootstrap
                .native_runtime_gate_binding_plans
                .iter()
                .flat_map(|gate| gate.required_platform_binding_names.iter())
                .all(|binding_name| package.binding_plan.has_binding_name(binding_name));

            ZsuiAdapterReusePackageGateBindingSummary {
                platform_name: package.platform_name(),
                toolkit_name: package.toolkit_name(),
                status_name: package.status_name(),
                adapter_boundary: package.binding_plan.adapter_boundary,
                gate_names: package.bootstrap.native_runtime_gate_names.clone(),
                gate_binding_counts: package
                    .bootstrap
                    .native_runtime_gate_binding_plans
                    .iter()
                    .map(|gate| gate.required_platform_binding_names.len())
                    .collect(),
                missing_gate_names: package
                    .bootstrap
                    .native_runtime_gate_completion
                    .missing_gate_names
                    .clone(),
                next_gate_name: package
                    .bootstrap
                    .native_runtime_gate_completion
                    .next_gate_name,
                completion_percent: package
                    .bootstrap
                    .native_runtime_gate_completion
                    .completion_percent,
                all_gate_bindings_present_in_adapter,
            }
        })
        .collect()
}

pub(crate) fn zsui_adapter_reuse_package_porting_work_items(
    packages: &[NativeUiAdapterReusePackage<ZsuiReuseBootstrapPlan>],
) -> Vec<ZsuiAdapterPortingWorkItem> {
    packages
        .iter()
        .flat_map(|package| {
            package
                .bootstrap
                .native_runtime_gate_binding_plans
                .iter()
                .filter(|gate| {
                    package
                        .bootstrap
                        .missing_native_runtime_gate_names
                        .contains(&gate.gate_name)
                })
                .map(|gate| ZsuiAdapterPortingWorkItem {
                    platform_name: package.platform_name(),
                    toolkit_name: package.toolkit_name(),
                    status_name: package.status_name(),
                    adapter_boundary: package.binding_plan.adapter_boundary,
                    adapter_module_path: package.bootstrap.adapter_module_path,
                    gate_name: gate.gate_name,
                    required_adapter_capability_names: gate
                        .required_adapter_capability_names
                        .clone(),
                    required_platform_binding_names: gate.required_platform_binding_names.clone(),
                    required_product_adapter_task_names: gate
                        .required_product_adapter_task_names
                        .clone(),
                    required_ai_executor_boundary_names: gate
                        .required_ai_executor_boundary_names
                        .clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn zsui_agent_context() -> ZsuiAgentContext {
    ZsuiAgentContext {
        framework_name: ZSUI_FRAMEWORK_NAME,
        api_version: APP_CORE_API_VERSION,
        framework_layers: zsui_framework_layers(),
        boundary_rules: zsui_framework_boundary_rules(),
        native_feature_parity: zsui_native_feature_parity_statuses(),
        clipboard_mvp_feature_matrix: zsui_clipboard_mvp_feature_matrix(),
        lan_sync_capability_matrix: zsui_lan_sync_capability_matrix(),
        window_system_capability_matrix: zsui_window_system_capability_matrix(),
        window_system_backend_work_items: zsui_window_system_backend_work_items(),
        user_feature_platform_statuses: zsui_user_feature_platform_statuses(),
        user_feature_completion_summaries: zsui_user_feature_completion_summaries(),
        user_feature_cross_platform_summaries: zsui_user_feature_cross_platform_summaries(),
        user_feature_release_progress: zsui_user_feature_release_progress(),
        ui_ingress_classifications: zsui_ui_ingress_classifications(),
        ui_extension_recipes: zsui_ui_extension_recipes(),
        native_component_families: native_component_family_descriptors(),
        native_feature_ui_ingress_requirements: zsui_native_feature_ui_ingress_requirements(),
        ui_protocol_convergence: zsui_ui_protocol_convergence(),
        native_ui_protocol_surfaces: native_ui_protocol_surfaces().to_vec(),
        native_ui_protocol_host_statuses: zsui_native_ui_protocol_host_statuses(),
        host_private_ui_ingress_audits: zsui_host_private_ui_ingress_audits(),
        native_ui_host_translation_work_items: zsui_native_ui_host_translation_work_items(),
        native_target_smoke_work_items: zsui_native_target_smoke_work_items(),
        readiness: zsui_reuse_readiness_report(),
        platform_bootstrap: zsui_agent_platform_bootstrap_summaries(),
        platform_runtime_gates: zsui_agent_platform_runtime_gates(),
        porting_work_items: Vec::new(),
        user_feature_work_items: zsui_user_feature_work_items(),
        reusable_app_blueprint: zsui_reusable_app_blueprint(),
        integration_steps: zsui_agent_integration_steps(),
        ai_routes: zsui_agent_ai_route_summaries(),
    }
}

pub(crate) fn zsui_agent_context_with_adapter_parity(
    packages: &[NativeUiAdapterReusePackage<ZsuiReuseBootstrapPlan>],
) -> ZsuiAgentContext {
    ZsuiAgentContext {
        framework_name: ZSUI_FRAMEWORK_NAME,
        api_version: APP_CORE_API_VERSION,
        framework_layers: zsui_framework_layers(),
        boundary_rules: zsui_framework_boundary_rules(),
        native_feature_parity: zsui_native_feature_parity_statuses(),
        clipboard_mvp_feature_matrix: zsui_clipboard_mvp_feature_matrix(),
        lan_sync_capability_matrix: zsui_lan_sync_capability_matrix(),
        window_system_capability_matrix: zsui_window_system_capability_matrix(),
        window_system_backend_work_items: zsui_window_system_backend_work_items(),
        user_feature_platform_statuses: zsui_user_feature_platform_statuses(),
        user_feature_completion_summaries: zsui_user_feature_completion_summaries(),
        user_feature_cross_platform_summaries: zsui_user_feature_cross_platform_summaries(),
        user_feature_release_progress: zsui_user_feature_release_progress(),
        ui_ingress_classifications: zsui_ui_ingress_classifications(),
        ui_extension_recipes: zsui_ui_extension_recipes(),
        native_component_families: native_component_family_descriptors(),
        native_feature_ui_ingress_requirements: zsui_native_feature_ui_ingress_requirements(),
        ui_protocol_convergence: zsui_ui_protocol_convergence(),
        native_ui_protocol_surfaces: native_ui_protocol_surfaces().to_vec(),
        native_ui_protocol_host_statuses: zsui_native_ui_protocol_host_statuses(),
        host_private_ui_ingress_audits: zsui_host_private_ui_ingress_audits(),
        native_ui_host_translation_work_items: zsui_native_ui_host_translation_work_items(),
        native_target_smoke_work_items: zsui_native_target_smoke_work_items(),
        readiness: zsui_reuse_readiness_report_with_adapter_parity(packages),
        platform_bootstrap: zsui_agent_platform_bootstrap_summaries(),
        platform_runtime_gates: zsui_agent_platform_runtime_gates(),
        porting_work_items: zsui_adapter_reuse_package_porting_work_items(packages),
        user_feature_work_items: zsui_user_feature_work_items(),
        reusable_app_blueprint: zsui_reusable_app_blueprint(),
        integration_steps: zsui_agent_integration_steps(),
        ai_routes: zsui_agent_ai_route_summaries(),
    }
}

fn zsui_agent_platform_bootstrap_summaries() -> Vec<ZsuiAgentPlatformBootstrapSummary> {
    SUPPORTED_NATIVE_UI_PLATFORMS
        .iter()
        .filter_map(|platform| zsui_reuse_bootstrap_plan(*platform))
        .map(|plan| ZsuiAgentPlatformBootstrapSummary {
            platform_name: plan.platform_name,
            toolkit_name: plan.toolkit_name,
            backend_status_name: plan.backend_status_name,
            adapter_boundary: plan.adapter_boundary,
            adapter_module_path: plan.adapter_module_path,
            native_adapter_capability_names: plan.native_adapter_capability_names,
        })
        .collect()
}

fn zsui_native_runtime_gate_names() -> Vec<&'static str> {
    zsui_native_runtime_gate_plans()
        .iter()
        .map(|gate| gate.gate_name)
        .collect()
}

fn zsui_completed_native_runtime_gate_names(
    platform: NativeUiPlatform,
    status: NativeUiBackendStatus,
) -> Vec<&'static str> {
    if status.is_native_runtime_ready() {
        return zsui_native_runtime_gate_names();
    }

    match platform {
        NativeUiPlatform::Windows => Vec::new(),
        NativeUiPlatform::Macos | NativeUiPlatform::Linux => zsui_native_runtime_gate_names(),
    }
}

fn zsui_missing_native_runtime_gate_names(
    platform: NativeUiPlatform,
    status: NativeUiBackendStatus,
) -> Vec<&'static str> {
    let completed_gate_names = zsui_completed_native_runtime_gate_names(platform, status);
    zsui_native_runtime_gate_names()
        .into_iter()
        .filter(|gate_name| !completed_gate_names.contains(gate_name))
        .collect()
}

fn zsui_native_runtime_gate_completion_report(
    gate_names: Vec<&'static str>,
    missing_gate_names: Vec<&'static str>,
) -> ZsuiNativeRuntimeGateCompletionReport {
    let total_gate_count = gate_names.len();
    let missing_gate_count = missing_gate_names.len();
    let completed_gate_names = gate_names
        .into_iter()
        .filter(|gate_name| !missing_gate_names.contains(gate_name))
        .collect::<Vec<_>>();
    let completed_gate_count = completed_gate_names.len();
    let completion_percent = zsui_percent(completed_gate_count, total_gate_count);
    let next_gate_name = missing_gate_names.first().copied();

    ZsuiNativeRuntimeGateCompletionReport {
        total_gate_count,
        completed_gate_count,
        missing_gate_count,
        completion_percent,
        completed_gate_names,
        missing_gate_names,
        next_gate_name,
    }
}

fn zsui_percent(done: usize, total: usize) -> u8 {
    if total == 0 {
        100
    } else {
        ((done * 100) / total) as u8
    }
}

fn zsui_native_runtime_gate_plans() -> Vec<ZsuiNativeRuntimeGateCapabilityPlan> {
    use NativeUiAdapterCapability::{
        Clipboard, EditDialog, FileDialog, Ime, InputDialog, MainExecutionPlanBridge,
        MainSearchControl, MainWindow, PasteTarget, PopupMenu, Renderer, SettingsDropdown,
        SettingsWindow, ShellOpen, StatusItem, TextLayout, TransientWindow, WindowIdentity,
    };
    use ProductAdapterIntegrationTask::{
        ConnectLlmExecutor, ConnectProductAiTools, ConnectSkillRegistry, PublishAiCatalog,
    };
    use ProductAiExecutorBoundary::{LlmExecutor, ProductAdapterTools, SkillRegistry};

    vec![
        zsui_native_runtime_gate_plan(
            "native_event_loop",
            &[MainWindow, StatusItem, MainExecutionPlanBridge],
            &[],
            &[],
        ),
        zsui_native_runtime_gate_plan(
            "native_window_surfaces",
            &[
                MainWindow,
                SettingsWindow,
                SettingsDropdown,
                InputDialog,
                EditDialog,
                TransientWindow,
            ],
            &[],
            &[],
        ),
        zsui_native_runtime_gate_plan(
            "native_control_mapping",
            &[
                MainSearchControl,
                SettingsDropdown,
                InputDialog,
                EditDialog,
                Ime,
            ],
            &[],
            &[],
        ),
        zsui_native_runtime_gate_plan("native_renderer", &[Renderer, TextLayout], &[], &[]),
        zsui_native_runtime_gate_plan(
            "native_clipboard_services",
            &[Clipboard, PasteTarget, WindowIdentity],
            &[],
            &[],
        ),
        zsui_native_runtime_gate_plan(
            "native_dialog_services",
            &[PopupMenu, ShellOpen, FileDialog, InputDialog, EditDialog],
            &[],
            &[],
        ),
        zsui_native_runtime_gate_plan(
            "native_settings_surfaces",
            &[SettingsWindow, SettingsDropdown],
            &[],
            &[],
        ),
        zsui_native_runtime_gate_plan(
            "ai_action_presentation",
            &[PopupMenu, SettingsWindow, MainExecutionPlanBridge],
            &[
                PublishAiCatalog,
                ConnectLlmExecutor,
                ConnectSkillRegistry,
                ConnectProductAiTools,
            ],
            &[LlmExecutor, SkillRegistry, ProductAdapterTools],
        ),
    ]
}

fn zsui_reusable_app_feature_requirements() -> Vec<ZsuiReusableAppFeatureRequirement> {
    vec![
        ZsuiReusableAppFeatureRequirement {
            feature_name: "native_app_entry",
            required_runtime_gate_names: vec!["native_event_loop"],
            required_adapter_capability_names: vec![
                "main_window",
                "status_item",
                "main_execution_plan_bridge",
            ],
            required_product_adapter_task_names: Vec::new(),
            required_ai_executor_boundary_names: Vec::new(),
        },
        ZsuiReusableAppFeatureRequirement {
            feature_name: "window_surfaces",
            required_runtime_gate_names: vec!["native_window_surfaces"],
            required_adapter_capability_names: vec![
                "main_window",
                "settings_window",
                "settings_dropdown",
                "input_dialog",
                "edit_dialog",
                "transient_window",
            ],
            required_product_adapter_task_names: Vec::new(),
            required_ai_executor_boundary_names: Vec::new(),
        },
        ZsuiReusableAppFeatureRequirement {
            feature_name: "control_mapping",
            required_runtime_gate_names: vec!["native_control_mapping"],
            required_adapter_capability_names: vec![
                "main_search_control",
                "settings_dropdown",
                "input_dialog",
                "edit_dialog",
                "ime",
            ],
            required_product_adapter_task_names: Vec::new(),
            required_ai_executor_boundary_names: Vec::new(),
        },
        ZsuiReusableAppFeatureRequirement {
            feature_name: "renderer_text_layout",
            required_runtime_gate_names: vec!["native_renderer"],
            required_adapter_capability_names: vec!["renderer", "text_layout"],
            required_product_adapter_task_names: Vec::new(),
            required_ai_executor_boundary_names: Vec::new(),
        },
        ZsuiReusableAppFeatureRequirement {
            feature_name: "system_services",
            required_runtime_gate_names: vec![
                "native_clipboard_services",
                "native_dialog_services",
            ],
            required_adapter_capability_names: vec![
                "clipboard",
                "paste_target",
                "window_identity",
                "popup_menu",
                "shell_open",
                "file_dialog",
                "input_dialog",
                "edit_dialog",
            ],
            required_product_adapter_task_names: Vec::new(),
            required_ai_executor_boundary_names: Vec::new(),
        },
        ZsuiReusableAppFeatureRequirement {
            feature_name: "settings_surfaces",
            required_runtime_gate_names: vec!["native_settings_surfaces"],
            required_adapter_capability_names: vec!["settings_window", "settings_dropdown"],
            required_product_adapter_task_names: Vec::new(),
            required_ai_executor_boundary_names: Vec::new(),
        },
        ZsuiReusableAppFeatureRequirement {
            feature_name: "ai_action_surfaces",
            required_runtime_gate_names: vec!["ai_action_presentation"],
            required_adapter_capability_names: vec![
                "popup_menu",
                "settings_window",
                "main_execution_plan_bridge",
            ],
            required_product_adapter_task_names: vec![
                "publish_ai_catalog",
                "connect_llm_executor",
                "connect_skill_registry",
                "connect_product_ai_tools",
            ],
            required_ai_executor_boundary_names: vec![
                "llm_executor",
                "skill_registry",
                "product_adapter_tools",
            ],
        },
    ]
}

fn zsui_reusable_app_feature_platform_statuses() -> Vec<ZsuiReusableAppFeaturePlatformStatus> {
    let requirements = zsui_reusable_app_feature_requirements();
    SUPPORTED_NATIVE_UI_PLATFORMS
        .iter()
        .filter_map(|platform| zsui_reuse_bootstrap_plan(*platform))
        .flat_map(|plan| {
            requirements.iter().map(move |requirement| {
                let blocking_runtime_gate_names = requirement
                    .required_runtime_gate_names
                    .iter()
                    .copied()
                    .filter(|gate_name| plan.missing_native_runtime_gate_names.contains(gate_name))
                    .collect::<Vec<_>>();
                let required_platform_binding_names = requirement
                    .required_adapter_capability_names
                    .iter()
                    .filter_map(|capability_name| {
                        zsui_platform_binding_name_for_capability(plan.platform, capability_name)
                    })
                    .collect::<Vec<_>>();

                ZsuiReusableAppFeaturePlatformStatus {
                    platform_name: plan.platform_name,
                    toolkit_name: plan.toolkit_name,
                    backend_status_name: plan.backend_status_name,
                    adapter_module_path: plan.adapter_module_path,
                    feature_name: requirement.feature_name,
                    runtime_status_name: plan.backend_status_name,
                    runtime_ready: blocking_runtime_gate_names.is_empty()
                        && plan.native_runtime_ready(),
                    required_runtime_gate_names: requirement.required_runtime_gate_names.clone(),
                    blocking_runtime_gate_names,
                    required_platform_binding_names,
                    required_product_adapter_task_names: requirement
                        .required_product_adapter_task_names
                        .clone(),
                    required_ai_executor_boundary_names: requirement
                        .required_ai_executor_boundary_names
                        .clone(),
                }
            })
        })
        .collect()
}

pub(crate) fn zsui_reusable_app_blueprint() -> ZsuiReusableAppBlueprint {
    let readiness = zsui_reuse_readiness_report();
    ZsuiReusableAppBlueprint {
        framework_name: ZSUI_FRAMEWORK_NAME,
        api_version: APP_CORE_API_VERSION,
        rust_ui_language_name: "zsui_rust_ui_contract",
        native_platform_names: readiness.platform_names,
        native_runtime_driver_operation_names: required_native_runtime_driver_operation_names(),
        runtime_harness_stage_names: zsui_reusable_runtime_harness_stage_names(),
        reusable_feature_names: zsui_reusable_app_feature_requirements()
            .iter()
            .map(|feature| feature.feature_name)
            .collect(),
        product_adapter_surface_names: readiness.product_adapter_surface_names,
        product_adapter_task_names: readiness.product_adapter_task_names,
        product_adapter_method_names: required_product_adapter_host_method_names(),
        product_function_flows: product_adapter_function_flows(),
        product_execution_pipeline: product_adapter_execution_pipeline(),
        ai_executor_boundary_names: readiness.ai_executor_boundary_names,
        feature_statuses: zsui_reusable_app_feature_platform_statuses(),
    }
}

fn zsui_native_runtime_gate_plan(
    gate_name: &'static str,
    adapter_capabilities: &[NativeUiAdapterCapability],
    product_adapter_tasks: &[ProductAdapterIntegrationTask],
    ai_executor_boundaries: &[ProductAiExecutorBoundary],
) -> ZsuiNativeRuntimeGateCapabilityPlan {
    ZsuiNativeRuntimeGateCapabilityPlan {
        gate_name,
        required_adapter_capability_names: adapter_capabilities
            .iter()
            .map(|capability| capability.capability_name())
            .collect(),
        required_product_adapter_task_names: product_adapter_tasks
            .iter()
            .map(|task| task.task_name())
            .collect(),
        required_ai_executor_boundary_names: ai_executor_boundaries
            .iter()
            .map(|boundary| boundary.boundary_name())
            .collect(),
    }
}

fn zsui_native_runtime_gate_binding_plans(
    platform: NativeUiPlatform,
    gate_plans: &[ZsuiNativeRuntimeGateCapabilityPlan],
) -> Vec<ZsuiNativeRuntimeGatePlatformBindingPlan> {
    gate_plans
        .iter()
        .map(|gate| ZsuiNativeRuntimeGatePlatformBindingPlan {
            gate_name: gate.gate_name,
            required_adapter_capability_names: gate.required_adapter_capability_names.clone(),
            required_platform_binding_names: gate
                .required_adapter_capability_names
                .iter()
                .filter_map(|capability_name| {
                    zsui_platform_binding_name_for_capability(platform, capability_name)
                })
                .collect(),
            required_product_adapter_task_names: gate.required_product_adapter_task_names.clone(),
            required_ai_executor_boundary_names: gate.required_ai_executor_boundary_names.clone(),
        })
        .collect()
}

fn zsui_platform_binding_name_for_capability(
    platform: NativeUiPlatform,
    capability_name: &str,
) -> Option<&'static str> {
    match (platform, capability_name) {
        (NativeUiPlatform::Windows, "main_window") => Some("win32_main_window_pair"),
        (NativeUiPlatform::Windows, "settings_window") => Some("win32_settings_window"),
        (NativeUiPlatform::Windows, "settings_dropdown") => Some("win32_dropdown_popup"),
        (NativeUiPlatform::Windows, "input_dialog") => Some("win32_text_input_dialog"),
        (NativeUiPlatform::Windows, "edit_dialog") => Some("win32_edit_text_dialog"),
        (NativeUiPlatform::Windows, "clipboard") => Some("windows_clipboard_host"),
        (NativeUiPlatform::Windows, "popup_menu") => Some("win32_popup_menu_host"),
        (NativeUiPlatform::Windows, "status_item") => Some("shell_notify_icon_status_item"),
        (NativeUiPlatform::Windows, "renderer") => Some("gdi_renderer"),
        (NativeUiPlatform::Windows, "text_layout") => Some("gdi_text_layout"),
        (NativeUiPlatform::Windows, "main_search_control") => Some("win32_edit_search_control"),
        (NativeUiPlatform::Windows, "transient_window") => {
            Some("win32_no_activate_transient_window")
        }
        (NativeUiPlatform::Windows, "ime") => Some("imm32_ime_bridge"),
        (NativeUiPlatform::Windows, "shell_open") => Some("shell_execute_launcher"),
        (NativeUiPlatform::Windows, "file_dialog") => Some("win32_open_file_dialog"),
        (NativeUiPlatform::Windows, "paste_target") => Some("win32_paste_target"),
        (NativeUiPlatform::Windows, "window_identity") => Some("win32_window_identity"),
        (NativeUiPlatform::Windows, "main_execution_plan_bridge") => {
            Some("shared_main_execution_plan_bridge")
        }
        (NativeUiPlatform::Macos, "main_window") => Some("ns_window_pair"),
        (NativeUiPlatform::Macos, "settings_window") => Some("settings_window_controller"),
        (NativeUiPlatform::Macos, "settings_dropdown") => Some("ns_pop_up_button_or_menu"),
        (NativeUiPlatform::Macos, "input_dialog") => Some("ns_alert_text_field_dialog"),
        (NativeUiPlatform::Macos, "edit_dialog") => Some("ns_text_view_editor_window"),
        (NativeUiPlatform::Macos, "clipboard") => Some("ns_pasteboard_bridge"),
        (NativeUiPlatform::Macos, "popup_menu") => Some("ns_menu_bridge"),
        (NativeUiPlatform::Macos, "status_item") => Some("ns_status_item_bridge"),
        (NativeUiPlatform::Macos, "renderer") => Some("core_graphics_renderer"),
        (NativeUiPlatform::Macos, "text_layout") => Some("core_text_layout"),
        (NativeUiPlatform::Macos, "main_search_control") => Some("ns_search_field"),
        (NativeUiPlatform::Macos, "transient_window") => Some("ns_panel_or_popover"),
        (NativeUiPlatform::Macos, "ime") => Some("ns_input_context_bridge"),
        (NativeUiPlatform::Macos, "shell_open") => Some("ns_workspace_launcher"),
        (NativeUiPlatform::Macos, "file_dialog") => Some("ns_open_panel"),
        (NativeUiPlatform::Macos, "paste_target") => Some("accessibility_paste_target"),
        (NativeUiPlatform::Macos, "window_identity") => {
            Some("ns_running_application_window_identity")
        }
        (NativeUiPlatform::Macos, "main_execution_plan_bridge") => {
            Some("shared_main_execution_plan_bridge")
        }
        (NativeUiPlatform::Linux, "main_window") => Some("adw_application_window"),
        (NativeUiPlatform::Linux, "settings_window") => Some("adw_preferences_window"),
        (NativeUiPlatform::Linux, "settings_dropdown") => Some("gtk_popover_or_combo_row"),
        (NativeUiPlatform::Linux, "input_dialog") => Some("adw_entry_dialog"),
        (NativeUiPlatform::Linux, "edit_dialog") => Some("adw_text_editor_dialog"),
        (NativeUiPlatform::Linux, "clipboard") => Some("gdk_clipboard_bridge"),
        (NativeUiPlatform::Linux, "popup_menu") => Some("gtk_popover_menu"),
        (NativeUiPlatform::Linux, "status_item") => Some("app_indicator_status_item"),
        (NativeUiPlatform::Linux, "renderer") => Some("gtk_snapshot_renderer"),
        (NativeUiPlatform::Linux, "text_layout") => Some("pango_text_layout"),
        (NativeUiPlatform::Linux, "main_search_control") => Some("gtk_search_entry"),
        (NativeUiPlatform::Linux, "transient_window") => Some("gtk_popover_or_layer_surface"),
        (NativeUiPlatform::Linux, "ime") => Some("gtk_input_method_bridge"),
        (NativeUiPlatform::Linux, "shell_open") => Some("gio_app_info_launcher"),
        (NativeUiPlatform::Linux, "file_dialog") => Some("gtk_file_dialog"),
        (NativeUiPlatform::Linux, "paste_target") => Some("portal_or_ats_pi_paste_target"),
        (NativeUiPlatform::Linux, "window_identity") => Some("portal_or_ats_pi_window_identity"),
        (NativeUiPlatform::Linux, "main_execution_plan_bridge") => {
            Some("shared_main_execution_plan_bridge")
        }
        _ => None,
    }
}

fn zsui_agent_platform_runtime_gates() -> Vec<ZsuiAgentPlatformRuntimeGate> {
    SUPPORTED_NATIVE_UI_PLATFORMS
        .iter()
        .filter_map(|platform| zsui_reuse_bootstrap_plan(*platform))
        .map(|plan| ZsuiAgentPlatformRuntimeGate {
            platform_name: plan.platform_name,
            toolkit_name: plan.toolkit_name,
            backend_status_name: plan.backend_status_name,
            native_runtime_ready: plan.native_runtime_ready(),
            gate_names: plan.native_runtime_gate_names,
            missing_gate_names: plan.missing_native_runtime_gate_names,
            next_gate_name: plan.next_native_runtime_gate_name,
            gate_plans: plan.native_runtime_gate_plans,
            gate_binding_plans: plan.native_runtime_gate_binding_plans,
            completion: plan.native_runtime_gate_completion,
        })
        .collect()
}

fn zsui_agent_integration_steps() -> Vec<ZsuiAgentIntegrationStep> {
    let readiness = zsui_reuse_readiness_report();
    vec![
        ZsuiAgentIntegrationStep {
            step_name: "select_native_adapter",
            owner_name: "native_adapter",
            required_names: readiness.platform_names,
        },
        ZsuiAgentIntegrationStep {
            step_name: "verify_adapter_capability_parity",
            owner_name: "native_adapter",
            required_names: readiness.native_adapter_capability_names,
        },
        ZsuiAgentIntegrationStep {
            step_name: "implement_product_adapter_surfaces",
            owner_name: "product_adapter",
            required_names: readiness.product_adapter_surface_names,
        },
        ZsuiAgentIntegrationStep {
            step_name: "complete_product_adapter_tasks",
            owner_name: "product_adapter",
            required_names: readiness.product_adapter_task_names,
        },
        ZsuiAgentIntegrationStep {
            step_name: "connect_llm_executor",
            owner_name: "ai_executor",
            required_names: vec!["llm_executor"],
        },
        ZsuiAgentIntegrationStep {
            step_name: "connect_skill_registry",
            owner_name: "ai_executor",
            required_names: vec!["skill_registry"],
        },
        ZsuiAgentIntegrationStep {
            step_name: "connect_product_ai_tools",
            owner_name: "ai_executor",
            required_names: vec!["product_adapter_tools"],
        },
    ]
}

fn zsui_agent_ai_route_summaries() -> Vec<ZsuiAgentAiRouteSummary> {
    product_ai_integration_manifest()
        .execution_routes
        .iter()
        .map(|route| ZsuiAgentAiRouteSummary {
            capability_id: route.capability_id,
            provider_name: route.provider_name(),
            executor_boundary_name: route.executor_boundary_name(),
            executor_task_name: route.executor_task_name(),
            action_name: route.action_name(),
            surface_name: route.surface_name(),
            input_context_names: route.input_context_names(),
            result_name: route.result_name(),
        })
        .collect()
}
