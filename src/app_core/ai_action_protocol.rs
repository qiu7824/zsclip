use super::{
    Point, ProductAdapterCommandResult, ProductAiCapability, ProductAiExecutionPlan,
    ProductAiInvocation, ProductAiUiSurface,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeAiActionMenuRequest<WindowHandle> {
    pub(crate) owner: WindowHandle,
    pub(crate) surface: ProductAiUiSurface,
    pub(crate) anchor: Point,
    pub(crate) capabilities: Vec<ProductAiCapability>,
    pub(crate) context_item_ids: Vec<i64>,
    pub(crate) prompt_text: String,
}

impl<WindowHandle> NativeAiActionMenuRequest<WindowHandle> {
    pub(crate) fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeAiSettingsSurfaceRequest<WindowHandle> {
    pub(crate) owner: Option<WindowHandle>,
    pub(crate) surface: ProductAiUiSurface,
    pub(crate) capabilities: Vec<ProductAiCapability>,
    pub(crate) provider_names: Vec<&'static str>,
}

impl<WindowHandle> NativeAiSettingsSurfaceRequest<WindowHandle> {
    pub(crate) fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

pub(crate) trait NativeAiActionPresenter {
    type WindowHandle: Copy + Eq;

    fn present_ai_action_menu(
        &mut self,
        request: NativeAiActionMenuRequest<Self::WindowHandle>,
    ) -> Option<ProductAiInvocation>;

    fn present_ai_settings_surface(
        &mut self,
        request: NativeAiSettingsSurfaceRequest<Self::WindowHandle>,
    ) -> bool;

    fn bridge_ai_execution_plan(
        &mut self,
        plan: ProductAiExecutionPlan,
    ) -> ProductAdapterCommandResult;
}

pub(crate) const REQUIRED_NATIVE_AI_ACTION_PRESENTER_OPERATIONS: [&str; 3] = [
    "present_ai_action_menu",
    "present_ai_settings_surface",
    "bridge_ai_execution_plan",
];
