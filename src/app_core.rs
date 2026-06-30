//! Platform-neutral contracts for the application layer.
//!
//! This module is the stable seam between product logic and platform backends.
//! It intentionally avoids Win32 handles and renderer-owned resources so UI
//! behavior can be migrated away from `app.rs` incrementally.

#![allow(dead_code)]

pub(crate) mod ai_action_protocol;
pub(crate) mod command_protocol;
pub(crate) mod component_protocol;
pub(crate) mod components;
pub(crate) mod control_protocol;
pub(crate) mod event_protocol;
pub(crate) mod framework_manifest;
pub(crate) mod host_protocol;
pub(crate) mod layout_protocol;
pub(crate) mod main_commands;
pub(crate) mod main_window;
pub(crate) mod main_window_protocol;
pub(crate) mod native_adapter_manifest;
pub(crate) mod native_component_protocol;
pub(crate) mod native_host_actions;
pub(crate) mod native_host_launch;
pub(crate) mod native_hosts;
pub(crate) mod product_adapter;
pub(crate) mod render_protocol;
pub(crate) mod runtime_protocol;
pub(crate) mod settings_protocol;
pub(crate) mod timer_protocol;
pub(crate) mod ui_surface_protocol;
pub(crate) mod zsui;

#[allow(unused_imports)]
pub(crate) use ai_action_protocol::*;
pub(crate) use command_protocol::*;
pub(crate) use component_protocol::*;
pub(crate) use control_protocol::*;
pub(crate) use event_protocol::*;
#[allow(unused_imports)]
pub(crate) use framework_manifest::*;
pub(crate) use host_protocol::*;
pub(crate) use layout_protocol::*;
pub(crate) use main_commands::*;
#[allow(unused_imports)]
pub(crate) use main_window::MainRowAiCapabilityPlan;
#[allow(unused_imports)]
pub(crate) use main_window::{
    clip_kind_filter_options_for_tab, main_copy_selection_plan, main_group_filter_menu_plan,
    main_group_filter_popup_entries, main_paste_completion_plan,
    main_paste_completion_plan_with_backspaces, main_paste_preparation_plan,
    main_row_ai_capability_plan, main_row_ai_invocation, main_row_current_item_action_plan,
    main_row_data_action_plan, main_row_delete_items_data_plan, main_row_delete_unpinned_data_plan,
    main_row_dialog_action_plan, main_row_external_action_plan, main_row_group_assignment_plan,
    main_row_group_popup_entries, main_row_menu_action_label, main_row_menu_plan,
    main_row_pin_data_plan, main_row_popup_menu_entries, main_vv_select_plan,
    parse_search_query_with_context, ClipGroup, ClipItem, ClipKind, ClipKindFilter, ClipListState,
    ItemsCursor, ItemsQuery, MainActivateSelectionPlan, MainCopySelectionPlan, MainEmptyStateKind,
    MainFontRole, MainFrameHitTarget, MainGroupFilterMenuEntry, MainHoverTarget, MainIconColorMode,
    MainIconCommand, MainIconKind, MainPaintCommand, MainPaintFill, MainPasteCompletionInput,
    MainPasteCompletionKind, MainPasteCompletionPlan, MainPastePreparationInput,
    MainPastePreparationStep, MainPointerDownStatePlan, MainPointerDownTarget,
    MainPointerModifiers, MainPointerUpTarget, MainRenderInput, MainRowAiCapabilityPresentation,
    MainRowContentInput, MainRowCurrentItemActionPlan, MainRowDataActionPlan,
    MainRowDialogActionPlan, MainRowExternalActionPlan, MainRowMenuAction, MainRowMenuEntry,
    MainRowMenuInput, MainRowMenuLabelInput, MainRowReleaseAction, MainRowTextCommand,
    MainShortcutEscapePlan, MainShortcutRowCommand, MainTextCommand, MainTextLayer, MainTextRole,
    MainThemeRole, MainUiLayout, MainVvPopupHit, MainVvPopupLayout, MainVvPopupRenderItem,
    MainVvPopupRenderPlan, MainVvPopupRenderStrings, MainVvPopupTextCommand, MainVvPopupTextRole,
    MainVvSelectPlan, SearchDateContext, SearchTimeFilter, SharedTabViewState, TabLoadState,
    TitleButtonVisibility, MAIN_EMPTY_GROUP_MENU_ID, MAIN_VV_POPUP_MAX_ITEMS,
};
pub(crate) use main_window_protocol::*;
pub(crate) use native_adapter_manifest::*;
#[allow(unused_imports)]
pub(crate) use native_component_protocol::*;
#[allow(unused_imports)]
pub(crate) use native_host_actions::*;
#[allow(unused_imports)]
pub(crate) use native_host_launch::*;
pub(crate) use native_hosts::*;
pub(crate) use product_adapter::*;
pub(crate) use render_protocol::*;
#[allow(unused_imports)]
pub(crate) use runtime_protocol::*;
pub(crate) use settings_protocol::*;
pub(crate) use timer_protocol::*;
#[allow(unused_imports)]
pub(crate) use ui_surface_protocol::*;
#[allow(unused_imports)]
pub(crate) use zsui::{
    ApiVersion, ZsuiLayer, APP_CORE_API_VERSION, ZSUI_FRAMEWORK_NAME, ZSUI_FRAMEWORK_TAGLINE,
};

#[cfg(test)]
mod tests;
