use super::command_protocol::Command;
use super::native_hosts::{
    NativeRuntimeDriver, NativeRuntimeStartupRequest, NativeRuntimeStartupResult,
};
use super::product_adapter::{
    product_ai_execution_plan, ProductAdapterAsyncBridgeResult, ProductAdapterCommandResult,
    ProductAdapterHost, ProductAiInvocation,
};

#[derive(Debug)]
pub(crate) struct ZsuiReusableRuntimeHarness<Driver, Product> {
    driver: Driver,
    product: Product,
}

impl<Driver, Product> ZsuiReusableRuntimeHarness<Driver, Product>
where
    Driver: NativeRuntimeDriver,
    Product: ProductAdapterHost,
{
    pub(crate) fn new(driver: Driver, product: Product) -> Self {
        Self { driver, product }
    }

    pub(crate) fn start(
        &mut self,
        request: NativeRuntimeStartupRequest,
    ) -> NativeRuntimeStartupResult<Driver::WindowHandle> {
        self.driver.start_runtime(request)
    }

    pub(crate) fn dispatch_command(&mut self, command: Command) -> ProductAdapterCommandResult {
        self.driver.dispatch_ui_command(command.clone());
        self.product.execute_product_command(command)
    }

    pub(crate) fn poll_and_bridge_event(&mut self) -> Option<ProductAdapterAsyncBridgeResult> {
        self.driver
            .poll_application_event()
            .map(|event| self.product.bridge_async_event(event))
    }

    pub(crate) fn execute_ai_invocation(
        &mut self,
        invocation: ProductAiInvocation,
    ) -> Option<ProductAdapterCommandResult> {
        product_ai_execution_plan(invocation).map(|plan| self.product.execute_ai_plan(plan))
    }

    pub(crate) fn request_shutdown(&mut self) {
        self.driver.request_shutdown();
    }

    pub(crate) fn driver(&self) -> &Driver {
        &self.driver
    }

    pub(crate) fn product(&self) -> &Product {
        &self.product
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZsuiReusableRuntimeHarnessStage {
    StartRuntime,
    DispatchCommand,
    BridgeApplicationEvent,
    ExecuteAiInvocation,
    RequestShutdown,
}

impl ZsuiReusableRuntimeHarnessStage {
    pub(crate) const fn stage_name(self) -> &'static str {
        match self {
            Self::StartRuntime => "start_runtime",
            Self::DispatchCommand => "dispatch_command",
            Self::BridgeApplicationEvent => "bridge_application_event",
            Self::ExecuteAiInvocation => "execute_ai_invocation",
            Self::RequestShutdown => "request_shutdown",
        }
    }
}

pub(crate) const REQUIRED_ZSUI_REUSABLE_RUNTIME_HARNESS_STAGES: [ZsuiReusableRuntimeHarnessStage;
    5] = [
    ZsuiReusableRuntimeHarnessStage::StartRuntime,
    ZsuiReusableRuntimeHarnessStage::DispatchCommand,
    ZsuiReusableRuntimeHarnessStage::BridgeApplicationEvent,
    ZsuiReusableRuntimeHarnessStage::ExecuteAiInvocation,
    ZsuiReusableRuntimeHarnessStage::RequestShutdown,
];

pub(crate) fn zsui_reusable_runtime_harness_stage_names() -> Vec<&'static str> {
    REQUIRED_ZSUI_REUSABLE_RUNTIME_HARNESS_STAGES
        .iter()
        .map(|stage| stage.stage_name())
        .collect()
}
