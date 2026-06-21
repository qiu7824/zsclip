//! ZSUI framework identity and shared Rust UI layering.
//!
//! ZSUI is the reusable UI foundation being extracted from ZSClip. It keeps
//! state transitions, layout math and host contracts in Rust, while native
//! platforms provide the concrete windows, controls and drawing backends.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ApiVersion {
    pub(crate) major: u16,
    pub(crate) minor: u16,
}

pub(crate) const APP_CORE_API_VERSION: ApiVersion = ApiVersion {
    major: 0,
    minor: 78,
};

pub(crate) const ZSUI_FRAMEWORK_NAME: &str = "ZSUI";
pub(crate) const ZSUI_FRAMEWORK_TAGLINE: &str = "shared Rust UI logic with native platform hosts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiLayer {
    CoreContracts,
    LayoutModel,
    RenderProtocol,
    NativeHost,
    ProductAdapter,
}

impl ZsuiLayer {
    pub(crate) const fn is_reusable_foundation(self) -> bool {
        matches!(
            self,
            Self::CoreContracts | Self::LayoutModel | Self::RenderProtocol | Self::NativeHost
        )
    }
}
