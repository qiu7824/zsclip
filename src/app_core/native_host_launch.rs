use crate::app_core::{NativeUiPlatform, NativeUiToolkit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostLaunchMode {
    RealNativeHost,
    ContractScaffoldFallback,
}

impl NativeHostLaunchMode {
    pub(crate) const fn mode_name(self) -> &'static str {
        match self {
            Self::RealNativeHost => "real_native_host",
            Self::ContractScaffoldFallback => "contract_scaffold_fallback",
        }
    }

    pub(crate) const fn enters_real_event_loop(self) -> bool {
        matches!(self, Self::RealNativeHost)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostLaunchPlan {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) toolkit: NativeUiToolkit,
    pub(crate) entry_point: &'static str,
    pub(crate) native_application_type: &'static str,
    pub(crate) native_window_type: &'static str,
    pub(crate) real_host_module_path: &'static str,
    pub(crate) fallback_module_path: &'static str,
    pub(crate) mode: NativeHostLaunchMode,
    pub(crate) target_os_verification_required: bool,
}

impl NativeHostLaunchPlan {
    pub(crate) const fn platform_name(&self) -> &'static str {
        self.platform.platform_name()
    }

    pub(crate) const fn toolkit_name(&self) -> &'static str {
        self.toolkit.toolkit_name()
    }

    pub(crate) const fn mode_name(&self) -> &'static str {
        self.mode.mode_name()
    }

    pub(crate) const fn enters_real_event_loop(&self) -> bool {
        self.mode.enters_real_event_loop()
    }

    pub(crate) const fn needs_target_os_verification(&self) -> bool {
        self.target_os_verification_required
    }
}
