use super::command_protocol::Command;
use crate::app_core::native_host_actions::NativeHostClipListItemProjection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeWindowToken(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplicationEvent {
    LanSyncReady,
    VvShowRequested { target: NativeWindowToken },
    VvHideRequested,
    VvSelectRequested { index: usize },
    ClipboardChanged { sequence: u32 },
    ItemsPageReady,
    StartupDataReconciled { deleted: usize },
    CloudSyncReady,
    UpdateCheckReady,
    ShellIntegrationRestored,
    TrayCallback { code: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageThumbnail {
    pub(crate) bytes: Vec<u8>,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImagePasteReadyResult {
    pub(crate) image: Option<(Vec<u8>, usize, usize)>,
    pub(crate) target: NativeWindowToken,
    pub(crate) hide_main: bool,
    pub(crate) backspaces: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextOperationReadyResult {
    pub(crate) text: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageThumbReadyResult {
    pub(crate) item_id: i64,
    pub(crate) image: Option<ImageThumbnail>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainAsyncEvent {
    ImagePaste(ImagePasteReadyResult),
    ImageOcr(TextOperationReadyResult),
    TextTranslate(TextOperationReadyResult),
    ImageThumbnail(ImageThumbReadyResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAiProviderKind {
    Llms,
    Skills,
    ProductAdapter,
}

impl ProductAiProviderKind {
    pub(crate) const fn provider_name(self) -> &'static str {
        match self {
            Self::Llms => "llms",
            Self::Skills => "skills",
            Self::ProductAdapter => "product_adapter",
        }
    }

    pub(crate) const fn executor_boundary(self) -> ProductAiExecutorBoundary {
        match self {
            Self::Llms => ProductAiExecutorBoundary::LlmExecutor,
            Self::Skills => ProductAiExecutorBoundary::SkillRegistry,
            Self::ProductAdapter => ProductAiExecutorBoundary::ProductAdapterTools,
        }
    }

    pub(crate) const fn integration_task(self) -> ProductAdapterIntegrationTask {
        match self {
            Self::Llms => ProductAdapterIntegrationTask::ConnectLlmExecutor,
            Self::Skills => ProductAdapterIntegrationTask::ConnectSkillRegistry,
            Self::ProductAdapter => ProductAdapterIntegrationTask::ConnectProductAiTools,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAiExecutorBoundary {
    LlmExecutor,
    SkillRegistry,
    ProductAdapterTools,
}

impl ProductAiExecutorBoundary {
    pub(crate) const fn boundary_name(self) -> &'static str {
        match self {
            Self::LlmExecutor => "llm_executor",
            Self::SkillRegistry => "skill_registry",
            Self::ProductAdapterTools => "product_adapter_tools",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAiUiSurface {
    MainWindow,
    RowContextMenu,
    SettingsPluginPage,
    BackgroundTask,
}

impl ProductAiUiSurface {
    pub(crate) const fn surface_name(self) -> &'static str {
        match self {
            Self::MainWindow => "main_window",
            Self::RowContextMenu => "row_context_menu",
            Self::SettingsPluginPage => "settings_plugin_page",
            Self::BackgroundTask => "background_task",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAiActionKind {
    CleanText,
    TranslateText,
    OcrImage,
    SummarizeItems,
    ExplainItem,
    InvokeSkill,
    ConfigureProvider,
}

impl ProductAiActionKind {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::CleanText => "clean_text",
            Self::TranslateText => "translate_text",
            Self::OcrImage => "ocr_image",
            Self::SummarizeItems => "summarize_items",
            Self::ExplainItem => "explain_item",
            Self::InvokeSkill => "invoke_skill",
            Self::ConfigureProvider => "configure_provider",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAiContextKind {
    UserPrompt,
    SelectedText,
    SelectedImage,
    SelectedFilePath,
    ClipboardItemIds,
    SettingsProfile,
}

impl ProductAiContextKind {
    pub(crate) const fn context_name(self) -> &'static str {
        match self {
            Self::UserPrompt => "user_prompt",
            Self::SelectedText => "selected_text",
            Self::SelectedImage => "selected_image",
            Self::SelectedFilePath => "selected_file_path",
            Self::ClipboardItemIds => "clipboard_item_ids",
            Self::SettingsProfile => "settings_profile",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAiResultKind {
    Text,
    ClipboardText,
    ClipboardItems,
    ProductCommand,
    SettingsMutation,
}

impl ProductAiResultKind {
    pub(crate) const fn result_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::ClipboardText => "clipboard_text",
            Self::ClipboardItems => "clipboard_items",
            Self::ProductCommand => "product_command",
            Self::SettingsMutation => "settings_mutation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAiCapability {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) provider: ProductAiProviderKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAiCapabilityDescriptor {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) provider: ProductAiProviderKind,
    pub(crate) action: ProductAiActionKind,
    pub(crate) surface: ProductAiUiSurface,
    pub(crate) input_contexts: &'static [ProductAiContextKind],
    pub(crate) result: ProductAiResultKind,
}

impl ProductAiCapabilityDescriptor {
    pub(crate) const fn capability(&self) -> ProductAiCapability {
        ProductAiCapability {
            id: self.id,
            label: self.label,
            provider: self.provider,
        }
    }

    pub(crate) fn accepts_context(&self, context: ProductAiContextKind) -> bool {
        self.input_contexts.contains(&context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAiInvocation {
    pub(crate) capability_id: String,
    pub(crate) input_text: String,
    pub(crate) context_item_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAiExecutionPlan {
    pub(crate) invocation: ProductAiInvocation,
    pub(crate) provider: ProductAiProviderKind,
    pub(crate) executor_boundary: ProductAiExecutorBoundary,
    pub(crate) action: ProductAiActionKind,
    pub(crate) surface: ProductAiUiSurface,
    pub(crate) input_contexts: Vec<ProductAiContextKind>,
    pub(crate) result: ProductAiResultKind,
}

impl ProductAiExecutionPlan {
    pub(crate) const fn provider_name(&self) -> &'static str {
        self.provider.provider_name()
    }

    pub(crate) const fn executor_boundary_name(&self) -> &'static str {
        self.executor_boundary.boundary_name()
    }

    pub(crate) const fn executor_task_name(&self) -> &'static str {
        self.provider.integration_task().task_name()
    }

    pub(crate) const fn action_name(&self) -> &'static str {
        self.action.action_name()
    }

    pub(crate) const fn surface_name(&self) -> &'static str {
        self.surface.surface_name()
    }

    pub(crate) fn input_context_names(&self) -> Vec<&'static str> {
        self.input_contexts
            .iter()
            .map(|context| context.context_name())
            .collect()
    }

    pub(crate) const fn result_name(&self) -> &'static str {
        self.result.result_name()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAiExecutionRoute {
    pub(crate) capability_id: &'static str,
    pub(crate) provider: ProductAiProviderKind,
    pub(crate) executor_boundary: ProductAiExecutorBoundary,
    pub(crate) action: ProductAiActionKind,
    pub(crate) surface: ProductAiUiSurface,
    pub(crate) input_contexts: Vec<ProductAiContextKind>,
    pub(crate) result: ProductAiResultKind,
}

impl ProductAiExecutionRoute {
    pub(crate) const fn provider_name(&self) -> &'static str {
        self.provider.provider_name()
    }

    pub(crate) const fn executor_boundary_name(&self) -> &'static str {
        self.executor_boundary.boundary_name()
    }

    pub(crate) const fn executor_task_name(&self) -> &'static str {
        self.provider.integration_task().task_name()
    }

    pub(crate) const fn action_name(&self) -> &'static str {
        self.action.action_name()
    }

    pub(crate) const fn surface_name(&self) -> &'static str {
        self.surface.surface_name()
    }

    pub(crate) fn input_context_names(&self) -> Vec<&'static str> {
        self.input_contexts
            .iter()
            .map(|context| context.context_name())
            .collect()
    }

    pub(crate) const fn result_name(&self) -> &'static str {
        self.result.result_name()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAiIntegrationManifest {
    pub(crate) total_capabilities: usize,
    pub(crate) llm_capabilities: usize,
    pub(crate) skill_capabilities: usize,
    pub(crate) product_adapter_capabilities: usize,
    pub(crate) providers: Vec<ProductAiProviderKind>,
    pub(crate) executor_boundaries: Vec<ProductAiExecutorBoundary>,
    pub(crate) actions: Vec<ProductAiActionKind>,
    pub(crate) surfaces: Vec<ProductAiUiSurface>,
    pub(crate) contexts: Vec<ProductAiContextKind>,
    pub(crate) results: Vec<ProductAiResultKind>,
    pub(crate) execution_routes: Vec<ProductAiExecutionRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAdapterContractSurface {
    ProductIdentity,
    ProductStateModel,
    ProductCommandExecutor,
    SettingsModel,
    AsyncEventBridge,
    AiCapabilityCatalog,
}

impl ProductAdapterContractSurface {
    pub(crate) const fn surface_name(self) -> &'static str {
        match self {
            Self::ProductIdentity => "product_identity",
            Self::ProductStateModel => "product_state_model",
            Self::ProductCommandExecutor => "product_command_executor",
            Self::SettingsModel => "settings_model",
            Self::AsyncEventBridge => "async_event_bridge",
            Self::AiCapabilityCatalog => "ai_capability_catalog",
        }
    }
}

pub(crate) const REQUIRED_PRODUCT_ADAPTER_CONTRACT_SURFACES: [ProductAdapterContractSurface; 6] = [
    ProductAdapterContractSurface::ProductIdentity,
    ProductAdapterContractSurface::ProductStateModel,
    ProductAdapterContractSurface::ProductCommandExecutor,
    ProductAdapterContractSurface::SettingsModel,
    ProductAdapterContractSurface::AsyncEventBridge,
    ProductAdapterContractSurface::AiCapabilityCatalog,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAdapterIntegrationTask {
    ProvideProductIdentity,
    ProjectProductState,
    ExecuteProductCommands,
    BindSettingsModel,
    BridgeAsyncEvents,
    PublishAiCatalog,
    ConnectLlmExecutor,
    ConnectSkillRegistry,
    ConnectProductAiTools,
}

impl ProductAdapterIntegrationTask {
    pub(crate) const fn task_name(self) -> &'static str {
        match self {
            Self::ProvideProductIdentity => "provide_product_identity",
            Self::ProjectProductState => "project_product_state",
            Self::ExecuteProductCommands => "execute_product_commands",
            Self::BindSettingsModel => "bind_settings_model",
            Self::BridgeAsyncEvents => "bridge_async_events",
            Self::PublishAiCatalog => "publish_ai_catalog",
            Self::ConnectLlmExecutor => "connect_llm_executor",
            Self::ConnectSkillRegistry => "connect_skill_registry",
            Self::ConnectProductAiTools => "connect_product_ai_tools",
        }
    }
}

pub(crate) const REQUIRED_PRODUCT_ADAPTER_INTEGRATION_TASKS: [ProductAdapterIntegrationTask; 9] = [
    ProductAdapterIntegrationTask::ProvideProductIdentity,
    ProductAdapterIntegrationTask::ProjectProductState,
    ProductAdapterIntegrationTask::ExecuteProductCommands,
    ProductAdapterIntegrationTask::BindSettingsModel,
    ProductAdapterIntegrationTask::BridgeAsyncEvents,
    ProductAdapterIntegrationTask::PublishAiCatalog,
    ProductAdapterIntegrationTask::ConnectLlmExecutor,
    ProductAdapterIntegrationTask::ConnectSkillRegistry,
    ProductAdapterIntegrationTask::ConnectProductAiTools,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterIntegrationContract {
    pub(crate) required_surfaces: Vec<ProductAdapterContractSurface>,
    pub(crate) ai: ProductAiIntegrationManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterReuseChecklist {
    pub(crate) surface_names: Vec<&'static str>,
    pub(crate) task_names: Vec<&'static str>,
    pub(crate) ai_provider_names: Vec<&'static str>,
    pub(crate) ai_executor_boundary_names: Vec<&'static str>,
    pub(crate) ai_route_ids: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAdapterFunctionFlowKind {
    AppBootstrap,
    StateProjection,
    UserCommand,
    SettingsSync,
    AsyncEvent,
    AiAction,
}

impl ProductAdapterFunctionFlowKind {
    pub(crate) const fn flow_name(self) -> &'static str {
        match self {
            Self::AppBootstrap => "app_bootstrap",
            Self::StateProjection => "state_projection",
            Self::UserCommand => "user_command",
            Self::SettingsSync => "settings_sync",
            Self::AsyncEvent => "async_event",
            Self::AiAction => "ai_action",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterFunctionFlow {
    pub(crate) flow_name: &'static str,
    pub(crate) required_surface_names: Vec<&'static str>,
    pub(crate) required_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAdapterPipelineStageKind {
    ReceiveUiIntent,
    ProjectProductState,
    ExecuteProductCommand,
    BridgeAsyncEvent,
    RouteAiAction,
    ProjectUiUpdate,
}

impl ProductAdapterPipelineStageKind {
    pub(crate) const fn stage_name(self) -> &'static str {
        match self {
            Self::ReceiveUiIntent => "receive_ui_intent",
            Self::ProjectProductState => "project_product_state",
            Self::ExecuteProductCommand => "execute_product_command",
            Self::BridgeAsyncEvent => "bridge_async_event",
            Self::RouteAiAction => "route_ai_action",
            Self::ProjectUiUpdate => "project_ui_update",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterPipelineStage {
    pub(crate) stage_name: &'static str,
    pub(crate) required_surface_names: Vec<&'static str>,
    pub(crate) required_task_names: Vec<&'static str>,
    pub(crate) required_ai_executor_boundary_names: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductAdapterHostMethod {
    ProductIdentity,
    ProjectProductState,
    ExecuteProductCommand,
    BindSettingsModel,
    BridgeAsyncEvent,
    PublishAiCatalog,
    ExecuteAiPlan,
}

impl ProductAdapterHostMethod {
    pub(crate) const fn method_name(self) -> &'static str {
        match self {
            Self::ProductIdentity => "product_identity",
            Self::ProjectProductState => "project_product_state",
            Self::ExecuteProductCommand => "execute_product_command",
            Self::BindSettingsModel => "bind_settings_model",
            Self::BridgeAsyncEvent => "bridge_async_event",
            Self::PublishAiCatalog => "publish_ai_catalog",
            Self::ExecuteAiPlan => "execute_ai_plan",
        }
    }
}

pub(crate) const REQUIRED_PRODUCT_ADAPTER_HOST_METHODS: [ProductAdapterHostMethod; 7] = [
    ProductAdapterHostMethod::ProductIdentity,
    ProductAdapterHostMethod::ProjectProductState,
    ProductAdapterHostMethod::ExecuteProductCommand,
    ProductAdapterHostMethod::BindSettingsModel,
    ProductAdapterHostMethod::BridgeAsyncEvent,
    ProductAdapterHostMethod::PublishAiCatalog,
    ProductAdapterHostMethod::ExecuteAiPlan,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterIdentity {
    pub(crate) product_id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterProjectedState {
    pub(crate) state_name: String,
    pub(crate) revision: u64,
    pub(crate) native_clip_items: Vec<NativeHostClipListItemProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterSettingsSnapshot {
    pub(crate) profile_name: String,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterCommandResult {
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductAdapterAsyncBridgeResult {
    pub(crate) bridged: bool,
    pub(crate) event_name: String,
}

pub(crate) trait ProductAdapterHost {
    fn product_identity(&self) -> ProductAdapterIdentity;
    fn project_product_state(&self) -> ProductAdapterProjectedState;
    fn execute_product_command(&mut self, command: Command) -> ProductAdapterCommandResult;
    fn bind_settings_model(&mut self, settings: ProductAdapterSettingsSnapshot);
    fn bridge_async_event(&mut self, event: ApplicationEvent) -> ProductAdapterAsyncBridgeResult;
    fn publish_ai_catalog(&self) -> Vec<ProductAiCapability>;
    fn execute_ai_plan(&mut self, plan: ProductAiExecutionPlan) -> ProductAdapterCommandResult;
}

const AI_TEXT_SELECTION_CONTEXTS: &[ProductAiContextKind] = &[
    ProductAiContextKind::UserPrompt,
    ProductAiContextKind::SelectedText,
    ProductAiContextKind::ClipboardItemIds,
];

const AI_IMAGE_SELECTION_CONTEXTS: &[ProductAiContextKind] = &[
    ProductAiContextKind::UserPrompt,
    ProductAiContextKind::SelectedImage,
    ProductAiContextKind::SelectedFilePath,
    ProductAiContextKind::ClipboardItemIds,
];

const AI_FILE_SELECTION_CONTEXTS: &[ProductAiContextKind] = &[
    ProductAiContextKind::UserPrompt,
    ProductAiContextKind::SelectedFilePath,
    ProductAiContextKind::ClipboardItemIds,
];

const AI_SETTINGS_CONTEXTS: &[ProductAiContextKind] = &[
    ProductAiContextKind::UserPrompt,
    ProductAiContextKind::SettingsProfile,
];

pub(crate) const PRODUCT_AI_CAPABILITY_CATALOG: [ProductAiCapabilityDescriptor; 7] = [
    ProductAiCapabilityDescriptor {
        id: "clipboard.clean",
        label: "Clean text",
        provider: ProductAiProviderKind::Llms,
        action: ProductAiActionKind::CleanText,
        surface: ProductAiUiSurface::RowContextMenu,
        input_contexts: AI_TEXT_SELECTION_CONTEXTS,
        result: ProductAiResultKind::ClipboardText,
    },
    ProductAiCapabilityDescriptor {
        id: "clipboard.summarize",
        label: "Summarize selected items",
        provider: ProductAiProviderKind::Llms,
        action: ProductAiActionKind::SummarizeItems,
        surface: ProductAiUiSurface::MainWindow,
        input_contexts: AI_TEXT_SELECTION_CONTEXTS,
        result: ProductAiResultKind::Text,
    },
    ProductAiCapabilityDescriptor {
        id: "clipboard.explain",
        label: "Explain selected item",
        provider: ProductAiProviderKind::Llms,
        action: ProductAiActionKind::ExplainItem,
        surface: ProductAiUiSurface::RowContextMenu,
        input_contexts: AI_TEXT_SELECTION_CONTEXTS,
        result: ProductAiResultKind::Text,
    },
    ProductAiCapabilityDescriptor {
        id: "clipboard.skill.translate",
        label: "Translate with skill",
        provider: ProductAiProviderKind::Skills,
        action: ProductAiActionKind::TranslateText,
        surface: ProductAiUiSurface::RowContextMenu,
        input_contexts: AI_TEXT_SELECTION_CONTEXTS,
        result: ProductAiResultKind::ClipboardText,
    },
    ProductAiCapabilityDescriptor {
        id: "clipboard.skill.run",
        label: "Run selected skill",
        provider: ProductAiProviderKind::Skills,
        action: ProductAiActionKind::InvokeSkill,
        surface: ProductAiUiSurface::MainWindow,
        input_contexts: AI_FILE_SELECTION_CONTEXTS,
        result: ProductAiResultKind::ProductCommand,
    },
    ProductAiCapabilityDescriptor {
        id: "clipboard.product.ocr",
        label: "OCR adapter",
        provider: ProductAiProviderKind::ProductAdapter,
        action: ProductAiActionKind::OcrImage,
        surface: ProductAiUiSurface::RowContextMenu,
        input_contexts: AI_IMAGE_SELECTION_CONTEXTS,
        result: ProductAiResultKind::ClipboardText,
    },
    ProductAiCapabilityDescriptor {
        id: "clipboard.product.configure_ai",
        label: "Configure AI providers",
        provider: ProductAiProviderKind::ProductAdapter,
        action: ProductAiActionKind::ConfigureProvider,
        surface: ProductAiUiSurface::SettingsPluginPage,
        input_contexts: AI_SETTINGS_CONTEXTS,
        result: ProductAiResultKind::SettingsMutation,
    },
];

pub(crate) fn product_ai_capability_catalog() -> &'static [ProductAiCapabilityDescriptor] {
    &PRODUCT_AI_CAPABILITY_CATALOG
}

pub(crate) fn product_adapter_integration_contract() -> ProductAdapterIntegrationContract {
    ProductAdapterIntegrationContract {
        required_surfaces: REQUIRED_PRODUCT_ADAPTER_CONTRACT_SURFACES.to_vec(),
        ai: product_ai_integration_manifest(),
    }
}

pub(crate) fn product_adapter_reuse_checklist() -> ProductAdapterReuseChecklist {
    let contract = product_adapter_integration_contract();
    ProductAdapterReuseChecklist {
        surface_names: contract
            .required_surfaces
            .iter()
            .map(|surface| surface.surface_name())
            .collect(),
        task_names: REQUIRED_PRODUCT_ADAPTER_INTEGRATION_TASKS
            .iter()
            .map(|task| task.task_name())
            .collect(),
        ai_provider_names: contract
            .ai
            .providers
            .iter()
            .map(|provider| provider.provider_name())
            .collect(),
        ai_executor_boundary_names: contract
            .ai
            .executor_boundaries
            .iter()
            .map(|boundary| boundary.boundary_name())
            .collect(),
        ai_route_ids: contract
            .ai
            .execution_routes
            .iter()
            .map(|route| route.capability_id)
            .collect(),
    }
}

pub(crate) fn product_adapter_function_flows() -> Vec<ProductAdapterFunctionFlow> {
    use ProductAdapterContractSurface::{
        AiCapabilityCatalog, AsyncEventBridge, ProductCommandExecutor, ProductIdentity,
        ProductStateModel, SettingsModel,
    };
    use ProductAdapterFunctionFlowKind::{
        AiAction, AppBootstrap, AsyncEvent, SettingsSync, StateProjection, UserCommand,
    };
    use ProductAdapterIntegrationTask::{
        BindSettingsModel, BridgeAsyncEvents, ConnectLlmExecutor, ConnectProductAiTools,
        ConnectSkillRegistry, ExecuteProductCommands, ProjectProductState, ProvideProductIdentity,
        PublishAiCatalog,
    };
    use ProductAiExecutorBoundary::{LlmExecutor, ProductAdapterTools, SkillRegistry};

    vec![
        product_adapter_function_flow(
            AppBootstrap,
            &[ProductIdentity],
            &[ProvideProductIdentity],
            &[],
        ),
        product_adapter_function_flow(
            StateProjection,
            &[ProductStateModel],
            &[ProjectProductState],
            &[],
        ),
        product_adapter_function_flow(
            UserCommand,
            &[ProductCommandExecutor],
            &[ExecuteProductCommands],
            &[],
        ),
        product_adapter_function_flow(SettingsSync, &[SettingsModel], &[BindSettingsModel], &[]),
        product_adapter_function_flow(AsyncEvent, &[AsyncEventBridge], &[BridgeAsyncEvents], &[]),
        product_adapter_function_flow(
            AiAction,
            &[AiCapabilityCatalog, ProductCommandExecutor],
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

fn product_adapter_function_flow(
    flow: ProductAdapterFunctionFlowKind,
    surfaces: &[ProductAdapterContractSurface],
    tasks: &[ProductAdapterIntegrationTask],
    ai_executor_boundaries: &[ProductAiExecutorBoundary],
) -> ProductAdapterFunctionFlow {
    ProductAdapterFunctionFlow {
        flow_name: flow.flow_name(),
        required_surface_names: surfaces
            .iter()
            .map(|surface| surface.surface_name())
            .collect(),
        required_task_names: tasks.iter().map(|task| task.task_name()).collect(),
        required_ai_executor_boundary_names: ai_executor_boundaries
            .iter()
            .map(|boundary| boundary.boundary_name())
            .collect(),
    }
}

pub(crate) fn product_adapter_execution_pipeline() -> Vec<ProductAdapterPipelineStage> {
    use ProductAdapterContractSurface::{
        AiCapabilityCatalog, AsyncEventBridge, ProductCommandExecutor, ProductStateModel,
    };
    use ProductAdapterIntegrationTask::{
        BridgeAsyncEvents, ConnectLlmExecutor, ConnectProductAiTools, ConnectSkillRegistry,
        ExecuteProductCommands, ProjectProductState, PublishAiCatalog,
    };
    use ProductAdapterPipelineStageKind::{
        BridgeAsyncEvent, ExecuteProductCommand, ProjectProductState as ProjectStateStage,
        ProjectUiUpdate, ReceiveUiIntent, RouteAiAction,
    };
    use ProductAiExecutorBoundary::{LlmExecutor, ProductAdapterTools, SkillRegistry};

    vec![
        product_adapter_pipeline_stage(ReceiveUiIntent, &[], &[], &[]),
        product_adapter_pipeline_stage(
            ProjectStateStage,
            &[ProductStateModel],
            &[ProjectProductState],
            &[],
        ),
        product_adapter_pipeline_stage(
            ExecuteProductCommand,
            &[ProductCommandExecutor],
            &[ExecuteProductCommands],
            &[],
        ),
        product_adapter_pipeline_stage(
            BridgeAsyncEvent,
            &[AsyncEventBridge],
            &[BridgeAsyncEvents],
            &[],
        ),
        product_adapter_pipeline_stage(
            RouteAiAction,
            &[AiCapabilityCatalog, ProductCommandExecutor],
            &[
                PublishAiCatalog,
                ConnectLlmExecutor,
                ConnectSkillRegistry,
                ConnectProductAiTools,
            ],
            &[LlmExecutor, SkillRegistry, ProductAdapterTools],
        ),
        product_adapter_pipeline_stage(
            ProjectUiUpdate,
            &[ProductStateModel],
            &[ProjectProductState],
            &[],
        ),
    ]
}

pub(crate) fn required_product_adapter_host_method_names() -> Vec<&'static str> {
    REQUIRED_PRODUCT_ADAPTER_HOST_METHODS
        .iter()
        .map(|method| method.method_name())
        .collect()
}

fn product_adapter_pipeline_stage(
    stage: ProductAdapterPipelineStageKind,
    surfaces: &[ProductAdapterContractSurface],
    tasks: &[ProductAdapterIntegrationTask],
    ai_executor_boundaries: &[ProductAiExecutorBoundary],
) -> ProductAdapterPipelineStage {
    ProductAdapterPipelineStage {
        stage_name: stage.stage_name(),
        required_surface_names: surfaces
            .iter()
            .map(|surface| surface.surface_name())
            .collect(),
        required_task_names: tasks.iter().map(|task| task.task_name()).collect(),
        required_ai_executor_boundary_names: ai_executor_boundaries
            .iter()
            .map(|boundary| boundary.boundary_name())
            .collect(),
    }
}

pub(crate) fn product_ai_integration_manifest() -> ProductAiIntegrationManifest {
    let mut manifest = ProductAiIntegrationManifest {
        total_capabilities: PRODUCT_AI_CAPABILITY_CATALOG.len(),
        llm_capabilities: 0,
        skill_capabilities: 0,
        product_adapter_capabilities: 0,
        providers: Vec::new(),
        executor_boundaries: Vec::new(),
        actions: Vec::new(),
        surfaces: Vec::new(),
        contexts: Vec::new(),
        results: Vec::new(),
        execution_routes: Vec::new(),
    };

    for capability in PRODUCT_AI_CAPABILITY_CATALOG {
        match capability.provider {
            ProductAiProviderKind::Llms => manifest.llm_capabilities += 1,
            ProductAiProviderKind::Skills => manifest.skill_capabilities += 1,
            ProductAiProviderKind::ProductAdapter => manifest.product_adapter_capabilities += 1,
        }
        push_unique(&mut manifest.providers, capability.provider);
        push_unique(
            &mut manifest.executor_boundaries,
            capability.provider.executor_boundary(),
        );
        push_unique(&mut manifest.actions, capability.action);
        push_unique(&mut manifest.surfaces, capability.surface);
        for context in capability.input_contexts {
            push_unique(&mut manifest.contexts, *context);
        }
        push_unique(&mut manifest.results, capability.result);
        manifest.execution_routes.push(ProductAiExecutionRoute {
            capability_id: capability.id,
            provider: capability.provider,
            executor_boundary: capability.provider.executor_boundary(),
            action: capability.action,
            surface: capability.surface,
            input_contexts: capability.input_contexts.to_vec(),
            result: capability.result,
        });
    }

    manifest
}

pub(crate) fn product_ai_capabilities_for_surface(
    surface: ProductAiUiSurface,
) -> Vec<&'static ProductAiCapabilityDescriptor> {
    PRODUCT_AI_CAPABILITY_CATALOG
        .iter()
        .filter(|capability| capability.surface == surface)
        .collect()
}

pub(crate) fn product_ai_capabilities_for_context(
    surface: ProductAiUiSurface,
    context: ProductAiContextKind,
) -> Vec<&'static ProductAiCapabilityDescriptor> {
    PRODUCT_AI_CAPABILITY_CATALOG
        .iter()
        .filter(|capability| capability.surface == surface && capability.accepts_context(context))
        .collect()
}

pub(crate) fn product_ai_capability_for_action(
    surface: ProductAiUiSurface,
    action: ProductAiActionKind,
) -> Option<&'static ProductAiCapabilityDescriptor> {
    PRODUCT_AI_CAPABILITY_CATALOG
        .iter()
        .find(|capability| capability.surface == surface && capability.action == action)
}

pub(crate) fn product_ai_capability_descriptor(
    id: &str,
) -> Option<&'static ProductAiCapabilityDescriptor> {
    PRODUCT_AI_CAPABILITY_CATALOG
        .iter()
        .find(|capability| capability.id == id)
}

pub(crate) fn product_ai_execution_plan(
    invocation: ProductAiInvocation,
) -> Option<ProductAiExecutionPlan> {
    let descriptor = product_ai_capability_descriptor(&invocation.capability_id)?;
    Some(ProductAiExecutionPlan {
        invocation,
        provider: descriptor.provider,
        executor_boundary: descriptor.provider.executor_boundary(),
        action: descriptor.action,
        surface: descriptor.surface,
        input_contexts: descriptor.input_contexts.to_vec(),
        result: descriptor.result,
    })
}

fn push_unique<T: Copy + Eq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}
