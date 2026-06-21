use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    host_protocol::NativePopupMenuEntry,
    main_commands::{
        main_group_filter_menu_all_id, main_group_filter_menu_group_id,
        main_row_group_menu_group_id, main_row_menu_action_id,
    },
    product_adapter::{
        product_ai_capabilities_for_context, ProductAiActionKind, ProductAiContextKind,
        ProductAiInvocation, ProductAiProviderKind, ProductAiResultKind, ProductAiUiSurface,
    },
    HorizontalAlign, TextWrap, UiRect, VerticalAlign,
};

fn gregorian_to_days(y: i32, m: i32, d: i32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let a = (14 - m) / 12;
    let yy = y + 4800 - a;
    let mm = m + 12 * a - 3;
    let jd = d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045;
    jd - 2440588
}

fn days_to_gregorian(days: i64) -> (i32, i32, i32) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i32;
    let y = if m <= 2 { y + 1 } else { y } as i32;
    (y, m, d)
}

#[derive(Clone, Debug)]
pub(crate) struct ClipGroup {
    pub(crate) id: i64,
    pub(crate) category: i64,
    pub(crate) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipKind {
    Text,
    Image,
    Phrase,
    Files,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainRowMenuAction {
    Copy,
    Pin,
    ToPhrase,
    AddToGroup,
    RemoveFromGroup,
    Delete,
    DeleteUnpinned,
    Sticker,
    SaveImage,
    ImageOcr,
    ExportFile,
    OpenPath,
    OpenFolder,
    CopyPath,
    QrImage,
    MailMerge,
    LanPush,
    Edit,
    QuickSearch,
    TextTranslate,
}

impl MainRowMenuAction {
    pub(crate) const fn ai_action_kind(self) -> Option<ProductAiActionKind> {
        match self {
            Self::ImageOcr => Some(ProductAiActionKind::OcrImage),
            Self::TextTranslate => Some(ProductAiActionKind::TranslateText),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainRowMenuEntry {
    Action {
        action: MainRowMenuAction,
        enabled: bool,
    },
    Separator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainRowMenuPlan {
    pub(crate) entries: Vec<MainRowMenuEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainRowAiCapabilityPresentation {
    pub(crate) capability_id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) provider: ProductAiProviderKind,
    pub(crate) action: ProductAiActionKind,
    pub(crate) result: ProductAiResultKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainRowAiCapabilityPlan {
    pub(crate) contexts: Vec<ProductAiContextKind>,
    pub(crate) target_item_ids: Vec<i64>,
    pub(crate) capabilities: Vec<MainRowAiCapabilityPresentation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowMenuInput {
    pub(crate) selected_count: usize,
    pub(crate) has_unpinned: bool,
    pub(crate) current_kind: ClipKind,
    pub(crate) grouping_enabled: bool,
    pub(crate) current_can_ocr: bool,
    pub(crate) current_can_translate: bool,
    pub(crate) current_is_excel: bool,
    pub(crate) quick_search_enabled: bool,
    pub(crate) qr_quick_enabled: bool,
    pub(crate) super_mail_merge_enabled: bool,
    pub(crate) lan_push_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowMenuLabelInput {
    pub(crate) selected_count: usize,
    pub(crate) has_unpinned: bool,
    pub(crate) current_is_dir: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainGroupFilterMenuEntry {
    All {
        checked: bool,
    },
    Separator,
    Group {
        index: usize,
        group_id: i64,
        label: String,
        checked: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainGroupFilterMenuPlan {
    pub(crate) entries: Vec<MainGroupFilterMenuEntry>,
}

pub(crate) fn main_group_filter_menu_plan(
    current_group_id: i64,
    groups: &[ClipGroup],
) -> MainGroupFilterMenuPlan {
    let mut entries = vec![MainGroupFilterMenuEntry::All {
        checked: current_group_id == 0,
    }];
    if !groups.is_empty() {
        entries.push(MainGroupFilterMenuEntry::Separator);
        entries.extend(groups.iter().enumerate().map(|(index, group)| {
            MainGroupFilterMenuEntry::Group {
                index,
                group_id: group.id,
                label: group.name.clone(),
                checked: current_group_id == group.id,
            }
        }));
    }
    MainGroupFilterMenuPlan { entries }
}

pub(crate) const MAIN_EMPTY_GROUP_MENU_ID: usize = 0xFFFF;

pub(crate) fn main_row_group_popup_entries(
    groups: &[ClipGroup],
    empty_label: impl Into<String>,
) -> Vec<NativePopupMenuEntry> {
    if groups.is_empty() {
        return vec![NativePopupMenuEntry::Command {
            id: MAIN_EMPTY_GROUP_MENU_ID,
            label: empty_label.into(),
            enabled: false,
            checked: false,
        }];
    }

    groups
        .iter()
        .enumerate()
        .map(|(index, group)| NativePopupMenuEntry::Command {
            id: main_row_group_menu_group_id(index),
            label: group.name.clone(),
            enabled: true,
            checked: false,
        })
        .collect()
}

pub(crate) fn main_row_menu_plan(input: MainRowMenuInput) -> MainRowMenuPlan {
    let mut entries = Vec::new();

    fn push_action(entries: &mut Vec<MainRowMenuEntry>, action: MainRowMenuAction, enabled: bool) {
        entries.push(MainRowMenuEntry::Action { action, enabled });
    }

    fn push_optional_action(
        entries: &mut Vec<MainRowMenuEntry>,
        action: MainRowMenuAction,
        enabled: bool,
    ) {
        if enabled {
            push_action(entries, action, true);
        }
    }

    fn push_common_mutation_entries(
        entries: &mut Vec<MainRowMenuEntry>,
        grouping_enabled: bool,
        has_unpinned: bool,
    ) {
        push_action(entries, MainRowMenuAction::Pin, true);
        if grouping_enabled {
            push_action(entries, MainRowMenuAction::AddToGroup, true);
        }
        push_action(entries, MainRowMenuAction::RemoveFromGroup, true);
        push_action(entries, MainRowMenuAction::Delete, true);
        push_action(entries, MainRowMenuAction::DeleteUnpinned, has_unpinned);
    }

    if input.selected_count > 1 {
        push_action(&mut entries, MainRowMenuAction::Copy, true);
        entries.push(MainRowMenuEntry::Separator);
        push_action(&mut entries, MainRowMenuAction::Pin, true);
        push_action(&mut entries, MainRowMenuAction::ToPhrase, true);
        push_optional_action(
            &mut entries,
            MainRowMenuAction::AddToGroup,
            input.grouping_enabled,
        );
        push_action(&mut entries, MainRowMenuAction::RemoveFromGroup, true);
        push_action(&mut entries, MainRowMenuAction::Delete, true);
        push_action(
            &mut entries,
            MainRowMenuAction::DeleteUnpinned,
            input.has_unpinned,
        );
        return MainRowMenuPlan { entries };
    }

    match input.current_kind {
        ClipKind::Image => {
            push_action(&mut entries, MainRowMenuAction::Sticker, true);
            push_action(&mut entries, MainRowMenuAction::SaveImage, true);
            push_optional_action(
                &mut entries,
                MainRowMenuAction::ImageOcr,
                input.current_can_ocr,
            );
            push_action(&mut entries, MainRowMenuAction::ExportFile, true);
        }
        ClipKind::Files => {
            push_action(&mut entries, MainRowMenuAction::OpenPath, true);
            push_action(&mut entries, MainRowMenuAction::OpenFolder, true);
            push_action(&mut entries, MainRowMenuAction::CopyPath, true);
            push_optional_action(
                &mut entries,
                MainRowMenuAction::ImageOcr,
                input.current_can_ocr,
            );
            push_optional_action(
                &mut entries,
                MainRowMenuAction::QrImage,
                input.qr_quick_enabled,
            );
            push_optional_action(
                &mut entries,
                MainRowMenuAction::MailMerge,
                input.current_is_excel && input.super_mail_merge_enabled,
            );
            push_optional_action(
                &mut entries,
                MainRowMenuAction::LanPush,
                input.lan_push_available,
            );
        }
        ClipKind::Text | ClipKind::Phrase => {
            push_action(&mut entries, MainRowMenuAction::Edit, true);
            push_optional_action(
                &mut entries,
                MainRowMenuAction::QuickSearch,
                input.quick_search_enabled,
            );
            push_optional_action(
                &mut entries,
                MainRowMenuAction::TextTranslate,
                input.current_can_translate,
            );
            push_optional_action(
                &mut entries,
                MainRowMenuAction::QrImage,
                input.qr_quick_enabled,
            );
            push_action(&mut entries, MainRowMenuAction::ExportFile, true);
        }
    }

    entries.push(MainRowMenuEntry::Separator);
    push_common_mutation_entries(&mut entries, input.grouping_enabled, input.has_unpinned);
    if matches!(input.current_kind, ClipKind::Text | ClipKind::Phrase) {
        let insert_at = entries
            .iter()
            .position(|entry| {
                matches!(
                    entry,
                    MainRowMenuEntry::Action {
                        action: MainRowMenuAction::AddToGroup | MainRowMenuAction::RemoveFromGroup,
                        ..
                    }
                )
            })
            .unwrap_or(entries.len());
        entries.insert(
            insert_at,
            MainRowMenuEntry::Action {
                action: MainRowMenuAction::ToPhrase,
                enabled: true,
            },
        );
    }

    MainRowMenuPlan { entries }
}

pub(crate) fn main_row_menu_action_label(
    action: MainRowMenuAction,
    input: MainRowMenuLabelInput,
) -> &'static str {
    match action {
        MainRowMenuAction::Copy => "合并复制",
        MainRowMenuAction::Pin if input.selected_count > 1 && input.has_unpinned => "置顶所选",
        MainRowMenuAction::Pin if input.has_unpinned => "置顶",
        MainRowMenuAction::Pin => "取消置顶",
        MainRowMenuAction::ToPhrase => "添加到短语",
        MainRowMenuAction::AddToGroup => "添加到分组",
        MainRowMenuAction::RemoveFromGroup => "移出分组",
        MainRowMenuAction::Delete if input.selected_count > 1 => "删除所选",
        MainRowMenuAction::Delete => "删除",
        MainRowMenuAction::DeleteUnpinned => "删除除置顶以外",
        MainRowMenuAction::Sticker => "贴图",
        MainRowMenuAction::SaveImage => "另存为 PNG",
        MainRowMenuAction::ImageOcr => "图片转文字",
        MainRowMenuAction::ExportFile => "导出为文件",
        MainRowMenuAction::OpenPath if input.current_is_dir => "打开文件夹",
        MainRowMenuAction::OpenPath => "打开文件",
        MainRowMenuAction::OpenFolder => "打开所在文件夹",
        MainRowMenuAction::CopyPath => "复制路径",
        MainRowMenuAction::QrImage => "快捷转换二维码",
        MainRowMenuAction::MailMerge => "超级邮件合并",
        MainRowMenuAction::LanPush => "推送到局域网设备",
        MainRowMenuAction::Edit => "编辑",
        MainRowMenuAction::QuickSearch => "快速搜索",
        MainRowMenuAction::TextTranslate => "文本翻译",
    }
}

pub(crate) fn main_row_popup_menu_entries(
    plan: &MainRowMenuPlan,
    label_input: MainRowMenuLabelInput,
    grouping_enabled: bool,
    group_entries: Vec<NativePopupMenuEntry>,
    label_mapper: impl Fn(&str) -> String,
) -> Vec<NativePopupMenuEntry> {
    let mut entries = Vec::new();
    for entry in &plan.entries {
        match entry {
            MainRowMenuEntry::Separator => entries.push(NativePopupMenuEntry::Separator),
            MainRowMenuEntry::Action {
                action: MainRowMenuAction::AddToGroup,
                enabled,
            } if grouping_enabled => entries.push(NativePopupMenuEntry::Submenu {
                label: label_mapper(main_row_menu_action_label(
                    MainRowMenuAction::AddToGroup,
                    label_input,
                )),
                enabled: *enabled,
                entries: group_entries.clone(),
            }),
            MainRowMenuEntry::Action { action, enabled } => {
                entries.push(NativePopupMenuEntry::Command {
                    id: main_row_menu_action_id(*action),
                    label: label_mapper(main_row_menu_action_label(*action, label_input)),
                    enabled: *enabled,
                    checked: false,
                });
            }
        }
    }
    entries
}

pub(crate) fn main_group_filter_popup_entries(
    plan: &MainGroupFilterMenuPlan,
    all_label: impl Into<String>,
) -> Vec<NativePopupMenuEntry> {
    let all_label = all_label.into();
    plan.entries
        .iter()
        .map(|entry| match entry {
            MainGroupFilterMenuEntry::All { checked } => NativePopupMenuEntry::Command {
                id: main_group_filter_menu_all_id(),
                label: all_label.clone(),
                enabled: true,
                checked: *checked,
            },
            MainGroupFilterMenuEntry::Separator => NativePopupMenuEntry::Separator,
            MainGroupFilterMenuEntry::Group {
                index,
                label,
                checked,
                ..
            } => NativePopupMenuEntry::Command {
                id: main_group_filter_menu_group_id(*index),
                label: label.clone(),
                enabled: true,
                checked: *checked,
            },
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainRowExternalActionPlan {
    OpenPaths(Vec<String>),
    OpenParentFolders(Vec<String>),
    CopyText(String),
    LanPushFiles(Vec<String>),
    QuickSearch(String),
    TextTranslate(String),
    QrText(String),
}

pub(crate) fn main_row_external_action_plan(
    action: MainRowMenuAction,
    current: Option<&ClipItem>,
    selected: &[ClipItem],
) -> Option<MainRowExternalActionPlan> {
    fn selected_or_current<'a>(
        current: Option<&'a ClipItem>,
        selected: &'a [ClipItem],
    ) -> Vec<&'a ClipItem> {
        if selected.is_empty() {
            current.into_iter().collect()
        } else {
            selected.iter().collect()
        }
    }

    fn file_paths_from_items<'a>(items: impl IntoIterator<Item = &'a ClipItem>) -> Vec<String> {
        items
            .into_iter()
            .filter_map(|item| item.file_paths.as_ref())
            .flat_map(|paths| paths.iter().cloned())
            .collect()
    }

    match action {
        MainRowMenuAction::OpenPath => {
            let paths = current
                .and_then(|item| item.file_paths.clone())
                .unwrap_or_default();
            (!paths.is_empty()).then_some(MainRowExternalActionPlan::OpenPaths(paths))
        }
        MainRowMenuAction::OpenFolder => {
            let paths = current
                .and_then(|item| item.file_paths.clone())
                .unwrap_or_default();
            (!paths.is_empty()).then_some(MainRowExternalActionPlan::OpenParentFolders(paths))
        }
        MainRowMenuAction::CopyPath => {
            let paths = file_paths_from_items(selected_or_current(current, selected));
            (!paths.is_empty()).then_some(MainRowExternalActionPlan::CopyText(paths.join("\n")))
        }
        MainRowMenuAction::LanPush => {
            let paths = file_paths_from_items(selected_or_current(current, selected));
            Some(MainRowExternalActionPlan::LanPushFiles(paths))
        }
        MainRowMenuAction::QuickSearch => current.map(|item| {
            let text = match item.kind {
                ClipKind::Text | ClipKind::Phrase => {
                    item.text.clone().unwrap_or_else(|| item.preview.clone())
                }
                ClipKind::Files => item
                    .file_paths
                    .as_ref()
                    .map(|paths| paths.join(" "))
                    .unwrap_or_else(|| item.preview.clone()),
                ClipKind::Image => item.preview.clone(),
            };
            MainRowExternalActionPlan::QuickSearch(text)
        }),
        MainRowMenuAction::TextTranslate => current.and_then(|item| {
            match item.kind {
                ClipKind::Text | ClipKind::Phrase => item
                    .text
                    .as_ref()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        let s = item.preview.trim();
                        if s.is_empty() {
                            None
                        } else {
                            Some(s.to_string())
                        }
                    }),
                _ => None,
            }
            .map(MainRowExternalActionPlan::TextTranslate)
        }),
        MainRowMenuAction::QrImage => current.and_then(|item| {
            let text = match item.kind {
                ClipKind::Text | ClipKind::Phrase => item
                    .text
                    .as_ref()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                ClipKind::Files => item
                    .file_paths
                    .as_ref()
                    .map(|paths| {
                        paths
                            .iter()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .filter(|s| !s.is_empty()),
                _ => None,
            };
            text.map(MainRowExternalActionPlan::QrText)
        }),
        _ => None,
    }
}

pub(crate) fn main_row_ai_capability_plan(
    current: Option<&ClipItem>,
    selected: &[ClipItem],
) -> Option<MainRowAiCapabilityPlan> {
    let targets = if selected.is_empty() {
        current.into_iter().collect::<Vec<_>>()
    } else {
        selected.iter().collect::<Vec<_>>()
    };
    if targets.is_empty() {
        return None;
    }

    let mut contexts = Vec::new();
    for item in &targets {
        let context = match item.kind {
            ClipKind::Text | ClipKind::Phrase => ProductAiContextKind::SelectedText,
            ClipKind::Image => ProductAiContextKind::SelectedImage,
            ClipKind::Files => ProductAiContextKind::SelectedFilePath,
        };
        if !contexts.contains(&context) {
            contexts.push(context);
        }
    }

    let target_item_ids = targets
        .iter()
        .filter_map(|item| (item.id > 0).then_some(item.id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut seen = BTreeSet::new();
    let mut capabilities = Vec::new();
    for context in &contexts {
        for capability in
            product_ai_capabilities_for_context(ProductAiUiSurface::RowContextMenu, *context)
        {
            if seen.insert(capability.id) {
                capabilities.push(MainRowAiCapabilityPresentation {
                    capability_id: capability.id,
                    label: capability.label,
                    provider: capability.provider,
                    action: capability.action,
                    result: capability.result,
                });
            }
        }
    }

    (!capabilities.is_empty()).then_some(MainRowAiCapabilityPlan {
        contexts,
        target_item_ids,
        capabilities,
    })
}

pub(crate) fn main_row_ai_invocation(
    plan: &MainRowAiCapabilityPlan,
    capability_id: &str,
    input_text: impl Into<String>,
) -> Option<ProductAiInvocation> {
    plan.capabilities
        .iter()
        .any(|capability| capability.capability_id == capability_id)
        .then(|| ProductAiInvocation {
            capability_id: capability_id.to_string(),
            input_text: input_text.into(),
            context_item_ids: plan.target_item_ids.clone(),
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainRowDialogActionPlan {
    MailMerge { excel_path: Option<String> },
    EditItem { item_id: i64, title: String },
}

pub(crate) fn main_row_dialog_action_plan(
    action: MainRowMenuAction,
    current: Option<&ClipItem>,
) -> Option<MainRowDialogActionPlan> {
    let item = current?;
    match action {
        MainRowMenuAction::MailMerge => Some(MainRowDialogActionPlan::MailMerge {
            excel_path: item
                .file_paths
                .as_ref()
                .and_then(|paths| paths.first())
                .cloned(),
        }),
        MainRowMenuAction::Edit => Some(MainRowDialogActionPlan::EditItem {
            item_id: item.id,
            title: format!(
                "编辑 — {}",
                item.preview.chars().take(40).collect::<String>()
            ),
        }),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub(crate) enum MainRowCurrentItemActionPlan {
    Sticker { item: ClipItem },
    SaveImage { item: ClipItem },
    ImageOcr { item: ClipItem },
}

pub(crate) fn main_row_current_item_action_plan(
    action: MainRowMenuAction,
    current: Option<&ClipItem>,
) -> Option<MainRowCurrentItemActionPlan> {
    let item = current?;
    match action {
        MainRowMenuAction::Sticker if item.kind == ClipKind::Image => {
            Some(MainRowCurrentItemActionPlan::Sticker { item: item.clone() })
        }
        MainRowMenuAction::SaveImage if item.kind == ClipKind::Image => {
            Some(MainRowCurrentItemActionPlan::SaveImage { item: item.clone() })
        }
        MainRowMenuAction::ImageOcr if matches!(item.kind, ClipKind::Image | ClipKind::Files) => {
            Some(MainRowCurrentItemActionPlan::ImageOcr { item: item.clone() })
        }
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainRowDataActionPlan {
    AddToPhrase {
        items: Vec<ClipItem>,
        refilter_current_tab: bool,
    },
    UpdatePinned {
        ids: Vec<i64>,
        pinned: bool,
        invalidate_tab: usize,
        clear_selection: bool,
    },
    DeleteItems {
        ids: Vec<i64>,
        clear_selection: bool,
        preserve_scroll_anchor: bool,
    },
    DeleteUnpinned {
        category: i64,
        active_tab: usize,
        clear_selection: bool,
        preserve_scroll_anchor: bool,
    },
    AssignGroup {
        ids: Vec<i64>,
        group_id: i64,
        refilter_after_reload: bool,
    },
}

pub(crate) fn main_row_data_action_plan(
    action: MainRowMenuAction,
    current: Option<&ClipItem>,
    selected: &[ClipItem],
    tab_index: usize,
) -> Option<MainRowDataActionPlan> {
    match action {
        MainRowMenuAction::ToPhrase => {
            let items = if selected.is_empty() {
                current.into_iter().cloned().collect()
            } else {
                selected.to_vec()
            };
            (!items.is_empty()).then_some(MainRowDataActionPlan::AddToPhrase {
                items,
                refilter_current_tab: tab_index == 1,
            })
        }
        _ => None,
    }
}

pub(crate) fn main_row_pin_data_plan(
    current: Option<&ClipItem>,
    selected: &[ClipItem],
    tab_index: usize,
) -> Option<MainRowDataActionPlan> {
    let items = if selected.is_empty() {
        current.into_iter().cloned().collect::<Vec<_>>()
    } else {
        selected.to_vec()
    };
    let pinned = items.iter().any(|item| !item.pinned);
    let ids = items
        .iter()
        .map(|item| item.id)
        .filter(|id| *id > 0)
        .collect::<Vec<_>>();
    (!ids.is_empty()).then_some(MainRowDataActionPlan::UpdatePinned {
        ids,
        pinned,
        invalidate_tab: tab_index,
        clear_selection: !selected.is_empty(),
    })
}

pub(crate) fn main_row_delete_items_data_plan(
    current: Option<&ClipItem>,
    selected: &[ClipItem],
) -> Option<MainRowDataActionPlan> {
    let items = if selected.is_empty() {
        current.into_iter().cloned().collect::<Vec<_>>()
    } else {
        selected.to_vec()
    };
    let ids = items
        .iter()
        .map(|item| item.id)
        .filter(|id| *id > 0)
        .collect::<Vec<_>>();
    (!ids.is_empty()).then_some(MainRowDataActionPlan::DeleteItems {
        ids,
        clear_selection: true,
        preserve_scroll_anchor: true,
    })
}

pub(crate) fn main_row_delete_unpinned_data_plan(
    category: i64,
    active_tab: usize,
) -> MainRowDataActionPlan {
    MainRowDataActionPlan::DeleteUnpinned {
        category,
        active_tab,
        clear_selection: true,
        preserve_scroll_anchor: true,
    }
}

pub(crate) fn main_row_group_assignment_plan(
    ids: &[i64],
    group_id: i64,
    refilter_after_reload: bool,
) -> Option<MainRowDataActionPlan> {
    let ids = ids.iter().copied().filter(|id| *id > 0).collect::<Vec<_>>();
    (!ids.is_empty()).then_some(MainRowDataActionPlan::AssignGroup {
        ids,
        group_id,
        refilter_after_reload,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClipItem {
    pub(crate) id: i64,
    pub(crate) kind: ClipKind,
    pub(crate) preview: String,
    pub(crate) text: Option<String>,
    pub(crate) source_app: String,
    pub(crate) file_paths: Option<Vec<String>>,
    pub(crate) image_bytes: Option<Vec<u8>>,
    pub(crate) image_path: Option<String>,
    pub(crate) image_width: usize,
    pub(crate) image_height: usize,
    pub(crate) pinned: bool,
    pub(crate) group_id: i64,
    pub(crate) created_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchTimeFilter {
    ExactDay(i64),
    RecentDays(i64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchDateContext {
    pub(crate) current_day: i64,
    pub(crate) current_year: i32,
}

impl SearchDateContext {
    pub(crate) fn from_date(year: i32, month: i32, day: i32) -> Self {
        Self {
            current_day: gregorian_to_days(year, month, day),
            current_year: year,
        }
    }

    pub(crate) fn utc_now() -> Self {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        let (year, month, day) = days_to_gregorian(now_secs.div_euclid(86_400));
        Self::from_date(year, month, day)
    }
}

fn parse_time_filter(raw: &str, date_context: SearchDateContext) -> Option<SearchTimeFilter> {
    let value = raw.trim().to_lowercase();
    if value.is_empty() {
        return None;
    }
    match value.as_str() {
        "today" | "今天" => Some(SearchTimeFilter::ExactDay(date_context.current_day)),
        "yesterday" | "昨天" => Some(SearchTimeFilter::ExactDay(date_context.current_day - 1)),
        "week" | "本周" | "最近7天" => Some(SearchTimeFilter::RecentDays(7)),
        "month" | "本月" | "最近30天" => Some(SearchTimeFilter::RecentDays(30)),
        _ => {
            if let Some(days) = value
                .strip_suffix('d')
                .or_else(|| value.strip_suffix("day"))
                .or_else(|| value.strip_suffix("days"))
                .or_else(|| value.strip_suffix('天'))
                .and_then(|v| v.trim().parse::<i64>().ok())
            {
                return Some(SearchTimeFilter::RecentDays(days.max(1)));
            }

            if let Some((y, m, d)) = value
                .split_once('-')
                .and_then(|(a, rest)| rest.split_once('-').map(|(b, c)| (a, b, c)))
                .and_then(|(y, m, d)| {
                    Some((
                        y.parse::<i32>().ok()?,
                        m.parse::<i32>().ok()?,
                        d.parse::<i32>().ok()?,
                    ))
                })
            {
                return Some(SearchTimeFilter::ExactDay(gregorian_to_days(y, m, d)));
            }

            if let Some((m, d)) = value
                .split_once('-')
                .and_then(|(m, d)| Some((m.parse::<i32>().ok()?, d.parse::<i32>().ok()?)))
            {
                return Some(SearchTimeFilter::ExactDay(gregorian_to_days(
                    date_context.current_year,
                    m,
                    d,
                )));
            }

            None
        }
    }
}

fn tokenize_search_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in query.chars() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
            }
            c if c.is_whitespace() => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    tokens
}

fn prefixed_search_value<'a>(
    token: &'a str,
    ascii_prefix: &str,
    cn_prefixes: &[&str],
) -> Option<&'a str> {
    let lower = token.to_lowercase();
    if let Some(value) = lower.strip_prefix(ascii_prefix) {
        let start = token.len().saturating_sub(value.len());
        return Some(&token[start..]);
    }
    for prefix in cn_prefixes {
        if let Some(value) = token.strip_prefix(prefix) {
            return Some(value);
        }
    }
    None
}

fn is_prefixed_search_token(token: &str) -> bool {
    prefixed_search_value(token, "time:", &["时间:", "时间：", "日期:", "日期："]).is_some()
        || prefixed_search_value(token, "date:", &["时间:", "时间：", "日期:", "日期："]).is_some()
        || prefixed_search_value(token, "app:", &["应用:", "应用："]).is_some()
}

fn collect_prefixed_value(tokens: &[String], start: usize, initial: &str) -> (String, usize) {
    let mut parts = Vec::new();
    if !initial.trim().is_empty() {
        parts.push(initial.trim().to_string());
    }
    let mut index = start + 1;
    while index < tokens.len() {
        let token = tokens[index].trim();
        if token.is_empty() || is_prefixed_search_token(token) {
            break;
        }
        parts.push(token.to_string());
        index += 1;
    }
    (parts.join(" ").trim().to_string(), index)
}

pub(crate) fn parse_search_query(
    query: &str,
) -> (Vec<String>, Option<SearchTimeFilter>, Option<String>) {
    parse_search_query_with_context(query, SearchDateContext::utc_now())
}

pub(crate) fn parse_search_query_with_context(
    query: &str,
    date_context: SearchDateContext,
) -> (Vec<String>, Option<SearchTimeFilter>, Option<String>) {
    let mut text_terms = Vec::new();
    let mut time_filter = None;
    let mut app_filter = None;

    let tokens = tokenize_search_query(query);
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index].trim();
        if token.is_empty() {
            index += 1;
            continue;
        }

        if let Some(initial) =
            prefixed_search_value(token, "time:", &["时间:", "时间：", "日期:", "日期："]).or_else(
                || prefixed_search_value(token, "date:", &["时间:", "时间：", "日期:", "日期："]),
            )
        {
            let (value, next_index) = collect_prefixed_value(&tokens, index, initial);
            if let Some(filter) = parse_time_filter(&value, date_context) {
                time_filter = Some(filter);
                index = next_index;
                continue;
            }
        }

        if let Some(initial) = prefixed_search_value(token, "app:", &["应用:", "应用："]) {
            let (value, next_index) = collect_prefixed_value(&tokens, index, initial);
            if !value.trim().is_empty() {
                app_filter = Some(value.trim().to_lowercase());
                index = next_index;
                continue;
            }
        }

        text_terms.push(token.to_lowercase());
        index += 1;
    }

    (text_terms, time_filter, app_filter)
}

#[derive(Clone, Debug)]
pub(crate) struct ClipListState {
    pub(crate) visible_len: usize,
    pub(crate) tab_index: usize,
    pub(crate) search_on: bool,
    pub(crate) search_text: String,
    pub(crate) hover_idx: i32,
    pub(crate) sel_idx: i32,
    pub(crate) scroll_y: i32,
    pub(crate) current_group_filter: i64,
    pub(crate) tab_group_filters: [i64; 2],
    pub(crate) selected_rows: BTreeSet<i32>,
    pub(crate) selection_anchor: i32,
    pub(crate) context_row: i32,
}

impl Default for ClipListState {
    fn default() -> Self {
        Self {
            visible_len: 0,
            tab_index: 0,
            search_on: false,
            search_text: String::new(),
            hover_idx: -1,
            sel_idx: -1,
            scroll_y: 0,
            current_group_filter: 0,
            tab_group_filters: [0, 0],
            selected_rows: BTreeSet::new(),
            selection_anchor: -1,
            context_row: -1,
        }
    }
}

impl ClipListState {
    pub(crate) fn apply_visible_len(&mut self, len: usize) {
        self.visible_len = len;
        self.sync_visible_state();
    }

    fn sync_visible_state(&mut self) {
        if self.sel_idx >= self.visible_len as i32 {
            self.sel_idx = if self.visible_len == 0 { -1 } else { 0 };
        }
        if self.hover_idx >= self.visible_len as i32 {
            self.hover_idx = -1;
        }
        let max_idx = self.visible_len as i32;
        self.selected_rows = self
            .selected_rows
            .iter()
            .copied()
            .filter(|i| *i >= 0 && *i < max_idx)
            .collect();
        if self.sel_idx >= max_idx {
            self.sel_idx = if max_idx > 0 { max_idx - 1 } else { -1 };
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.sel_idx = -1;
        self.hover_idx = -1;
        self.selected_rows.clear();
        self.selection_anchor = -1;
        self.context_row = -1;
    }

    pub(crate) fn tab_switch_plan(&self, target_tab: usize) -> MainTabSwitchPlan {
        let tab_index = if target_tab == 1 { 1 } else { 0 };
        MainTabSwitchPlan {
            tab_index,
            current_group_filter: self.tab_group_filters[tab_index],
            scroll_y: 0,
            clear_selection: true,
        }
    }

    pub(crate) fn apply_tab_switch_plan(&mut self, plan: MainTabSwitchPlan) {
        self.tab_index = plan.tab_index;
        self.current_group_filter = plan.current_group_filter;
        self.scroll_y = plan.scroll_y;
        if plan.clear_selection {
            self.clear_selection();
        }
    }

    pub(crate) fn group_filter_plan(
        &self,
        target_tab: usize,
        group_id: i64,
    ) -> MainGroupFilterPlan {
        let tab_index = if target_tab == 1 { 1 } else { 0 };
        let mut tab_group_filters = self.tab_group_filters;
        tab_group_filters[tab_index] = group_id;
        MainGroupFilterPlan {
            tab_index,
            tab_group_filters,
            current_group_filter: group_id,
            scroll_y: 0,
            clear_selection: true,
        }
    }

    pub(crate) fn apply_group_filter_plan(&mut self, plan: MainGroupFilterPlan) {
        self.tab_index = plan.tab_index;
        self.tab_group_filters = plan.tab_group_filters;
        self.current_group_filter = plan.current_group_filter;
        self.scroll_y = plan.scroll_y;
        if plan.clear_selection {
            self.clear_selection();
        }
    }

    pub(crate) fn scroll_to_top_release_plan(&self, activated: bool) -> MainScrollToTopReleasePlan {
        MainScrollToTopReleasePlan {
            down_scroll_to_top: false,
            scroll_y: if activated { 0 } else { self.scroll_y },
            show_scrollbar_feedback: activated,
        }
    }

    pub(crate) fn apply_scroll_to_top_release_plan(&mut self, plan: MainScrollToTopReleasePlan) {
        self.scroll_y = plan.scroll_y;
    }

    pub(crate) fn scroll_position_update_plan(&self, scroll_y: i32) -> MainScrollUpdate {
        MainScrollUpdate {
            scroll_y,
            changed: self.scroll_y != scroll_y,
        }
    }

    pub(crate) fn apply_scroll_update(&mut self, update: MainScrollUpdate) {
        self.scroll_y = update.scroll_y;
    }

    pub(crate) fn search_filter_apply_plan(&self) -> MainSearchFilterApplyPlan {
        MainSearchFilterApplyPlan {
            sel_idx: 0,
            scroll_y: 0,
        }
    }

    pub(crate) fn apply_search_filter_plan(&mut self, plan: MainSearchFilterApplyPlan) {
        self.sel_idx = plan.sel_idx;
        self.scroll_y = plan.scroll_y;
    }

    pub(crate) fn search_reset_plan(&self) -> Option<MainSearchResetPlan> {
        if !self.search_on && self.search_text.is_empty() {
            return None;
        }
        Some(MainSearchResetPlan {
            search_on: false,
            search_text: String::new(),
            clear_selection: true,
        })
    }

    pub(crate) fn apply_search_reset_plan(&mut self, plan: MainSearchResetPlan) {
        self.search_on = plan.search_on;
        self.search_text = plan.search_text;
        if plan.clear_selection {
            self.clear_selection();
        }
    }

    pub(crate) fn row_release_state_plan(
        &self,
        release: MainRowRelease,
    ) -> MainRowReleaseStatePlan {
        MainRowReleaseStatePlan {
            down_row: -1,
            down_x: 0,
            down_y: 0,
            sel_idx: if release.accepted {
                release.release_row
            } else {
                self.sel_idx
            },
        }
    }

    pub(crate) fn apply_row_release_state_plan(&mut self, plan: MainRowReleaseStatePlan) {
        self.sel_idx = plan.sel_idx;
    }

    pub(crate) fn pointer_up_press_clear_plan(&self) -> MainPointerUpPressClearPlan {
        MainPointerUpPressClearPlan {
            down_row: -1,
            down_x: 0,
            down_y: 0,
        }
    }

    pub(crate) fn row_pointer_down_focus_plan(
        &self,
        row: i32,
    ) -> Option<MainRowPointerDownFocusPlan> {
        if row < 0 || row as usize >= self.visible_len {
            return None;
        }
        Some(MainRowPointerDownFocusPlan { sel_idx: row })
    }

    pub(crate) fn apply_row_pointer_down_focus_plan(&mut self, plan: MainRowPointerDownFocusPlan) {
        self.sel_idx = plan.sel_idx;
    }

    pub(crate) fn row_double_click_state_plan(
        &self,
        row: i32,
    ) -> Option<MainRowDoubleClickStatePlan> {
        if row < 0 || row as usize >= self.visible_len {
            return None;
        }
        Some(MainRowDoubleClickStatePlan {
            paste_sel_idx: row,
            finish_sel_idx: -1,
            finish_hover_idx: -1,
        })
    }

    pub(crate) fn apply_row_double_click_focus_plan(&mut self, plan: MainRowDoubleClickStatePlan) {
        self.sel_idx = plan.paste_sel_idx;
    }

    pub(crate) fn apply_row_double_click_finish_plan(&mut self, plan: MainRowDoubleClickStatePlan) {
        self.sel_idx = plan.finish_sel_idx;
        self.hover_idx = plan.finish_hover_idx;
    }

    pub(crate) fn context_menu_state_plan(
        &self,
        row: i32,
        ctrl: bool,
        shift: bool,
    ) -> Option<MainContextMenuStatePlan> {
        if row < 0 || row as usize >= self.visible_len {
            return None;
        }

        let mut next = self.clone();
        next.apply_context_pointer_selection(row, ctrl, shift);
        next.context_row = row;
        let context_selection_count = next.context_selection_count();

        Some(MainContextMenuStatePlan {
            row,
            sel_idx: next.sel_idx,
            selected_rows: next.selected_rows,
            selection_anchor: next.selection_anchor,
            context_row: next.context_row,
            context_selection_count,
        })
    }

    pub(crate) fn apply_context_menu_state_plan(&mut self, plan: MainContextMenuStatePlan) {
        self.sel_idx = plan.sel_idx;
        self.selected_rows = plan.selected_rows;
        self.selection_anchor = plan.selection_anchor;
        self.context_row = plan.context_row;
    }

    pub(crate) fn context_row_selection_plan(&self) -> Option<MainContextRowSelectionPlan> {
        if self.context_row < 0 || self.context_row as usize >= self.visible_len {
            return None;
        }
        Some(MainContextRowSelectionPlan {
            sel_idx: self.context_row,
        })
    }

    pub(crate) fn apply_context_row_selection_plan(&mut self, plan: MainContextRowSelectionPlan) {
        self.sel_idx = plan.sel_idx;
    }

    pub(crate) fn context_menu_finish_plan(&self) -> MainContextMenuFinishPlan {
        MainContextMenuFinishPlan { context_row: -1 }
    }

    pub(crate) fn apply_context_menu_finish_plan(&mut self, plan: MainContextMenuFinishPlan) {
        self.context_row = plan.context_row;
    }

    pub(crate) fn keyboard_move_selection_plan(
        &self,
        delta: i32,
        extend: bool,
    ) -> Option<MainSelectionPlan> {
        if self.visible_len == 0 {
            return None;
        }

        let last_idx = self.visible_len as i32 - 1;
        let sel_idx = if delta < 0 {
            if self.sel_idx <= 0 {
                0
            } else {
                self.sel_idx - 1
            }
        } else if self.sel_idx < 0 {
            0
        } else {
            self.sel_idx.saturating_add(1).min(last_idx)
        };

        let mut selected_rows = BTreeSet::new();
        let selection_anchor = if extend {
            let anchor = if self.selection_anchor < 0 {
                self.sel_idx
            } else {
                self.selection_anchor
            };
            let a = anchor.min(sel_idx);
            let b = anchor.max(sel_idx);
            for row in a..=b {
                selected_rows.insert(row);
            }
            anchor
        } else {
            -1
        };

        Some(MainSelectionPlan {
            sel_idx,
            selected_rows,
            selection_anchor,
        })
    }

    pub(crate) fn select_all_selection_plan(&self) -> MainSelectionPlan {
        let selected_rows = (0..self.visible_len as i32).collect();
        MainSelectionPlan {
            sel_idx: self.sel_idx,
            selected_rows,
            selection_anchor: 0,
        }
    }

    pub(crate) fn escape_shortcut_plan(&self) -> MainShortcutEscapePlan {
        if !self.selected_rows.is_empty() {
            MainShortcutEscapePlan::ClearSelection
        } else if self.search_on {
            MainShortcutEscapePlan::CloseSearch
        } else {
            MainShortcutEscapePlan::HideWindow
        }
    }

    pub(crate) fn activate_selection_plan(&self) -> MainActivateSelectionPlan {
        if self.context_selection_count() > 1 {
            MainActivateSelectionPlan::CopySelectionThenPaste
        } else {
            MainActivateSelectionPlan::PasteSelection
        }
    }

    pub(crate) fn shortcut_row_command_plan(
        &self,
        command: MainShortcutRowCommand,
    ) -> MainShortcutRowCommandPlan {
        MainShortcutRowCommandPlan {
            context_row: self.sel_idx,
            command,
        }
    }

    pub(crate) fn apply_shortcut_row_command_plan(&mut self, plan: MainShortcutRowCommandPlan) {
        self.context_row = plan.context_row;
    }

    pub(crate) fn apply_selection_plan(&mut self, plan: MainSelectionPlan) {
        self.sel_idx = plan.sel_idx;
        self.selected_rows = plan.selected_rows;
        self.selection_anchor = plan.selection_anchor;
    }

    pub(crate) fn row_is_selected(&self, visible_idx: i32) -> bool {
        visible_idx >= 0
            && (self.sel_idx == visible_idx || self.selected_rows.contains(&visible_idx))
    }

    pub(crate) fn selected_visible_rows(&self) -> Vec<i32> {
        let mut rows: Vec<i32> = self.selected_rows.iter().copied().collect();
        if self.sel_idx >= 0 && !rows.contains(&self.sel_idx) {
            rows.push(self.sel_idx);
        }
        rows.sort_unstable();
        rows
    }

    pub(crate) fn selected_source_indices(&self) -> Vec<usize> {
        let mut src: Vec<usize> = self
            .selected_visible_rows()
            .into_iter()
            .filter(|v| *v >= 0 && (*v as usize) < self.visible_len)
            .map(|v| v as usize)
            .collect();
        src.sort_unstable();
        src.dedup();
        src
    }

    pub(crate) fn selected_count(&self) -> usize {
        self.selected_source_indices().len()
    }

    pub(crate) fn context_selection_count(&self) -> usize {
        let n = self.selected_count();
        if n == 0 && self.context_row >= 0 {
            1
        } else {
            n
        }
    }

    pub(crate) fn apply_primary_pointer_selection(
        &mut self,
        visible_idx: i32,
        ctrl: bool,
        shift: bool,
    ) {
        if visible_idx < 0 {
            return;
        }
        if ctrl {
            if !self.selected_rows.insert(visible_idx) {
                self.selected_rows.remove(&visible_idx);
            }
            self.selection_anchor = visible_idx;
        } else if shift {
            if self.selection_anchor < 0 {
                self.selection_anchor = visible_idx;
            }
            self.selected_rows.clear();
            let a = self.selection_anchor.min(visible_idx);
            let b = self.selection_anchor.max(visible_idx);
            for i in a..=b {
                self.selected_rows.insert(i);
            }
        }
        self.sel_idx = visible_idx;
    }

    pub(crate) fn apply_context_pointer_selection(
        &mut self,
        visible_idx: i32,
        ctrl: bool,
        shift: bool,
    ) {
        if visible_idx < 0 {
            return;
        }
        if shift && self.selection_anchor >= 0 {
            self.selected_rows.clear();
            let a = self.selection_anchor.min(visible_idx);
            let b = self.selection_anchor.max(visible_idx);
            for i in a..=b {
                self.selected_rows.insert(i);
            }
            self.sel_idx = visible_idx;
        } else if ctrl {
            if !self.selected_rows.insert(visible_idx) {
                self.selected_rows.remove(&visible_idx);
            }
            self.sel_idx = visible_idx;
            if self.selection_anchor < 0 {
                self.selection_anchor = visible_idx;
            }
        } else {
            let already_multi_selected =
                self.selected_rows.len() > 1 && self.selected_rows.contains(&visible_idx);
            if !already_multi_selected {
                self.selected_rows.clear();
                self.sel_idx = visible_idx;
                self.selection_anchor = visible_idx;
            } else {
                self.sel_idx = visible_idx;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainTabSwitchPlan {
    pub(crate) tab_index: usize,
    pub(crate) current_group_filter: i64,
    pub(crate) scroll_y: i32,
    pub(crate) clear_selection: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainGroupFilterPlan {
    pub(crate) tab_index: usize,
    pub(crate) tab_group_filters: [i64; 2],
    pub(crate) current_group_filter: i64,
    pub(crate) scroll_y: i32,
    pub(crate) clear_selection: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainScrollToTopReleasePlan {
    pub(crate) down_scroll_to_top: bool,
    pub(crate) scroll_y: i32,
    pub(crate) show_scrollbar_feedback: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowReleaseStatePlan {
    pub(crate) down_row: i32,
    pub(crate) down_x: i32,
    pub(crate) down_y: i32,
    pub(crate) sel_idx: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPointerUpPressClearPlan {
    pub(crate) down_row: i32,
    pub(crate) down_x: i32,
    pub(crate) down_y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowPointerDownFocusPlan {
    pub(crate) sel_idx: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainSearchFilterApplyPlan {
    pub(crate) sel_idx: i32,
    pub(crate) scroll_y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainSearchResetPlan {
    pub(crate) search_on: bool,
    pub(crate) search_text: String,
    pub(crate) clear_selection: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowDoubleClickStatePlan {
    pub(crate) paste_sel_idx: i32,
    pub(crate) finish_sel_idx: i32,
    pub(crate) finish_hover_idx: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainContextMenuStatePlan {
    pub(crate) row: i32,
    pub(crate) sel_idx: i32,
    pub(crate) selected_rows: BTreeSet<i32>,
    pub(crate) selection_anchor: i32,
    pub(crate) context_row: i32,
    pub(crate) context_selection_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainContextRowSelectionPlan {
    pub(crate) sel_idx: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainContextMenuFinishPlan {
    pub(crate) context_row: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainSelectionPlan {
    pub(crate) sel_idx: i32,
    pub(crate) selected_rows: BTreeSet<i32>,
    pub(crate) selection_anchor: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainShortcutEscapePlan {
    ClearSelection,
    CloseSearch,
    HideWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainActivateSelectionPlan {
    PasteSelection,
    CopySelectionThenPaste,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainCopySelectionPlan {
    CopyCurrentItem,
    CopyMergedText(String),
}

pub(crate) fn main_copy_selection_plan(
    current: Option<&ClipItem>,
    selected: &[ClipItem],
) -> Option<MainCopySelectionPlan> {
    if selected.len() <= 1 {
        return current.map(|_| MainCopySelectionPlan::CopyCurrentItem);
    }
    let mut parts: Vec<String> = Vec::new();
    for item in selected {
        match item.kind {
            ClipKind::Text | ClipKind::Phrase => {
                if let Some(text) = &item.text {
                    let text = text.trim();
                    if !text.is_empty() {
                        parts.push(text.to_string());
                    }
                }
            }
            ClipKind::Image => {
                let preview = item.preview.trim();
                if !preview.is_empty() {
                    parts.push(preview.to_string());
                }
            }
            ClipKind::Files => {
                if let Some(paths) = &item.file_paths {
                    for path in paths {
                        let path = path.trim();
                        if !path.is_empty() {
                            parts.push(path.to_string());
                        }
                    }
                }
            }
        }
    }
    let merged = parts.join("\n");
    (!merged.trim().is_empty()).then_some(MainCopySelectionPlan::CopyMergedText(merged))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainPasteCompletionKind {
    DirectEdit,
    AsyncImage,
    Clipboard,
    VvAsyncImage,
    VvClipboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPasteCompletionInput {
    pub(crate) item_id: i64,
    pub(crate) move_pasted_item_to_top: bool,
    pub(crate) click_hide: bool,
    pub(crate) paste_success_sound_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPasteCompletionPlan {
    pub(crate) promote_item_id: Option<i64>,
    pub(crate) reset_plain_text_paste_mode: bool,
    pub(crate) clear_selection: bool,
    pub(crate) clear_hover: bool,
    pub(crate) hide_main_now: bool,
    pub(crate) play_success_sound: bool,
    pub(crate) send_paste_after_clipboard: bool,
    pub(crate) paste_hide_main: bool,
    pub(crate) paste_backspaces: u8,
}

pub(crate) fn main_paste_completion_plan(
    kind: MainPasteCompletionKind,
    input: MainPasteCompletionInput,
) -> MainPasteCompletionPlan {
    let promote_item_id =
        (input.move_pasted_item_to_top && input.item_id > 0).then_some(input.item_id);
    match kind {
        MainPasteCompletionKind::DirectEdit => MainPasteCompletionPlan {
            promote_item_id,
            reset_plain_text_paste_mode: false,
            clear_selection: true,
            clear_hover: true,
            hide_main_now: input.click_hide,
            play_success_sound: input.paste_success_sound_enabled,
            send_paste_after_clipboard: false,
            paste_hide_main: false,
            paste_backspaces: 0,
        },
        MainPasteCompletionKind::AsyncImage => MainPasteCompletionPlan {
            promote_item_id,
            reset_plain_text_paste_mode: false,
            clear_selection: true,
            clear_hover: true,
            hide_main_now: input.click_hide,
            play_success_sound: false,
            send_paste_after_clipboard: false,
            paste_hide_main: false,
            paste_backspaces: 0,
        },
        MainPasteCompletionKind::Clipboard => MainPasteCompletionPlan {
            promote_item_id,
            reset_plain_text_paste_mode: true,
            clear_selection: true,
            clear_hover: true,
            hide_main_now: false,
            play_success_sound: false,
            send_paste_after_clipboard: true,
            paste_hide_main: input.click_hide,
            paste_backspaces: 0,
        },
        MainPasteCompletionKind::VvAsyncImage => MainPasteCompletionPlan {
            promote_item_id,
            reset_plain_text_paste_mode: false,
            clear_selection: false,
            clear_hover: false,
            hide_main_now: false,
            play_success_sound: false,
            send_paste_after_clipboard: false,
            paste_hide_main: false,
            paste_backspaces: 0,
        },
        MainPasteCompletionKind::VvClipboard => MainPasteCompletionPlan {
            promote_item_id,
            reset_plain_text_paste_mode: false,
            clear_selection: false,
            clear_hover: false,
            hide_main_now: false,
            play_success_sound: false,
            send_paste_after_clipboard: true,
            paste_hide_main: input.click_hide,
            paste_backspaces: 0,
        },
    }
}

pub(crate) fn main_paste_completion_plan_with_backspaces(
    kind: MainPasteCompletionKind,
    input: MainPasteCompletionInput,
    backspaces: u8,
) -> MainPasteCompletionPlan {
    let mut plan = main_paste_completion_plan(kind, input);
    plan.paste_backspaces = backspaces;
    plan
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainPastePreparationStep {
    DirectEdit,
    AsyncImage,
    Clipboard { plain_text: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPastePreparationInput {
    pub(crate) item_kind: ClipKind,
    pub(crate) item_id: i64,
    pub(crate) image_payload_loaded: bool,
    pub(crate) direct_edit_candidate: bool,
    pub(crate) plain_text_paste_mode: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainPastePreparationPlan {
    pub(crate) steps: Vec<MainPastePreparationStep>,
}

pub(crate) fn main_paste_preparation_plan(
    input: MainPastePreparationInput,
) -> MainPastePreparationPlan {
    let mut steps = Vec::new();
    if input.direct_edit_candidate {
        steps.push(MainPastePreparationStep::DirectEdit);
    }
    if input.item_kind == ClipKind::Image && input.item_id > 0 && !input.image_payload_loaded {
        steps.push(MainPastePreparationStep::AsyncImage);
    }
    steps.push(MainPastePreparationStep::Clipboard {
        plain_text: input.plain_text_paste_mode,
    });
    MainPastePreparationPlan { steps }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MainVvSelectPlan {
    HidePopup,
    Paste { item: ClipItem, backspaces: u8 },
}

pub(crate) fn main_vv_select_plan(
    popup_visible: bool,
    index: usize,
    items: &[ClipItem],
    backspaces: u8,
) -> Option<MainVvSelectPlan> {
    if !popup_visible {
        return None;
    }
    items
        .get(index)
        .cloned()
        .map(|item| MainVvSelectPlan::Paste { item, backspaces })
        .or(Some(MainVvSelectPlan::HidePopup))
}

pub(crate) const MAIN_VV_POPUP_MAX_ITEMS: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainVvPopupHit {
    Group,
    Row(usize),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainVvPopupTextRole {
    Title,
    GroupName,
    GroupArrow,
    Hint,
    Empty,
    RowIndex,
    RowPreview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainVvPopupTextCommand {
    pub(crate) role: MainVvPopupTextRole,
    pub(crate) text: String,
    pub(crate) rect: UiRect,
    pub(crate) color: MainThemeRole,
    pub(crate) size: i32,
    pub(crate) bold: bool,
    pub(crate) horizontal_align: HorizontalAlign,
    pub(crate) font: MainFontRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainVvPopupRenderItem {
    pub(crate) index: usize,
    pub(crate) label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainVvPopupRenderPlan {
    pub(crate) paint_commands: Vec<MainPaintCommand>,
    pub(crate) text_commands: Vec<MainVvPopupTextCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainVvPopupRenderStrings {
    pub(crate) title: String,
    pub(crate) hint: String,
    pub(crate) empty: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainVvPopupLayout {
    pub(crate) width: i32,
    pub(crate) header_h: i32,
    pub(crate) row_h: i32,
}

impl Default for MainVvPopupLayout {
    fn default() -> Self {
        Self {
            width: 360,
            header_h: 58,
            row_h: 30,
        }
    }
}

impl MainVvPopupLayout {
    pub(crate) fn height(self, rows: usize) -> i32 {
        self.header_h + 20 + rows.max(1) as i32 * self.row_h
    }

    pub(crate) fn row_rect(self, row: usize) -> UiRect {
        let top = self.header_h + 10 + row as i32 * self.row_h;
        UiRect::new(12, top, self.width - 12, top + self.row_h - 2)
    }

    pub(crate) fn group_rect(self) -> UiRect {
        UiRect::new(self.width - 150, 10, self.width - 14, 34)
    }

    pub(crate) fn hit_test(self, x: i32, y: i32, rows: usize) -> MainVvPopupHit {
        if self.group_rect().contains(x, y) {
            return MainVvPopupHit::Group;
        }
        for row in 0..rows {
            let rect = self.row_rect(row);
            if y >= rect.top && y < rect.bottom {
                return MainVvPopupHit::Row(row);
            }
        }
        MainVvPopupHit::None
    }

    pub(crate) fn render_plan(
        self,
        client_rect: UiRect,
        strings: &MainVvPopupRenderStrings,
        group_name: &str,
        items: &[MainVvPopupRenderItem],
    ) -> MainVvPopupRenderPlan {
        let mut paint_commands = vec![MainPaintCommand::RoundRect {
            rect: client_rect,
            fill: MainPaintFill::Theme(MainThemeRole::Surface),
            stroke: Some(MainThemeRole::Stroke),
            radius: 12,
        }];
        let mut text_commands = vec![
            MainVvPopupTextCommand {
                role: MainVvPopupTextRole::Title,
                text: strings.title.clone(),
                rect: UiRect::new(14, 10, 150, 30),
                color: MainThemeRole::Text,
                size: 13,
                bold: true,
                horizontal_align: HorizontalAlign::Start,
                font: MainFontRole::Display,
            },
            MainVvPopupTextCommand {
                role: MainVvPopupTextRole::Hint,
                text: strings.hint.clone(),
                rect: UiRect::new(14, 34, client_rect.right - 14, 52),
                color: MainThemeRole::TextMuted,
                size: 11,
                bold: false,
                horizontal_align: HorizontalAlign::Start,
                font: MainFontRole::UiText,
            },
        ];

        let group_rect = self.group_rect();
        paint_commands.push(MainPaintCommand::RoundFill {
            rect: group_rect,
            fill: MainPaintFill::Theme(MainThemeRole::Background),
            radius: 10,
        });
        paint_commands.push(MainPaintCommand::RoundRect {
            rect: group_rect,
            fill: MainPaintFill::Theme(MainThemeRole::Background),
            stroke: Some(MainThemeRole::Stroke),
            radius: 10,
        });
        text_commands.push(MainVvPopupTextCommand {
            role: MainVvPopupTextRole::GroupName,
            text: group_name.to_string(),
            rect: UiRect::new(
                group_rect.left + 10,
                group_rect.top,
                group_rect.right - 20,
                group_rect.bottom,
            ),
            color: MainThemeRole::Text,
            size: 11,
            bold: false,
            horizontal_align: HorizontalAlign::Center,
            font: MainFontRole::UiText,
        });
        text_commands.push(MainVvPopupTextCommand {
            role: MainVvPopupTextRole::GroupArrow,
            text: "v".to_string(),
            rect: UiRect::new(
                group_rect.right - 18,
                group_rect.top,
                group_rect.right - 4,
                group_rect.bottom,
            ),
            color: MainThemeRole::TextMuted,
            size: 11,
            bold: true,
            horizontal_align: HorizontalAlign::Center,
            font: MainFontRole::UiText,
        });

        if items.is_empty() {
            text_commands.push(MainVvPopupTextCommand {
                role: MainVvPopupTextRole::Empty,
                text: strings.empty.clone(),
                rect: UiRect::new(
                    16,
                    self.header_h + 16,
                    client_rect.right - 16,
                    self.header_h + 48,
                ),
                color: MainThemeRole::TextMuted,
                size: 12,
                bold: true,
                horizontal_align: HorizontalAlign::Center,
                font: MainFontRole::UiText,
            });
        } else {
            for (row, item) in items.iter().enumerate() {
                let row_rect = self.row_rect(row);
                let bubble = UiRect::new(
                    row_rect.left,
                    row_rect.top + 4,
                    row_rect.left + 24,
                    row_rect.top + 24,
                );
                paint_commands.push(MainPaintCommand::RoundFill {
                    rect: bubble,
                    fill: MainPaintFill::Theme(MainThemeRole::Accent),
                    radius: 8,
                });
                text_commands.push(MainVvPopupTextCommand {
                    role: MainVvPopupTextRole::RowIndex,
                    text: item.index.to_string(),
                    rect: bubble,
                    color: MainThemeRole::OnAccent,
                    size: 11,
                    bold: true,
                    horizontal_align: HorizontalAlign::Center,
                    font: MainFontRole::UiText,
                });
                text_commands.push(MainVvPopupTextCommand {
                    role: MainVvPopupTextRole::RowPreview,
                    text: item.label.clone(),
                    rect: UiRect::new(
                        row_rect.left + 34,
                        row_rect.top,
                        row_rect.right,
                        row_rect.bottom,
                    ),
                    color: MainThemeRole::Text,
                    size: 12,
                    bold: false,
                    horizontal_align: HorizontalAlign::Start,
                    font: MainFontRole::UiText,
                });
            }
        }

        MainVvPopupRenderPlan {
            paint_commands,
            text_commands,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainShortcutRowCommand {
    CopySelection,
    DeleteSelection,
    TogglePin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainShortcutRowCommandPlan {
    pub(crate) context_row: i32,
    pub(crate) command: MainShortcutRowCommand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SharedTabViewState {
    pub(crate) tab_index: usize,
    pub(crate) tab_group_filters: [i64; 2],
}

impl Default for SharedTabViewState {
    fn default() -> Self {
        Self {
            tab_index: 0,
            tab_group_filters: [0, 0],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ItemsQuery {
    pub(crate) category: i64,
    pub(crate) group_id: i64,
    pub(crate) search_text: String,
}

impl ItemsQuery {
    pub(crate) fn for_tab(
        tab: usize,
        grouping_enabled: bool,
        tab_group_filters: [i64; 2],
        search_text: &str,
    ) -> Self {
        Self {
            category: tab as i64,
            group_id: if grouping_enabled {
                tab_group_filters.get(tab).copied().unwrap_or(0)
            } else {
                0
            },
            search_text: search_text.trim().to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemsCursor {
    pub(crate) pinned: bool,
    pub(crate) id: i64,
}

pub(crate) struct TabLoadState {
    pub(crate) query: Option<ItemsQuery>,
    pub(crate) next_cursor: Option<ItemsCursor>,
    pub(crate) has_more: bool,
    pub(crate) loading: bool,
    pub(crate) request_seq: u64,
    pub(crate) error: Option<String>,
}

impl Default for TabLoadState {
    fn default() -> Self {
        Self {
            query: None,
            next_cursor: None,
            has_more: true,
            loading: false,
            request_seq: 0,
            error: None,
        }
    }
}

impl TabLoadState {
    pub(crate) fn invalidate(&mut self) {
        self.request_seq = self.request_seq.wrapping_add(1);
        self.query = None;
        self.next_cursor = None;
        self.has_more = true;
        self.loading = false;
        self.error = None;
    }

    pub(crate) fn begin_request(&mut self, query: ItemsQuery, reset: bool) -> u64 {
        self.request_seq = self.request_seq.wrapping_add(1);
        self.query = Some(query);
        self.loading = true;
        self.error = None;
        if reset {
            self.next_cursor = None;
            self.has_more = true;
        }
        self.request_seq
    }

    pub(crate) fn accepts_result(&self, request_seq: u64, query: &ItemsQuery) -> bool {
        self.request_seq == request_seq && self.query.as_ref() == Some(query)
    }

    pub(crate) fn finish_request(
        &mut self,
        error: Option<String>,
        next_cursor: Option<ItemsCursor>,
        has_more: bool,
    ) {
        self.loading = false;
        self.error = error;
        self.next_cursor = next_cursor;
        self.has_more = has_more;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TitleButtonVisibility {
    pub(crate) search: bool,
    pub(crate) setting: bool,
    pub(crate) minimize: bool,
    pub(crate) close: bool,
}

impl Default for TitleButtonVisibility {
    fn default() -> Self {
        Self {
            search: true,
            setting: true,
            minimize: true,
            close: true,
        }
    }
}

impl TitleButtonVisibility {
    fn is_visible(self, key: &str) -> bool {
        match key {
            "search" => self.search,
            "setting" => self.setting,
            "min" => self.minimize,
            "close" => self.close,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainHoverTarget {
    pub(crate) title_button: &'static str,
    pub(crate) tab: i32,
    pub(crate) scrollbar: bool,
    pub(crate) scroll_to_top: bool,
    pub(crate) row: i32,
}

impl Default for MainHoverTarget {
    fn default() -> Self {
        Self {
            title_button: "",
            tab: -1,
            scrollbar: false,
            scroll_to_top: false,
            row: -1,
        }
    }
}

impl MainHoverTarget {
    pub(crate) fn clear_transition(self, clear_scrollbar: bool) -> MainHoverClearTransition {
        let next = MainHoverTarget {
            title_button: "",
            tab: -1,
            scrollbar: if clear_scrollbar {
                false
            } else {
                self.scrollbar
            },
            scroll_to_top: false,
            row: -1,
        };
        MainHoverClearTransition {
            next,
            changed: self != next,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainHoverClearTransition {
    pub(crate) next: MainHoverTarget,
    pub(crate) changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainHoverTransition {
    pub(crate) next: MainHoverTarget,
    pub(crate) target_changed: bool,
    pub(crate) row_changed: bool,
    pub(crate) show_scrollbar_feedback: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPointerMoveTransition {
    pub(crate) drag_scroll_y: Option<i32>,
    pub(crate) hover: Option<MainHoverTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowRelease {
    pub(crate) down_row: i32,
    pub(crate) release_row: i32,
    pub(crate) accepted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainPointerUpTarget {
    None,
    TitleButton { key: &'static str, activated: bool },
    ScrollToTop { activated: bool },
    Row(MainRowRelease),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPointerUpTransition {
    pub(crate) target: MainPointerUpTarget,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MainPointerModifiers {
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainRowReleaseAction {
    None,
    QuickDelete {
        row: i32,
    },
    Select {
        row: i32,
        modifiers: MainPointerModifiers,
    },
    Paste {
        row: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainPointerDownTarget {
    None,
    TitleDrag,
    TitleButton(&'static str),
    ScrollToTop,
    ScrollbarThumb,
    ScrollbarTrack,
    Tab(usize),
    Row(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainScrollDragStart {
    pub(crate) pointer_y: i32,
    pub(crate) scroll_y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainPointerDownStatePlan {
    pub(crate) down_title_button: &'static str,
    pub(crate) down_scroll_to_top: bool,
    pub(crate) down_row: i32,
    pub(crate) down_x: i32,
    pub(crate) down_y: i32,
    pub(crate) scroll_drag_start: Option<MainScrollDragStart>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainFrameHitTarget {
    Client,
    Caption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainTitleButtonRender {
    pub(crate) key: &'static str,
    pub(crate) kind: MainTitleButtonKind,
    pub(crate) rect: UiRect,
    pub(crate) state: MainControlVisualState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainTitleButtonKind {
    Search,
    Settings,
    Minimize,
    Close,
}

impl MainTitleButtonKind {
    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "search" => Self::Search,
            "setting" => Self::Settings,
            "min" => Self::Minimize,
            _ => Self::Close,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MainControlVisualState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainRowBackground {
    Selected,
    Hovered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainThemeRole {
    Background,
    Surface,
    Surface2,
    Stroke,
    SegmentSelected,
    ControlBg,
    ControlStroke,
    ButtonHover,
    ButtonPressed,
    CloseHover,
    ItemSelected,
    ItemHovered,
    Accent,
    OnAccent,
    Text,
    TextMuted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainPaintFill {
    Theme(MainThemeRole),
    ScrollbarThumb { alpha: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainPaintCommand {
    FillRect {
        rect: UiRect,
        fill: MainPaintFill,
    },
    RoundRect {
        rect: UiRect,
        fill: MainPaintFill,
        stroke: Option<MainThemeRole>,
        radius: i32,
    },
    RoundFill {
        rect: UiRect,
        fill: MainPaintFill,
        radius: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainTextRole {
    SegmentRecords,
    SegmentPhrases,
    EmptyLoading,
    EmptyError,
    EmptyGroup,
    EmptyRecords,
    EmptyPhrases,
    LoadingFooter,
    ScrollToTopArrow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainFontRole {
    UiText,
    Display,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainTextLayer {
    Content,
    Overlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainTextCommand {
    pub(crate) role: MainTextRole,
    pub(crate) layer: MainTextLayer,
    pub(crate) rect: UiRect,
    pub(crate) color: MainThemeRole,
    pub(crate) size: i32,
    pub(crate) bold: bool,
    pub(crate) horizontal_align: HorizontalAlign,
    pub(crate) vertical_align: VerticalAlign,
    pub(crate) wrap: TextWrap,
    pub(crate) font: MainFontRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainSearchHostPlan {
    pub(crate) visible: bool,
    pub(crate) outer_rect: UiRect,
    pub(crate) input_rect: UiRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainIconKind {
    App,
    Search,
    Settings,
    Minimize,
    Close,
    Text,
    Image,
    File,
    Folder,
    Pin,
    Delete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainIconColorMode {
    ThemeAware,
    Original,
}

impl MainIconKind {
    pub(crate) fn for_clip_item(kind: ClipKind, is_directory: bool) -> Self {
        match kind {
            ClipKind::Text | ClipKind::Phrase => Self::Text,
            ClipKind::Image => Self::Image,
            ClipKind::Files => {
                if is_directory {
                    Self::Folder
                } else {
                    Self::File
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainIconCommand {
    pub(crate) kind: MainIconKind,
    pub(crate) rect: UiRect,
    pub(crate) color_mode: MainIconColorMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowRender {
    pub(crate) index: i32,
    pub(crate) rect: UiRect,
    pub(crate) icon_rect: Option<UiRect>,
    pub(crate) item_icon_command: Option<MainIconCommand>,
    pub(crate) pin_rect: Option<UiRect>,
    pub(crate) selected: bool,
    pub(crate) hovered: bool,
    pub(crate) background: Option<MainRowBackground>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MainRowContentInput {
    pub(crate) pinned: bool,
    pub(crate) show_delete: bool,
    pub(crate) show_preview: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainRowTextCommand {
    pub(crate) rect: UiRect,
    pub(crate) color: MainThemeRole,
    pub(crate) size: i32,
    pub(crate) bold: bool,
    pub(crate) horizontal_align: HorizontalAlign,
    pub(crate) vertical_align: VerticalAlign,
    pub(crate) wrap: TextWrap,
    pub(crate) font: MainFontRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainRowContentPlan {
    pub(crate) text_rect: UiRect,
    pub(crate) text_command: MainRowTextCommand,
    pub(crate) delete_rect: Option<UiRect>,
    pub(crate) preview_rect: Option<UiRect>,
    pub(crate) paint_commands: Vec<MainPaintCommand>,
    pub(crate) icon_commands: Vec<MainIconCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainRenderPlan {
    pub(crate) chrome_commands: Vec<MainPaintCommand>,
    pub(crate) title_buttons: Vec<MainTitleButtonRender>,
    pub(crate) search_rect: Option<UiRect>,
    pub(crate) search_host: MainSearchHostPlan,
    pub(crate) segment_rect: UiRect,
    pub(crate) tab_rects: [UiRect; 2],
    pub(crate) segment_commands: Vec<MainPaintCommand>,
    pub(crate) segment_text_commands: Vec<MainTextCommand>,
    pub(crate) list_rect: UiRect,
    pub(crate) list_clip_rect: UiRect,
    pub(crate) empty_state_rect: Option<UiRect>,
    pub(crate) row_background_commands: Vec<MainPaintCommand>,
    pub(crate) visible_rows: Vec<MainRowRender>,
    pub(crate) loading_footer_rect: Option<UiRect>,
    pub(crate) scrollbar_thumb_rect: Option<UiRect>,
    pub(crate) scroll_to_top_rect: Option<UiRect>,
    pub(crate) overlay_commands: Vec<MainPaintCommand>,
    pub(crate) text_commands: Vec<MainTextCommand>,
    pub(crate) icon_commands: Vec<MainIconCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MainRenderInput {
    pub(crate) client_rect: UiRect,
    pub(crate) visible_len: usize,
    pub(crate) scroll_y: i32,
    pub(crate) empty_state: MainEmptyStateKind,
    pub(crate) hover_idx: i32,
    pub(crate) sel_idx: i32,
    pub(crate) selected_rows: Vec<i32>,
    pub(crate) row_icon_kinds: Vec<MainIconKind>,
    pub(crate) tab_index: i32,
    pub(crate) hover_tab: i32,
    pub(crate) hover_title_button: &'static str,
    pub(crate) down_title_button: &'static str,
    pub(crate) search_on: bool,
    pub(crate) active_loading: bool,
    pub(crate) scroll_fade_alpha: u8,
    pub(crate) hover_scroll: bool,
    pub(crate) scroll_to_top_visible: bool,
    pub(crate) hover_scroll_to_top: bool,
    pub(crate) down_scroll_to_top: bool,
    pub(crate) title_buttons: TitleButtonVisibility,
}

impl MainRenderInput {
    pub(crate) fn empty_records(client_rect: UiRect) -> Self {
        Self {
            client_rect,
            visible_len: 0,
            scroll_y: 0,
            empty_state: MainEmptyStateKind::Records,
            hover_idx: -1,
            sel_idx: -1,
            selected_rows: Vec::new(),
            row_icon_kinds: Vec::new(),
            tab_index: 0,
            hover_tab: -1,
            hover_title_button: "",
            down_title_button: "",
            search_on: false,
            active_loading: false,
            scroll_fade_alpha: 0,
            hover_scroll: false,
            scroll_to_top_visible: false,
            hover_scroll_to_top: false,
            down_scroll_to_top: false,
            title_buttons: TitleButtonVisibility::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainScrollUpdate {
    pub(crate) scroll_y: i32,
    pub(crate) changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainEmptyStateKind {
    Loading,
    Error,
    Group,
    Records,
    Phrases,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MainUiLayout {
    pub(crate) win_w: i32,
    pub(crate) title_h: i32,
    pub(crate) seg_x: i32,
    pub(crate) seg_y: i32,
    pub(crate) seg_w: i32,
    pub(crate) seg_h: i32,
    pub(crate) list_x: i32,
    pub(crate) list_y: i32,
    pub(crate) list_w: i32,
    pub(crate) list_h: i32,
    pub(crate) list_pad: i32,
    pub(crate) row_h: i32,
    pub(crate) btn_w: i32,
    pub(crate) btn_gap: i32,
    pub(crate) search_left: i32,
    pub(crate) search_top: i32,
    pub(crate) search_w: i32,
    pub(crate) search_h: i32,
}

impl MainUiLayout {
    pub(crate) const fn zsclip() -> Self {
        Self {
            win_w: 300,
            title_h: 35,
            seg_x: 6,
            seg_y: 36,
            seg_w: 288,
            seg_h: 30,
            list_x: 6,
            list_y: 70,
            list_w: 288,
            list_h: 538,
            list_pad: 4,
            row_h: 44,
            btn_w: 32,
            btn_gap: 2,
            search_left: 58,
            search_top: 4,
            search_w: 112,
            search_h: 30,
        }
    }

    pub(crate) fn scaled(self, dpi: u32) -> Self {
        fn scale(value: i32, dpi: u32) -> i32 {
            let dpi = (dpi.max(96)) as i32;
            ((value * dpi) + 48) / 96
        }
        Self {
            win_w: scale(self.win_w, dpi),
            title_h: scale(self.title_h, dpi),
            seg_x: scale(self.seg_x, dpi),
            seg_y: scale(self.seg_y, dpi),
            seg_w: scale(self.seg_w, dpi),
            seg_h: scale(self.seg_h, dpi),
            list_x: scale(self.list_x, dpi),
            list_y: scale(self.list_y, dpi),
            list_w: scale(self.list_w, dpi),
            list_h: scale(self.list_h, dpi),
            list_pad: scale(self.list_pad, dpi),
            row_h: scale(self.row_h, dpi),
            btn_w: scale(self.btn_w, dpi),
            btn_gap: scale(self.btn_gap, dpi),
            search_left: scale(self.search_left, dpi),
            search_top: scale(self.search_top, dpi),
            search_w: scale(self.search_w, dpi),
            search_h: scale(self.search_h, dpi),
        }
    }

    pub(crate) fn list_view_height(self) -> i32 {
        self.list_h - 2 * self.list_pad
    }

    pub(crate) fn row_text_size(self) -> i32 {
        ((self.row_h * 12) / 44).clamp(12, 16)
    }

    pub(crate) fn row_muted_text_size(self) -> i32 {
        (self.row_text_size() - 1).max(11)
    }

    pub(crate) fn total_content_height(self, filtered_len: usize) -> i32 {
        filtered_len as i32 * self.row_h
    }

    pub(crate) fn max_scroll(self, filtered_len: usize) -> i32 {
        (self.total_content_height(filtered_len) - self.list_view_height()).max(0)
    }

    pub(crate) fn clamp_scroll(self, scroll_y: i32, filtered_len: usize) -> i32 {
        scroll_y.clamp(0, self.max_scroll(filtered_len))
    }

    pub(crate) fn scroll_update_for_target(
        self,
        current_scroll_y: i32,
        filtered_len: usize,
        target_scroll_y: i32,
    ) -> MainScrollUpdate {
        let scroll_y = self.clamp_scroll(target_scroll_y, filtered_len);
        MainScrollUpdate {
            scroll_y,
            changed: scroll_y != current_scroll_y,
        }
    }

    pub(crate) fn scroll_update_for_wheel(
        self,
        current_scroll_y: i32,
        filtered_len: usize,
        wheel_delta: i32,
    ) -> MainScrollUpdate {
        self.scroll_update_for_target(
            current_scroll_y,
            filtered_len,
            self.wheel_scroll_target(current_scroll_y, filtered_len, wheel_delta),
        )
    }

    pub(crate) fn wheel_scroll_target(
        self,
        scroll_y: i32,
        filtered_len: usize,
        wheel_delta: i32,
    ) -> i32 {
        let scroll_step = (self.row_h * 2).max(32);
        let next = if wheel_delta > 0 {
            scroll_y - scroll_step
        } else {
            scroll_y + scroll_step
        };
        self.clamp_scroll(next, filtered_len)
    }

    pub(crate) fn ensure_visible(self, scroll_y: i32, idx: i32, filtered_len: usize) -> i32 {
        if idx < 0 {
            return self.clamp_scroll(scroll_y, filtered_len);
        }
        let top = idx * self.row_h;
        let bottom = top + self.row_h;
        let view_top = scroll_y;
        let view_bottom = scroll_y + self.list_view_height();
        let next = if top < view_top {
            top
        } else if bottom > view_bottom {
            bottom - self.list_view_height()
        } else {
            scroll_y
        };
        self.clamp_scroll(next, filtered_len)
    }

    pub(crate) fn row_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
    ) -> Option<UiRect> {
        if visible_idx < 0 || visible_idx >= filtered_len as i32 {
            return None;
        }
        let inner_l = self.list_x + self.list_pad;
        let inner_t = self.list_y + self.list_pad;
        let y0 = inner_t + visible_idx * self.row_h - scroll_y;
        Some(UiRect::new(
            inner_l,
            y0,
            inner_l + self.list_w - 2 * self.list_pad,
            y0 + self.row_h,
        ))
    }

    pub(crate) fn list_inner_rect(self) -> UiRect {
        UiRect::new(
            self.list_x + self.list_pad,
            self.list_y + self.list_pad,
            self.list_x + self.list_w - self.list_pad,
            self.list_y + self.list_h - self.list_pad,
        )
    }

    pub(crate) fn hit_test_row(self, x: i32, y: i32, filtered_len: usize, scroll_y: i32) -> i32 {
        let inner = self.list_inner_rect();
        if !inner.contains(x, y) {
            return -1;
        }
        let yy = y - inner.top + scroll_y;
        let idx = yy / self.row_h;
        if idx < 0 || idx >= filtered_len as i32 {
            -1
        } else {
            idx
        }
    }

    pub(crate) fn quick_action_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
        slot: i32,
    ) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = (self.row_h * 16 / 44).max(16);
        let gap = (self.row_h * 8 / 44).max(8);
        let right_pad = (self.row_h * 10 / 44).max(10);
        let icon_offset = (self.row_h * 12 / 44).max(12);
        let left = row.right - right_pad - size - icon_offset - slot.max(0) * (size + gap);
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    pub(crate) fn row_icon_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
    ) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = (self.row_h * 20 / 44).max(18);
        let left_pad = (self.row_h * 12 / 44).max(12);
        let left = row.left + left_pad;
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    pub(crate) fn row_pin_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
    ) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = (self.row_h * 16 / 44).max(16);
        let gap = (self.row_h * 6 / 44).max(6);
        let left = if let Some(icon) = self.row_icon_rect(visible_idx, filtered_len, scroll_y) {
            icon.right + gap
        } else {
            row.left + (self.row_h * 32 / 44).max(24)
        };
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    fn quick_action_rect_for_row(self, row: UiRect, slot: i32) -> UiRect {
        let size = (self.row_h * 16 / 44).max(16);
        let gap = (self.row_h * 8 / 44).max(8);
        let right_pad = (self.row_h * 10 / 44).max(10);
        let icon_offset = (self.row_h * 12 / 44).max(12);
        let left = row.right - right_pad - size - icon_offset - slot.max(0) * (size + gap);
        let top = row.top + (self.row_h - size) / 2;
        UiRect::new(left, top, left + size, top + size)
    }

    fn title_button_icon_rect(self, rect: UiRect) -> UiRect {
        let slot = rect.width();
        let min_size = (self.title_h * 18 / 35).max(18);
        let iw = ((slot * 18 / 36).max(min_size)).min((slot - 4).max(min_size));
        let ix = rect.left + (rect.width() - iw) / 2;
        let iy = rect.top + (rect.height() - iw) / 2;
        UiRect::new(ix, iy, ix + iw, iy + iw)
    }

    pub(crate) fn app_icon_rect(self) -> UiRect {
        let min_pad = (self.title_h * 6 / 35).max(6);
        let min_size = (self.title_h * 20 / 35).max(20);
        let size = ((self.title_h * 24) / 35)
            .max(min_size)
            .min((self.title_h - min_pad * 2).max(min_size));
        let x = ((self.title_h - size) / 2).max(min_pad);
        let y = ((self.title_h - size) / 2).max(min_pad.saturating_sub(1));
        UiRect::new(x, y, x + size, y + size)
    }

    pub(crate) fn row_content_plan(
        self,
        row: &MainRowRender,
        input: MainRowContentInput,
    ) -> MainRowContentPlan {
        let mut text_rect = row.rect;
        text_rect.left += (self.row_h * 12 / 44).clamp(10, 20);
        if let Some(icon) = row.icon_rect {
            text_rect.left = text_rect
                .left
                .max(icon.right + (self.row_h * 12 / 44).clamp(10, 18));
        }
        if input.pinned {
            if let Some(pin) = row.pin_rect {
                text_rect.left = text_rect
                    .left
                    .max(pin.right + (self.row_h * 8 / 44).clamp(8, 16));
            }
        }

        let delete_rect = input
            .show_delete
            .then_some(self.quick_action_rect_for_row(row.rect, 0));
        let mut paint_commands = Vec::new();
        let mut icon_commands = Vec::new();
        if input.pinned {
            if let Some(rect) = row.pin_rect {
                icon_commands.push(MainIconCommand {
                    kind: MainIconKind::Pin,
                    rect,
                    color_mode: MainIconColorMode::ThemeAware,
                });
            }
        }
        if let Some(rect) = delete_rect {
            icon_commands.push(MainIconCommand {
                kind: MainIconKind::Delete,
                rect,
                color_mode: MainIconColorMode::ThemeAware,
            });
            paint_commands.push(MainPaintCommand::RoundRect {
                rect: rect.inflate(2, 2),
                fill: MainPaintFill::Theme(MainThemeRole::Surface),
                stroke: Some(MainThemeRole::Stroke),
                radius: 10,
            });
        }
        text_rect.right -= if input.show_delete {
            (self.row_h * 42 / 44).max(42)
        } else {
            (self.row_h * 18 / 44).max(18)
        };

        let preview_rect = if input.show_preview {
            let size = (text_rect.height() - 8).max(24);
            let left = text_rect.left + 2;
            let top = text_rect.top + (text_rect.height() - size) / 2;
            let preview = UiRect::new(left, top, left + size, top + size);
            text_rect.left = preview.right + (self.row_h * 10 / 44).clamp(10, 16);
            paint_commands.push(MainPaintCommand::RoundRect {
                rect: preview.inflate(2, 2),
                fill: MainPaintFill::Theme(MainThemeRole::Surface2),
                stroke: Some(MainThemeRole::Stroke),
                radius: 8,
            });
            Some(preview)
        } else {
            None
        };

        MainRowContentPlan {
            text_rect,
            text_command: MainRowTextCommand {
                rect: text_rect,
                color: MainThemeRole::Text,
                size: self.row_text_size(),
                bold: false,
                horizontal_align: HorizontalAlign::Start,
                vertical_align: VerticalAlign::Center,
                wrap: TextWrap::NoWrap,
                font: MainFontRole::UiText,
            },
            delete_rect,
            preview_rect,
            paint_commands,
            icon_commands,
        }
    }

    pub(crate) fn scroll_to_top_button_rect(self) -> UiRect {
        let size = (self.row_h * 36 / 44).max(32);
        let margin = (self.row_h * 10 / 44).max(10);
        let bottom = self.list_y + self.list_h - self.list_pad - margin;
        UiRect::new(
            self.list_x + self.list_w - self.list_pad - size - margin,
            bottom - size,
            self.list_x + self.list_w - self.list_pad - margin,
            bottom,
        )
    }

    pub(crate) fn search_rect(self) -> UiRect {
        UiRect::new(
            self.search_left,
            self.search_top,
            self.search_left + self.search_w,
            self.search_top + self.search_h,
        )
    }

    pub(crate) fn search_host_plan(self, visible: bool) -> MainSearchHostPlan {
        let outer_rect = self.search_rect();
        MainSearchHostPlan {
            visible,
            outer_rect,
            input_rect: UiRect::new(
                outer_rect.left + 10,
                outer_rect.top + 5,
                outer_rect.right - 10,
                outer_rect.bottom - 5,
            ),
        }
    }

    pub(crate) fn title_button_rect(self, key: &str) -> UiRect {
        let x_close = self.win_w - 4 - self.btn_w;
        let x_min = x_close - self.btn_gap - self.btn_w;
        let x_set = x_min - self.btn_gap - self.btn_w;
        let x_search = x_set - self.btn_gap - self.btn_w;
        let x = match key {
            "search" => x_search,
            "setting" => x_set,
            "min" => x_min,
            _ => x_close,
        };
        let top = (self.title_h - self.btn_w) / 2;
        UiRect::new(x, top, x + self.btn_w, top + self.btn_w)
    }

    pub(crate) fn segment_rects(self) -> (UiRect, UiRect) {
        let inner_l = self.seg_x + 1;
        let inner_t = self.seg_y + 1;
        let inner_w = self.seg_w - 2;
        let inner_h = self.seg_h - 2;
        let gap = 1;
        let btn_w = (inner_w - gap) / 2;
        (
            UiRect::new(inner_l, inner_t, inner_l + btn_w, inner_t + inner_h),
            UiRect::new(
                inner_l + btn_w + gap,
                inner_t,
                inner_l + inner_w,
                inner_t + inner_h,
            ),
        )
    }

    pub(crate) fn segment_rect(self) -> UiRect {
        UiRect::new(
            self.seg_x,
            self.seg_y,
            self.seg_x + self.seg_w,
            self.seg_y + self.seg_h,
        )
    }

    pub(crate) fn list_rect(self) -> UiRect {
        UiRect::new(
            self.list_x,
            self.list_y,
            self.list_x + self.list_w,
            self.list_y + self.list_h,
        )
    }

    pub(crate) fn render_plan(self, input: MainRenderInput) -> MainRenderPlan {
        let title_buttons: Vec<MainTitleButtonRender> = ["search", "setting", "min", "close"]
            .into_iter()
            .filter(|key| input.title_buttons.is_visible(key))
            .map(|key| {
                let state = if input.down_title_button == key {
                    MainControlVisualState::Pressed
                } else if input.hover_title_button == key {
                    MainControlVisualState::Hovered
                } else {
                    MainControlVisualState::Normal
                };
                MainTitleButtonRender {
                    key,
                    kind: MainTitleButtonKind::from_key(key),
                    rect: self.title_button_rect(key),
                    state,
                }
            })
            .collect();
        let mut chrome_commands = vec![MainPaintCommand::FillRect {
            rect: input.client_rect,
            fill: MainPaintFill::Theme(MainThemeRole::Background),
        }];
        let mut icon_commands = vec![MainIconCommand {
            kind: MainIconKind::App,
            rect: self.app_icon_rect(),
            color_mode: MainIconColorMode::Original,
        }];
        for button in &title_buttons {
            match (button.kind, button.state) {
                (MainTitleButtonKind::Close, MainControlVisualState::Pressed)
                | (MainTitleButtonKind::Close, MainControlVisualState::Hovered) => {
                    chrome_commands.push(MainPaintCommand::FillRect {
                        rect: button.rect,
                        fill: MainPaintFill::Theme(MainThemeRole::CloseHover),
                    });
                }
                (_, MainControlVisualState::Pressed) => {
                    chrome_commands.push(MainPaintCommand::RoundRect {
                        rect: button.rect.inflate(-2, -2),
                        fill: MainPaintFill::Theme(MainThemeRole::ButtonPressed),
                        stroke: None,
                        radius: 6,
                    });
                }
                (_, MainControlVisualState::Hovered) => {
                    chrome_commands.push(MainPaintCommand::RoundRect {
                        rect: button.rect.inflate(-2, -2),
                        fill: MainPaintFill::Theme(MainThemeRole::ButtonHover),
                        stroke: None,
                        radius: 6,
                    });
                }
                (_, MainControlVisualState::Normal) => {}
            }
            let kind = match button.kind {
                MainTitleButtonKind::Search => MainIconKind::Search,
                MainTitleButtonKind::Settings => MainIconKind::Settings,
                MainTitleButtonKind::Minimize => MainIconKind::Minimize,
                MainTitleButtonKind::Close => MainIconKind::Close,
            };
            icon_commands.push(MainIconCommand {
                kind,
                rect: self.title_button_icon_rect(button.rect),
                color_mode: MainIconColorMode::ThemeAware,
            });
        }
        let (tab0, tab1) = self.segment_rects();
        let segment_rect = self.segment_rect();
        let selected_tab = if input.tab_index == 1 { 1 } else { 0 };
        let tab_rects = [tab0, tab1];
        let mut segment_commands = vec![MainPaintCommand::RoundRect {
            rect: segment_rect,
            fill: MainPaintFill::Theme(MainThemeRole::Surface),
            stroke: Some(MainThemeRole::Stroke),
            radius: 4,
        }];
        segment_commands.push(MainPaintCommand::RoundRect {
            rect: tab_rects[selected_tab as usize].inflate(-2, -2),
            fill: MainPaintFill::Theme(MainThemeRole::SegmentSelected),
            stroke: Some(MainThemeRole::Stroke),
            radius: 3,
        });
        if (input.hover_tab == 0 || input.hover_tab == 1) && input.hover_tab != selected_tab {
            segment_commands.push(MainPaintCommand::RoundFill {
                rect: tab_rects[input.hover_tab as usize].inflate(-2, -2),
                fill: MainPaintFill::Theme(MainThemeRole::ItemHovered),
                radius: 3,
            });
        }
        let segment_text_size = (segment_rect.height() * 13 / 30).clamp(12, 16);
        let segment_text_commands = [
            (0, MainTextRole::SegmentRecords, tab0),
            (1, MainTextRole::SegmentPhrases, tab1),
        ]
        .into_iter()
        .map(|(index, role, rect)| MainTextCommand {
            role,
            layer: MainTextLayer::Content,
            rect,
            color: if selected_tab == index || input.hover_tab == index {
                MainThemeRole::Text
            } else {
                MainThemeRole::TextMuted
            },
            size: segment_text_size,
            bold: false,
            horizontal_align: HorizontalAlign::Center,
            vertical_align: VerticalAlign::Center,
            wrap: TextWrap::NoWrap,
            font: MainFontRole::Display,
        })
        .collect();
        let list_clip_rect = UiRect::new(
            self.list_x + 1,
            self.list_y + 1,
            self.list_x + self.list_w - 1,
            self.list_y + self.list_h - 1,
        );
        let empty_state_rect = if input.visible_len == 0 {
            Some(UiRect::new(
                self.list_x + 20,
                self.list_y + 20,
                self.list_x + self.list_w - 20,
                self.list_y + self.list_h - 20,
            ))
        } else {
            None
        };
        if let Some(search_rect) = input.search_on.then_some(self.search_rect()) {
            chrome_commands.push(MainPaintCommand::RoundRect {
                rect: search_rect,
                fill: MainPaintFill::Theme(MainThemeRole::ControlBg),
                stroke: Some(MainThemeRole::ControlStroke),
                radius: 10,
            });
        }
        chrome_commands.push(MainPaintCommand::RoundRect {
            rect: self.list_rect(),
            fill: MainPaintFill::Theme(MainThemeRole::Surface),
            stroke: Some(MainThemeRole::Stroke),
            radius: 10,
        });

        let mut visible_rows = Vec::new();
        let mut row_background_commands = Vec::new();
        if input.visible_len > 0 {
            let view_top = self.list_y + self.list_pad;
            let view_bottom = self.list_y + self.list_h - self.list_pad;
            let start_idx = (input.scroll_y / self.row_h).max(0);
            let end_idx = (input.visible_len as i32)
                .min((input.scroll_y + self.list_view_height()) / self.row_h + 2);
            for index in start_idx..end_idx {
                let Some(rect) = self.row_rect(index, input.visible_len, input.scroll_y) else {
                    continue;
                };
                if rect.bottom <= view_top || rect.top >= view_bottom {
                    continue;
                }
                let selected =
                    index >= 0 && (input.sel_idx == index || input.selected_rows.contains(&index));
                let hovered = input.hover_idx == index;
                let background = if selected {
                    Some(MainRowBackground::Selected)
                } else if hovered {
                    Some(MainRowBackground::Hovered)
                } else {
                    None
                };
                if let Some(background) = background {
                    let fill = match background {
                        MainRowBackground::Selected => MainThemeRole::ItemSelected,
                        MainRowBackground::Hovered => MainThemeRole::ItemHovered,
                    };
                    row_background_commands.push(MainPaintCommand::FillRect {
                        rect,
                        fill: MainPaintFill::Theme(fill),
                    });
                }
                visible_rows.push(MainRowRender {
                    index,
                    rect,
                    icon_rect: self.row_icon_rect(index, input.visible_len, input.scroll_y),
                    item_icon_command: input.row_icon_kinds.get(index as usize).and_then(|kind| {
                        self.row_icon_rect(index, input.visible_len, input.scroll_y)
                            .map(|rect| MainIconCommand {
                                kind: *kind,
                                rect,
                                color_mode: MainIconColorMode::ThemeAware,
                            })
                    }),
                    pin_rect: self.row_pin_rect(index, input.visible_len, input.scroll_y),
                    selected,
                    hovered,
                    background,
                });
            }
        }

        let loading_footer_rect = if input.visible_len > 0 && input.active_loading {
            Some(UiRect::new(
                self.list_x + 18,
                self.list_y + self.list_h - (self.row_h * 36 / 44).clamp(28, 44),
                self.list_x + self.list_w - 18,
                self.list_y + self.list_h - 12,
            ))
        } else {
            None
        };
        let mut text_commands = Vec::new();
        if let Some(rect) = empty_state_rect {
            let role = match input.empty_state {
                MainEmptyStateKind::Loading => MainTextRole::EmptyLoading,
                MainEmptyStateKind::Error => MainTextRole::EmptyError,
                MainEmptyStateKind::Group => MainTextRole::EmptyGroup,
                MainEmptyStateKind::Records => MainTextRole::EmptyRecords,
                MainEmptyStateKind::Phrases => MainTextRole::EmptyPhrases,
            };
            text_commands.push(MainTextCommand {
                role,
                layer: MainTextLayer::Content,
                rect,
                color: MainThemeRole::TextMuted,
                size: self.row_muted_text_size(),
                bold: false,
                horizontal_align: HorizontalAlign::Center,
                vertical_align: VerticalAlign::Center,
                wrap: TextWrap::NoWrap,
                font: MainFontRole::UiText,
            });
        }
        if let Some(rect) = loading_footer_rect {
            text_commands.push(MainTextCommand {
                role: MainTextRole::LoadingFooter,
                layer: MainTextLayer::Content,
                rect,
                color: MainThemeRole::TextMuted,
                size: self.row_muted_text_size(),
                bold: false,
                horizontal_align: HorizontalAlign::Center,
                vertical_align: VerticalAlign::Center,
                wrap: TextWrap::NoWrap,
                font: MainFontRole::UiText,
            });
        }

        let scrollbar_thumb_rect = if input.visible_len > 0
            && input.scroll_fade_alpha > 0
            && self.total_content_height(input.visible_len) > self.list_view_height()
        {
            self.scrollbar_thumb_rect(input.visible_len, input.scroll_y)
                .map(|thumb| {
                    let thumb_w = if input.hover_scroll { 6 } else { 4 };
                    UiRect::new(thumb.right - thumb_w, thumb.top, thumb.right, thumb.bottom)
                })
        } else {
            None
        };
        let mut overlay_commands = Vec::new();
        if let Some(thumb) = scrollbar_thumb_rect {
            overlay_commands.push(MainPaintCommand::RoundFill {
                rect: thumb,
                fill: MainPaintFill::ScrollbarThumb {
                    alpha: input.scroll_fade_alpha,
                },
                radius: 3,
            });
        }
        let scroll_to_top_rect = input
            .scroll_to_top_visible
            .then_some(self.scroll_to_top_button_rect());
        if let Some(rect) = scroll_to_top_rect {
            let fill = if input.down_scroll_to_top {
                MainThemeRole::ButtonPressed
            } else if input.hover_scroll_to_top {
                MainThemeRole::ButtonHover
            } else {
                MainThemeRole::Surface
            };
            overlay_commands.push(MainPaintCommand::RoundRect {
                rect,
                fill: MainPaintFill::Theme(fill),
                stroke: Some(MainThemeRole::Stroke),
                radius: 10,
            });
            text_commands.push(MainTextCommand {
                role: MainTextRole::ScrollToTopArrow,
                layer: MainTextLayer::Overlay,
                rect,
                color: MainThemeRole::Text,
                size: 18,
                bold: true,
                horizontal_align: HorizontalAlign::Center,
                vertical_align: VerticalAlign::Center,
                wrap: TextWrap::NoWrap,
                font: MainFontRole::Display,
            });
        }

        MainRenderPlan {
            chrome_commands,
            title_buttons,
            search_rect: input.search_on.then_some(self.search_rect()),
            search_host: self.search_host_plan(input.search_on),
            segment_rect,
            tab_rects,
            segment_commands,
            segment_text_commands,
            list_rect: self.list_rect(),
            list_clip_rect,
            empty_state_rect,
            row_background_commands,
            visible_rows,
            loading_footer_rect,
            scrollbar_thumb_rect,
            scroll_to_top_rect,
            overlay_commands,
            text_commands,
            icon_commands,
        }
    }

    pub(crate) fn scrollbar_track_rect(self, filtered_len: usize) -> Option<UiRect> {
        if self.total_content_height(filtered_len) <= self.list_view_height() {
            return None;
        }
        let track_gap = (self.row_h * 8 / 44).max(8);
        let track_w = (self.row_h * 6 / 44).max(4);
        Some(UiRect::new(
            self.list_x + self.list_w - self.list_pad - track_gap - 2,
            self.list_y + self.list_pad + 2,
            self.list_x + self.list_w - self.list_pad - 2 + (track_w - 6).max(0),
            self.list_y + self.list_h - self.list_pad - 2,
        ))
    }

    pub(crate) fn scrollbar_thumb_rect(self, filtered_len: usize, scroll_y: i32) -> Option<UiRect> {
        let track = self.scrollbar_track_rect(filtered_len)?;
        let track_h = track.bottom - track.top;
        let total_h = self.total_content_height(filtered_len);
        let view_h = self.list_view_height();
        let thumb_h = ((track_h as f32) * (view_h as f32 / total_h as f32)) as i32;
        let thumb_h = thumb_h.max(28);
        let max_scroll = self.max_scroll(filtered_len).max(1);
        let thumb_y =
            track.top + ((track_h - thumb_h) as f32 * (scroll_y as f32 / max_scroll as f32)) as i32;
        Some(UiRect::new(
            track.left + 1,
            thumb_y,
            track.right - 1,
            thumb_y + thumb_h,
        ))
    }

    pub(crate) fn scrollbar_track_click_scroll_target(
        self,
        filtered_len: usize,
        pointer_y: i32,
    ) -> Option<i32> {
        let track = self.scrollbar_track_rect(filtered_len)?;
        let thumb = self.scrollbar_thumb_rect(filtered_len, 0)?;
        let track_h = (track.bottom - track.top).max(1);
        let thumb_h = (thumb.bottom - thumb.top).max(1);
        let max_scroll = self.max_scroll(filtered_len).max(0);
        if max_scroll <= 0 {
            return Some(0);
        }
        let drag_range = (track_h - thumb_h).max(1);
        let pos = (pointer_y - track.top - (thumb_h / 2)).clamp(0, track_h - thumb_h);
        Some(((pos as f32 / drag_range as f32) * max_scroll as f32) as i32)
    }

    pub(crate) fn scroll_update_for_track_click(
        self,
        current_scroll_y: i32,
        filtered_len: usize,
        pointer_y: i32,
    ) -> Option<MainScrollUpdate> {
        self.scrollbar_track_click_scroll_target(filtered_len, pointer_y)
            .map(|target| self.scroll_update_for_target(current_scroll_y, filtered_len, target))
    }

    pub(crate) fn scrollbar_drag_scroll_target(
        self,
        filtered_len: usize,
        drag_start_y: i32,
        drag_start_scroll: i32,
        pointer_y: i32,
    ) -> Option<i32> {
        let track = self.scrollbar_track_rect(filtered_len)?;
        let thumb = self.scrollbar_thumb_rect(filtered_len, drag_start_scroll)?;
        let track_h = (track.bottom - track.top).max(1);
        let thumb_h = (thumb.bottom - thumb.top).max(1);
        let max_scroll = self.max_scroll(filtered_len).max(0);
        if max_scroll <= 0 {
            return Some(0);
        }
        let drag_range = (track_h - thumb_h).max(1);
        let dy = pointer_y - drag_start_y;
        let next = drag_start_scroll + ((dy as f32 / drag_range as f32) * max_scroll as f32) as i32;
        Some(next.clamp(0, max_scroll))
    }

    pub(crate) fn hover_target(
        self,
        x: i32,
        y: i32,
        filtered_len: usize,
        scroll_y: i32,
        title_buttons: TitleButtonVisibility,
        scroll_to_top_visible: bool,
    ) -> MainHoverTarget {
        let mut target = MainHoverTarget {
            tab: -1,
            row: -1,
            ..MainHoverTarget::default()
        };

        for key in ["search", "setting", "min", "close"] {
            if title_buttons.is_visible(key) && self.title_button_rect(key).contains(x, y) {
                target.title_button = key;
                break;
            }
        }

        let (tab0, tab1) = self.segment_rects();
        target.tab = if tab0.contains(x, y) {
            0
        } else if tab1.contains(x, y) {
            1
        } else {
            -1
        };

        target.scrollbar = self
            .scrollbar_track_rect(filtered_len)
            .map(|tr| UiRect::new(tr.left - 8, tr.top, tr.right + 2, tr.bottom).contains(x, y))
            .unwrap_or(false);
        target.scroll_to_top =
            scroll_to_top_visible && self.scroll_to_top_button_rect().contains(x, y);
        target.row = if target.scroll_to_top {
            -1
        } else {
            self.hit_test_row(x, y, filtered_len, scroll_y)
        };

        target
    }

    pub(crate) fn pointer_move_transition(
        self,
        x: i32,
        y: i32,
        filtered_len: usize,
        scroll_y: i32,
        title_buttons: TitleButtonVisibility,
        scroll_to_top_visible: bool,
        current_hover: MainHoverTarget,
        scroll_dragging: bool,
        drag_start_y: i32,
        drag_start_scroll: i32,
    ) -> MainPointerMoveTransition {
        if scroll_dragging {
            return MainPointerMoveTransition {
                drag_scroll_y: self.scrollbar_drag_scroll_target(
                    filtered_len,
                    drag_start_y,
                    drag_start_scroll,
                    y,
                ),
                hover: None,
            };
        }

        let next = self.hover_target(
            x,
            y,
            filtered_len,
            scroll_y,
            title_buttons,
            scroll_to_top_visible,
        );
        MainPointerMoveTransition {
            drag_scroll_y: None,
            hover: Some(MainHoverTransition {
                next,
                target_changed: current_hover != next,
                row_changed: current_hover.row != next.row,
                show_scrollbar_feedback: next.scrollbar && !current_hover.scrollbar,
            }),
        }
    }

    pub(crate) fn pointer_down_target(
        self,
        x: i32,
        y: i32,
        filtered_len: usize,
        scroll_y: i32,
        title_buttons: TitleButtonVisibility,
        search_on: bool,
        scroll_to_top_visible: bool,
    ) -> MainPointerDownTarget {
        if (0..self.title_h).contains(&y) {
            let title_button = ["search", "setting", "min", "close"]
                .into_iter()
                .find(|key| {
                    title_buttons.is_visible(key) && self.title_button_rect(key).contains(x, y)
                });
            if title_button.is_none() && (!search_on || !self.search_rect().contains(x, y)) {
                return MainPointerDownTarget::TitleDrag;
            }
        }

        for key in ["search", "setting", "min", "close"] {
            if title_buttons.is_visible(key) && self.title_button_rect(key).contains(x, y) {
                return MainPointerDownTarget::TitleButton(key);
            }
        }

        if scroll_to_top_visible && self.scroll_to_top_button_rect().contains(x, y) {
            return MainPointerDownTarget::ScrollToTop;
        }

        if let Some(thumb) = self.scrollbar_thumb_rect(filtered_len, scroll_y) {
            if UiRect::new(thumb.left - 8, thumb.top, thumb.right + 8, thumb.bottom).contains(x, y)
            {
                return MainPointerDownTarget::ScrollbarThumb;
            }
        }

        if let Some(track) = self.scrollbar_track_rect(filtered_len) {
            if UiRect::new(track.left - 8, track.top, track.right + 8, track.bottom).contains(x, y)
            {
                return MainPointerDownTarget::ScrollbarTrack;
            }
        }

        let (tab0, tab1) = self.segment_rects();
        if tab0.contains(x, y) {
            return MainPointerDownTarget::Tab(0);
        }
        if tab1.contains(x, y) {
            return MainPointerDownTarget::Tab(1);
        }

        let row = self.hit_test_row(x, y, filtered_len, scroll_y);
        if row >= 0 {
            MainPointerDownTarget::Row(row)
        } else {
            MainPointerDownTarget::None
        }
    }

    pub(crate) fn pointer_down_state_plan(
        self,
        target: MainPointerDownTarget,
        x: i32,
        y: i32,
        current_scroll_y: i32,
    ) -> MainPointerDownStatePlan {
        match target {
            MainPointerDownTarget::TitleButton(key) => MainPointerDownStatePlan {
                down_title_button: key,
                down_scroll_to_top: false,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: None,
            },
            MainPointerDownTarget::ScrollToTop => MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: true,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: None,
            },
            MainPointerDownTarget::ScrollbarThumb => MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: false,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: Some(MainScrollDragStart {
                    pointer_y: y,
                    scroll_y: current_scroll_y,
                }),
            },
            MainPointerDownTarget::Row(row) => MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: false,
                down_row: row,
                down_x: x,
                down_y: y,
                scroll_drag_start: None,
            },
            MainPointerDownTarget::None
            | MainPointerDownTarget::TitleDrag
            | MainPointerDownTarget::ScrollbarTrack
            | MainPointerDownTarget::Tab(_) => MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: false,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: None,
            },
        }
    }

    pub(crate) fn pointer_up_transition(
        self,
        x: i32,
        y: i32,
        filtered_len: usize,
        scroll_y: i32,
        down_title_button: &'static str,
        down_scroll_to_top: bool,
        down_row: i32,
    ) -> MainPointerUpTransition {
        if !down_title_button.is_empty() {
            return MainPointerUpTransition {
                target: MainPointerUpTarget::TitleButton {
                    key: down_title_button,
                    activated: self.title_button_rect(down_title_button).contains(x, y),
                },
            };
        }

        if down_scroll_to_top {
            return MainPointerUpTransition {
                target: MainPointerUpTarget::ScrollToTop {
                    activated: self.scroll_to_top_button_rect().contains(x, y),
                },
            };
        }

        if down_row >= 0 {
            let release_row = self.hit_test_row(x, y, filtered_len, scroll_y);
            return MainPointerUpTransition {
                target: MainPointerUpTarget::Row(MainRowRelease {
                    down_row,
                    release_row,
                    accepted: release_row == down_row,
                }),
            };
        }

        MainPointerUpTransition {
            target: MainPointerUpTarget::None,
        }
    }

    pub(crate) fn row_release_action(
        self,
        release: MainRowRelease,
        x: i32,
        y: i32,
        filtered_len: usize,
        scroll_y: i32,
        hover_idx: i32,
        quick_delete_available: bool,
        modifiers: MainPointerModifiers,
    ) -> MainRowReleaseAction {
        if !release.accepted || release.release_row < 0 {
            return MainRowReleaseAction::None;
        }

        let row = release.release_row;
        if quick_delete_available && hover_idx == row {
            if self
                .quick_action_rect(row, filtered_len, scroll_y, 0)
                .map(|rect| rect.contains(x, y))
                .unwrap_or(false)
            {
                return MainRowReleaseAction::QuickDelete { row };
            }
        }

        if modifiers.ctrl || modifiers.shift {
            return MainRowReleaseAction::Select { row, modifiers };
        }

        MainRowReleaseAction::Paste { row }
    }

    pub(crate) fn frame_hit_target(
        self,
        x: i32,
        y: i32,
        title_buttons: TitleButtonVisibility,
        search_on: bool,
        caption_draggable: bool,
    ) -> MainFrameHitTarget {
        if !(0..self.title_h).contains(&y) {
            return MainFrameHitTarget::Client;
        }
        if search_on && self.search_rect().contains(x, y) {
            return MainFrameHitTarget::Client;
        }
        for key in ["search", "setting", "min", "close"] {
            if title_buttons.is_visible(key) && self.title_button_rect(key).contains(x, y) {
                return MainFrameHitTarget::Client;
            }
        }
        if caption_draggable {
            MainFrameHitTarget::Caption
        } else {
            MainFrameHitTarget::Client
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_layout() -> MainUiLayout {
        MainUiLayout {
            win_w: 400,
            title_h: 40,
            seg_x: 20,
            seg_y: 50,
            seg_w: 200,
            seg_h: 32,
            list_x: 20,
            list_y: 100,
            list_w: 260,
            list_h: 220,
            list_pad: 10,
            row_h: 20,
            btn_w: 32,
            btn_gap: 4,
            search_left: 70,
            search_top: 6,
            search_w: 120,
            search_h: 28,
        }
    }

    #[test]
    fn main_layout_core_maps_pointer_targets_without_host_state() {
        let layout = test_layout();
        let close = layout.title_button_rect("close");
        assert_eq!(
            layout.pointer_down_target(
                close.left + 1,
                close.top + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false,
            ),
            MainPointerDownTarget::TitleButton("close")
        );

        let (records, phrases) = layout.segment_rects();
        assert_eq!(
            layout.pointer_down_target(
                records.left + 1,
                records.top + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false,
            ),
            MainPointerDownTarget::Tab(0)
        );
        assert_eq!(
            layout.pointer_down_target(
                phrases.left + 1,
                phrases.top + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false,
            ),
            MainPointerDownTarget::Tab(1)
        );
    }

    #[test]
    fn main_pointer_down_state_plan_describes_press_state_without_host_actions() {
        let layout = test_layout();
        assert_eq!(
            layout.pointer_down_state_plan(
                MainPointerDownTarget::TitleButton("close"),
                391,
                8,
                120,
            ),
            MainPointerDownStatePlan {
                down_title_button: "close",
                down_scroll_to_top: false,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: None,
            }
        );
        assert_eq!(
            layout.pointer_down_state_plan(MainPointerDownTarget::ScrollToTop, 240, 290, 120),
            MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: true,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: None,
            }
        );
        assert_eq!(
            layout.pointer_down_state_plan(MainPointerDownTarget::ScrollbarThumb, 278, 180, 120),
            MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: false,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: Some(MainScrollDragStart {
                    pointer_y: 180,
                    scroll_y: 120
                }),
            }
        );
        assert_eq!(
            layout.pointer_down_state_plan(MainPointerDownTarget::Row(3), 80, 176, 120),
            MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: false,
                down_row: 3,
                down_x: 80,
                down_y: 176,
                scroll_drag_start: None,
            }
        );
        assert_eq!(
            layout.pointer_down_state_plan(MainPointerDownTarget::Tab(1), 140, 60, 120),
            MainPointerDownStatePlan {
                down_title_button: "",
                down_scroll_to_top: false,
                down_row: -1,
                down_x: 0,
                down_y: 0,
                scroll_drag_start: None,
            }
        );
    }

    #[test]
    fn main_pointer_move_transition_describes_hover_and_drag_without_host_state() {
        let layout = test_layout();
        let search = layout.title_button_rect("search");
        let search_hover = layout.pointer_move_transition(
            search.left + 1,
            search.top + 1,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
            MainHoverTarget::default(),
            false,
            0,
            0,
        );
        let hover = search_hover.hover.unwrap();
        assert_eq!(hover.next.title_button, "search");
        assert_eq!(hover.next.row, -1);
        assert!(hover.target_changed);
        assert!(!hover.row_changed);
        assert!(!hover.show_scrollbar_feedback);

        let row_hover = layout.pointer_move_transition(
            layout.list_x + layout.list_pad + 4,
            layout.list_y + layout.list_pad + layout.row_h + 4,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
            hover.next,
            false,
            0,
            0,
        );
        let hover = row_hover.hover.unwrap();
        assert_eq!(hover.next.row, 1);
        assert_eq!(hover.next.title_button, "");
        assert!(hover.target_changed);
        assert!(hover.row_changed);

        let track = layout.scrollbar_track_rect(30).unwrap();
        let scrollbar_hover = layout.pointer_move_transition(
            track.left - 2,
            track.top + 2,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
            MainHoverTarget::default(),
            false,
            0,
            0,
        );
        let hover = scrollbar_hover.hover.unwrap();
        assert!(hover.next.scrollbar);
        assert!(hover.show_scrollbar_feedback);

        let drag = layout.pointer_move_transition(
            search.left + 1,
            220,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
            MainHoverTarget::default(),
            true,
            120,
            80,
        );
        assert_eq!(drag.hover, None);
        assert_eq!(
            drag.drag_scroll_y,
            layout.scrollbar_drag_scroll_target(30, 120, 80, 220)
        );
    }

    #[test]
    fn main_hover_clear_transition_can_preserve_or_clear_scrollbar_hover() {
        let current = MainHoverTarget {
            title_button: "search",
            tab: 1,
            scrollbar: true,
            scroll_to_top: true,
            row: 3,
        };

        let leave = current.clear_transition(true);
        assert_eq!(leave.next, MainHoverTarget::default());
        assert!(leave.changed);

        let clear = current.clear_transition(false);
        assert_eq!(
            clear.next,
            MainHoverTarget {
                scrollbar: true,
                ..MainHoverTarget::default()
            }
        );
        assert!(clear.changed);

        assert!(!MainHoverTarget::default().clear_transition(true).changed);
    }

    #[test]
    fn main_pointer_up_transition_describes_release_targets_without_host_state() {
        let layout = test_layout();
        let close = layout.title_button_rect("close");
        assert_eq!(
            layout
                .pointer_up_transition(close.left + 1, close.top + 1, 30, 0, "close", false, -1)
                .target,
            MainPointerUpTarget::TitleButton {
                key: "close",
                activated: true
            }
        );
        assert_eq!(
            layout
                .pointer_up_transition(close.left - 4, close.top - 4, 30, 0, "close", false, -1)
                .target,
            MainPointerUpTarget::TitleButton {
                key: "close",
                activated: false
            }
        );

        let to_top = layout.scroll_to_top_button_rect();
        assert_eq!(
            layout
                .pointer_up_transition(to_top.left + 1, to_top.top + 1, 30, 80, "", true, -1)
                .target,
            MainPointerUpTarget::ScrollToTop { activated: true }
        );
        assert_eq!(
            layout
                .pointer_up_transition(to_top.left - 1, to_top.top - 1, 30, 80, "", true, -1)
                .target,
            MainPointerUpTarget::ScrollToTop { activated: false }
        );

        let row_x = layout.list_x + layout.list_pad + 4;
        let row2_y = layout.list_y + layout.list_pad + layout.row_h * 2 + 4;
        assert_eq!(
            layout
                .pointer_up_transition(row_x, row2_y, 30, 0, "", false, 2)
                .target,
            MainPointerUpTarget::Row(MainRowRelease {
                down_row: 2,
                release_row: 2,
                accepted: true
            })
        );
        assert_eq!(
            layout
                .pointer_up_transition(row_x, row2_y + layout.row_h, 30, 0, "", false, 2)
                .target,
            MainPointerUpTarget::Row(MainRowRelease {
                down_row: 2,
                release_row: 3,
                accepted: false
            })
        );
        assert_eq!(
            layout
                .pointer_up_transition(0, 0, 30, 0, "", false, -1)
                .target,
            MainPointerUpTarget::None
        );
    }

    #[test]
    fn main_row_release_action_prioritizes_quick_delete_selection_and_paste() {
        let layout = test_layout();
        let release = MainRowRelease {
            down_row: 2,
            release_row: 2,
            accepted: true,
        };
        let delete = layout.quick_action_rect(2, 30, 0, 0).unwrap();
        assert_eq!(
            layout.row_release_action(
                release,
                delete.left + 1,
                delete.top + 1,
                30,
                0,
                2,
                true,
                MainPointerModifiers {
                    ctrl: true,
                    shift: false
                },
            ),
            MainRowReleaseAction::QuickDelete { row: 2 }
        );

        let row_x = layout.list_x + layout.list_pad + 4;
        let row_y = layout.list_y + layout.list_pad + layout.row_h * 2 + 4;
        assert_eq!(
            layout.row_release_action(
                release,
                row_x,
                row_y,
                30,
                0,
                2,
                true,
                MainPointerModifiers {
                    ctrl: true,
                    shift: false
                },
            ),
            MainRowReleaseAction::Select {
                row: 2,
                modifiers: MainPointerModifiers {
                    ctrl: true,
                    shift: false
                }
            }
        );

        assert_eq!(
            layout.row_release_action(
                release,
                row_x,
                row_y,
                30,
                0,
                2,
                false,
                MainPointerModifiers::default(),
            ),
            MainRowReleaseAction::Paste { row: 2 }
        );

        assert_eq!(
            layout.row_release_action(
                MainRowRelease {
                    accepted: false,
                    ..release
                },
                row_x,
                row_y,
                30,
                0,
                2,
                true,
                MainPointerModifiers::default(),
            ),
            MainRowReleaseAction::None
        );
    }

    #[test]
    fn main_layout_core_scrollbar_math_is_platform_neutral() {
        let layout = test_layout();
        assert_eq!(layout.max_scroll(30), 400);
        assert_eq!(layout.wheel_scroll_target(100, 30, 120), 60);
        assert_eq!(layout.wheel_scroll_target(100, 30, -120), 140);
        assert_eq!(
            layout.scroll_update_for_wheel(100, 30, 120),
            MainScrollUpdate {
                scroll_y: 60,
                changed: true
            }
        );
        assert_eq!(
            layout.scroll_update_for_target(0, 30, -200),
            MainScrollUpdate {
                scroll_y: 0,
                changed: false
            }
        );

        let track = layout.scrollbar_track_rect(30).unwrap();
        assert_eq!(
            layout.scrollbar_track_click_scroll_target(30, track.top),
            Some(0)
        );
        assert_eq!(
            layout.scrollbar_track_click_scroll_target(30, track.bottom),
            Some(layout.max_scroll(30))
        );
        assert_eq!(
            layout.scroll_update_for_track_click(100, 30, track.top),
            Some(MainScrollUpdate {
                scroll_y: 0,
                changed: true
            })
        );
        assert_eq!(
            layout.scroll_update_for_track_click(layout.max_scroll(30), 30, track.bottom),
            Some(MainScrollUpdate {
                scroll_y: layout.max_scroll(30),
                changed: false
            })
        );
    }

    #[test]
    fn main_icon_kind_maps_clip_item_semantics_without_host_resources() {
        assert_eq!(
            MainIconKind::for_clip_item(ClipKind::Text, false),
            MainIconKind::Text
        );
        assert_eq!(
            MainIconKind::for_clip_item(ClipKind::Phrase, false),
            MainIconKind::Text
        );
        assert_eq!(
            MainIconKind::for_clip_item(ClipKind::Image, false),
            MainIconKind::Image
        );
        assert_eq!(
            MainIconKind::for_clip_item(ClipKind::Files, false),
            MainIconKind::File
        );
        assert_eq!(
            MainIconKind::for_clip_item(ClipKind::Files, true),
            MainIconKind::Folder
        );
    }

    #[test]
    fn main_render_plan_describes_visible_regions_without_host_renderer() {
        let layout = test_layout();
        let mut row_icon_kinds = vec![MainIconKind::Text; 20];
        row_icon_kinds[3] = MainIconKind::Image;
        row_icon_kinds[4] = MainIconKind::File;
        row_icon_kinds[5] = MainIconKind::Folder;
        let plan = layout.render_plan(MainRenderInput {
            client_rect: UiRect::new(0, 0, 300, 620),
            visible_len: 20,
            scroll_y: 45,
            empty_state: MainEmptyStateKind::Records,
            hover_idx: 4,
            sel_idx: 3,
            selected_rows: vec![7],
            row_icon_kinds,
            tab_index: 1,
            hover_tab: 0,
            hover_title_button: "search",
            down_title_button: "close",
            search_on: true,
            active_loading: true,
            scroll_fade_alpha: 180,
            hover_scroll: true,
            scroll_to_top_visible: true,
            hover_scroll_to_top: false,
            down_scroll_to_top: true,
            title_buttons: TitleButtonVisibility {
                search: true,
                setting: false,
                minimize: true,
                close: true,
            },
        });

        assert_eq!(
            plan.title_buttons
                .iter()
                .map(|button| button.key)
                .collect::<Vec<_>>(),
            vec!["search", "min", "close"]
        );
        assert_eq!(
            plan.title_buttons
                .iter()
                .map(|button| (button.kind, button.state))
                .collect::<Vec<_>>(),
            vec![
                (MainTitleButtonKind::Search, MainControlVisualState::Hovered),
                (
                    MainTitleButtonKind::Minimize,
                    MainControlVisualState::Normal
                ),
                (MainTitleButtonKind::Close, MainControlVisualState::Pressed)
            ]
        );
        assert_eq!(
            plan.icon_commands
                .iter()
                .map(|command| command.kind)
                .collect::<Vec<_>>(),
            vec![
                MainIconKind::App,
                MainIconKind::Search,
                MainIconKind::Minimize,
                MainIconKind::Close
            ]
        );
        assert_eq!(plan.icon_commands[0].rect, layout.app_icon_rect());
        assert_eq!(
            plan.icon_commands[0].color_mode,
            MainIconColorMode::Original
        );
        assert!(plan.icon_commands[1..]
            .iter()
            .all(|command| command.color_mode == MainIconColorMode::ThemeAware));
        assert!(plan
            .icon_commands
            .iter()
            .all(|command| command.rect.width() > 0 && command.rect.height() > 0));
        assert_eq!(plan.chrome_commands.len(), 5);
        assert!(matches!(
            plan.chrome_commands[0],
            MainPaintCommand::FillRect {
                rect: UiRect {
                    left: 0,
                    top: 0,
                    right: 300,
                    bottom: 620
                },
                fill: MainPaintFill::Theme(MainThemeRole::Background),
            }
        ));
        assert!(matches!(
            plan.chrome_commands[1],
            MainPaintCommand::RoundRect {
                fill: MainPaintFill::Theme(MainThemeRole::ButtonHover),
                stroke: None,
                radius: 6,
                ..
            }
        ));
        assert!(matches!(
            plan.chrome_commands[2],
            MainPaintCommand::FillRect {
                fill: MainPaintFill::Theme(MainThemeRole::CloseHover),
                ..
            }
        ));
        assert!(matches!(
            plan.chrome_commands[3],
            MainPaintCommand::RoundRect {
                fill: MainPaintFill::Theme(MainThemeRole::ControlBg),
                stroke: Some(MainThemeRole::ControlStroke),
                radius: 10,
                ..
            }
        ));
        assert!(matches!(
            plan.chrome_commands[4],
            MainPaintCommand::RoundRect {
                fill: MainPaintFill::Theme(MainThemeRole::Surface),
                stroke: Some(MainThemeRole::Stroke),
                radius: 10,
                ..
            }
        ));
        assert_eq!(plan.search_rect, Some(layout.search_rect()));
        assert_eq!(
            plan.search_host,
            MainSearchHostPlan {
                visible: true,
                outer_rect: layout.search_rect(),
                input_rect: UiRect::new(
                    layout.search_rect().left + 10,
                    layout.search_rect().top + 5,
                    layout.search_rect().right - 10,
                    layout.search_rect().bottom - 5
                ),
            }
        );
        assert_eq!(plan.segment_rect, layout.segment_rect());
        let (left, right) = layout.segment_rects();
        assert_eq!(plan.tab_rects, [left, right]);
        assert_eq!(
            plan.segment_commands,
            vec![
                MainPaintCommand::RoundRect {
                    rect: layout.segment_rect(),
                    fill: MainPaintFill::Theme(MainThemeRole::Surface),
                    stroke: Some(MainThemeRole::Stroke),
                    radius: 4,
                },
                MainPaintCommand::RoundRect {
                    rect: right.inflate(-2, -2),
                    fill: MainPaintFill::Theme(MainThemeRole::SegmentSelected),
                    stroke: Some(MainThemeRole::Stroke),
                    radius: 3,
                },
                MainPaintCommand::RoundFill {
                    rect: left.inflate(-2, -2),
                    fill: MainPaintFill::Theme(MainThemeRole::ItemHovered),
                    radius: 3,
                },
            ]
        );
        assert_eq!(
            plan.segment_text_commands
                .iter()
                .map(|command| (
                    command.role,
                    command.rect,
                    command.color,
                    command.size,
                    command.horizontal_align,
                    command.font
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    MainTextRole::SegmentRecords,
                    left,
                    MainThemeRole::Text,
                    (layout.segment_rect().height() * 13 / 30).clamp(12, 16),
                    HorizontalAlign::Center,
                    MainFontRole::Display
                ),
                (
                    MainTextRole::SegmentPhrases,
                    right,
                    MainThemeRole::Text,
                    (layout.segment_rect().height() * 13 / 30).clamp(12, 16),
                    HorizontalAlign::Center,
                    MainFontRole::Display
                ),
            ]
        );
        assert_eq!(plan.list_rect, layout.list_rect());
        assert_eq!(
            plan.visible_rows
                .iter()
                .map(|row| row.index)
                .collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        );
        assert!(plan
            .visible_rows
            .iter()
            .all(|row| row.icon_rect.is_some() && row.pin_rect.is_some()));
        assert_eq!(
            plan.visible_rows
                .iter()
                .filter_map(|row| row
                    .item_icon_command
                    .map(|command| (row.index, command.kind)))
                .take(4)
                .collect::<Vec<_>>(),
            vec![
                (2, MainIconKind::Text),
                (3, MainIconKind::Image),
                (4, MainIconKind::File),
                (5, MainIconKind::Folder)
            ]
        );
        assert_eq!(
            plan.visible_rows
                .iter()
                .filter(|row| row.selected)
                .map(|row| row.index)
                .collect::<Vec<_>>(),
            vec![3, 7]
        );
        assert_eq!(
            plan.visible_rows
                .iter()
                .filter_map(|row| row.background.map(|background| (row.index, background)))
                .collect::<Vec<_>>(),
            vec![
                (3, MainRowBackground::Selected),
                (4, MainRowBackground::Hovered),
                (7, MainRowBackground::Selected)
            ]
        );
        assert_eq!(plan.row_background_commands.len(), 3);
        assert!(matches!(
            plan.row_background_commands[0],
            MainPaintCommand::FillRect {
                fill: MainPaintFill::Theme(MainThemeRole::ItemSelected),
                ..
            }
        ));
        assert!(matches!(
            plan.row_background_commands[1],
            MainPaintCommand::FillRect {
                fill: MainPaintFill::Theme(MainThemeRole::ItemHovered),
                ..
            }
        ));
        assert_eq!(
            plan.visible_rows
                .iter()
                .filter(|row| row.hovered)
                .map(|row| row.index)
                .collect::<Vec<_>>(),
            vec![4]
        );
        assert!(plan.empty_state_rect.is_none());
        assert!(plan.loading_footer_rect.is_some());
        assert!(plan.scrollbar_thumb_rect.is_some());
        assert_eq!(
            plan.scroll_to_top_rect,
            Some(layout.scroll_to_top_button_rect())
        );
        assert_eq!(plan.overlay_commands.len(), 2);
        assert!(matches!(
            plan.overlay_commands[0],
            MainPaintCommand::RoundFill {
                fill: MainPaintFill::ScrollbarThumb { alpha: 180 },
                radius: 3,
                ..
            }
        ));
        assert!(matches!(
            plan.overlay_commands[1],
            MainPaintCommand::RoundRect {
                fill: MainPaintFill::Theme(MainThemeRole::ButtonPressed),
                stroke: Some(MainThemeRole::Stroke),
                radius: 10,
                ..
            }
        ));
        assert_eq!(
            plan.text_commands
                .iter()
                .map(|command| (
                    command.role,
                    command.layer,
                    command.color,
                    command.size,
                    command.bold,
                    command.horizontal_align,
                    command.font
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    MainTextRole::LoadingFooter,
                    MainTextLayer::Content,
                    MainThemeRole::TextMuted,
                    layout.row_muted_text_size(),
                    false,
                    HorizontalAlign::Center,
                    MainFontRole::UiText
                ),
                (
                    MainTextRole::ScrollToTopArrow,
                    MainTextLayer::Overlay,
                    MainThemeRole::Text,
                    18,
                    true,
                    HorizontalAlign::Center,
                    MainFontRole::Display
                )
            ]
        );
    }

    #[test]
    fn main_text_commands_use_platform_neutral_layout_semantics() {
        let layout = test_layout();
        let plan = layout.render_plan(MainRenderInput {
            client_rect: UiRect::new(0, 0, 300, 620),
            visible_len: 1,
            scroll_y: 0,
            empty_state: MainEmptyStateKind::Records,
            hover_idx: -1,
            sel_idx: -1,
            selected_rows: Vec::new(),
            row_icon_kinds: vec![MainIconKind::Text],
            tab_index: 0,
            hover_tab: -1,
            hover_title_button: "",
            down_title_button: "",
            search_on: false,
            active_loading: false,
            scroll_fade_alpha: 0,
            hover_scroll: false,
            scroll_to_top_visible: false,
            hover_scroll_to_top: false,
            down_scroll_to_top: false,
            title_buttons: TitleButtonVisibility::default(),
        });

        assert!(plan.segment_text_commands.iter().all(|command| {
            command.horizontal_align == HorizontalAlign::Center
                && command.vertical_align == VerticalAlign::Center
                && command.wrap == TextWrap::NoWrap
        }));

        let row = plan.visible_rows.first().expect("one visible row");
        let content = layout.row_content_plan(
            row,
            MainRowContentInput {
                pinned: false,
                show_delete: false,
                show_preview: false,
            },
        );
        assert_eq!(
            content.text_command.horizontal_align,
            HorizontalAlign::Start
        );
        assert_eq!(content.text_command.vertical_align, VerticalAlign::Center);
        assert_eq!(content.text_command.wrap, TextWrap::NoWrap);
    }

    fn menu_actions(plan: &MainRowMenuPlan) -> Vec<Option<MainRowMenuAction>> {
        plan.entries
            .iter()
            .map(|entry| match entry {
                MainRowMenuEntry::Action { action, .. } => Some(*action),
                MainRowMenuEntry::Separator => None,
            })
            .collect()
    }

    fn test_clip_item(kind: ClipKind, preview: &str) -> ClipItem {
        ClipItem {
            id: 1,
            kind,
            preview: preview.to_string(),
            text: None,
            source_app: "test.exe".to_string(),
            file_paths: None,
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: "2026-06-06 12:00:00".to_string(),
        }
    }

    #[test]
    fn main_row_menu_plan_describes_multi_selection_without_host_menu() {
        let plan = main_row_menu_plan(MainRowMenuInput {
            selected_count: 3,
            has_unpinned: false,
            current_kind: ClipKind::Text,
            grouping_enabled: true,
            current_can_ocr: false,
            current_can_translate: false,
            current_is_excel: false,
            quick_search_enabled: false,
            qr_quick_enabled: false,
            super_mail_merge_enabled: false,
            lan_push_available: false,
        });

        assert_eq!(
            menu_actions(&plan),
            vec![
                Some(MainRowMenuAction::Copy),
                None,
                Some(MainRowMenuAction::Pin),
                Some(MainRowMenuAction::ToPhrase),
                Some(MainRowMenuAction::AddToGroup),
                Some(MainRowMenuAction::RemoveFromGroup),
                Some(MainRowMenuAction::Delete),
                Some(MainRowMenuAction::DeleteUnpinned),
            ]
        );
        assert!(matches!(
            plan.entries.last(),
            Some(MainRowMenuEntry::Action {
                action: MainRowMenuAction::DeleteUnpinned,
                enabled: false
            })
        ));
    }

    #[test]
    fn main_row_menu_plan_describes_file_and_text_feature_entries() {
        let files = main_row_menu_plan(MainRowMenuInput {
            selected_count: 1,
            has_unpinned: true,
            current_kind: ClipKind::Files,
            grouping_enabled: false,
            current_can_ocr: true,
            current_can_translate: false,
            current_is_excel: true,
            quick_search_enabled: false,
            qr_quick_enabled: true,
            super_mail_merge_enabled: true,
            lan_push_available: true,
        });
        assert_eq!(
            menu_actions(&files)[..8],
            [
                Some(MainRowMenuAction::OpenPath),
                Some(MainRowMenuAction::OpenFolder),
                Some(MainRowMenuAction::CopyPath),
                Some(MainRowMenuAction::ImageOcr),
                Some(MainRowMenuAction::QrImage),
                Some(MainRowMenuAction::MailMerge),
                Some(MainRowMenuAction::LanPush),
                None,
            ]
        );

        let text = main_row_menu_plan(MainRowMenuInput {
            selected_count: 1,
            has_unpinned: true,
            current_kind: ClipKind::Text,
            grouping_enabled: true,
            current_can_ocr: false,
            current_can_translate: true,
            current_is_excel: false,
            quick_search_enabled: true,
            qr_quick_enabled: true,
            super_mail_merge_enabled: false,
            lan_push_available: false,
        });
        assert_eq!(
            menu_actions(&text),
            vec![
                Some(MainRowMenuAction::Edit),
                Some(MainRowMenuAction::QuickSearch),
                Some(MainRowMenuAction::TextTranslate),
                Some(MainRowMenuAction::QrImage),
                Some(MainRowMenuAction::ExportFile),
                None,
                Some(MainRowMenuAction::Pin),
                Some(MainRowMenuAction::ToPhrase),
                Some(MainRowMenuAction::AddToGroup),
                Some(MainRowMenuAction::RemoveFromGroup),
                Some(MainRowMenuAction::Delete),
                Some(MainRowMenuAction::DeleteUnpinned),
            ]
        );
    }

    #[test]
    fn main_row_menu_labels_are_platform_neutral() {
        let single_unpinned = MainRowMenuLabelInput {
            selected_count: 1,
            has_unpinned: true,
            current_is_dir: false,
        };
        assert_eq!(
            main_row_menu_action_label(MainRowMenuAction::Pin, single_unpinned),
            "置顶"
        );

        let single_pinned = MainRowMenuLabelInput {
            selected_count: 1,
            has_unpinned: false,
            current_is_dir: false,
        };
        assert_eq!(
            main_row_menu_action_label(MainRowMenuAction::Pin, single_pinned),
            "取消置顶"
        );

        let multi = MainRowMenuLabelInput {
            selected_count: 3,
            has_unpinned: true,
            current_is_dir: false,
        };
        assert_eq!(
            main_row_menu_action_label(MainRowMenuAction::Pin, multi),
            "置顶所选"
        );
        assert_eq!(
            main_row_menu_action_label(MainRowMenuAction::Delete, multi),
            "删除所选"
        );

        let directory = MainRowMenuLabelInput {
            selected_count: 1,
            has_unpinned: false,
            current_is_dir: true,
        };
        assert_eq!(
            main_row_menu_action_label(MainRowMenuAction::OpenPath, directory),
            "打开文件夹"
        );
    }

    #[test]
    fn main_row_external_actions_prepare_host_work_without_platform_calls() {
        let mut current = test_clip_item(ClipKind::Files, "files");
        current.file_paths = Some(vec!["C:\\one.txt".to_string(), "D:\\two.txt".to_string()]);

        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::OpenPath, Some(&current), &[]),
            Some(MainRowExternalActionPlan::OpenPaths(vec![
                "C:\\one.txt".to_string(),
                "D:\\two.txt".to_string()
            ]))
        );
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::OpenFolder, Some(&current), &[]),
            Some(MainRowExternalActionPlan::OpenParentFolders(vec![
                "C:\\one.txt".to_string(),
                "D:\\two.txt".to_string()
            ]))
        );

        let mut selected_a = test_clip_item(ClipKind::Files, "a");
        selected_a.file_paths = Some(vec!["A:\\a.txt".to_string()]);
        let mut selected_b = test_clip_item(ClipKind::Files, "b");
        selected_b.file_paths = Some(vec!["B:\\b.txt".to_string(), "B:\\c.txt".to_string()]);
        let selected = vec![selected_a, selected_b];
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::CopyPath, Some(&current), &selected),
            Some(MainRowExternalActionPlan::CopyText(
                "A:\\a.txt\nB:\\b.txt\nB:\\c.txt".to_string()
            ))
        );
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::LanPush, Some(&current), &[]),
            Some(MainRowExternalActionPlan::LanPushFiles(vec![
                "C:\\one.txt".to_string(),
                "D:\\two.txt".to_string()
            ]))
        );
    }

    #[test]
    fn main_copy_selection_plan_prepares_merged_clipboard_text() {
        let mut current = test_clip_item(ClipKind::Text, "current");
        current.text = Some("current text".to_string());
        assert_eq!(
            main_copy_selection_plan(Some(&current), &[]),
            Some(MainCopySelectionPlan::CopyCurrentItem)
        );

        let mut text = test_clip_item(ClipKind::Text, "text");
        text.text = Some("  text body  ".to_string());
        let image = test_clip_item(ClipKind::Image, "image preview");
        let mut files = test_clip_item(ClipKind::Files, "files");
        files.file_paths = Some(vec![
            " C:\\a.txt ".to_string(),
            "".to_string(),
            "D:\\b.txt".to_string(),
        ]);
        assert_eq!(
            main_copy_selection_plan(Some(&current), &[text, image, files]),
            Some(MainCopySelectionPlan::CopyMergedText(
                "text body\nimage preview\nC:\\a.txt\nD:\\b.txt".to_string()
            ))
        );

        let empty = test_clip_item(ClipKind::Text, "empty");
        assert_eq!(main_copy_selection_plan(None, &[empty]), None);
    }

    #[test]
    fn main_paste_completion_plan_separates_host_workflow_steps() {
        let input = MainPasteCompletionInput {
            item_id: 42,
            move_pasted_item_to_top: true,
            click_hide: true,
            paste_success_sound_enabled: true,
        };
        assert_eq!(
            main_paste_completion_plan(MainPasteCompletionKind::DirectEdit, input),
            MainPasteCompletionPlan {
                promote_item_id: Some(42),
                reset_plain_text_paste_mode: false,
                clear_selection: true,
                clear_hover: true,
                hide_main_now: true,
                play_success_sound: true,
                send_paste_after_clipboard: false,
                paste_hide_main: false,
                paste_backspaces: 0,
            }
        );
        assert_eq!(
            main_paste_completion_plan(MainPasteCompletionKind::AsyncImage, input),
            MainPasteCompletionPlan {
                promote_item_id: Some(42),
                reset_plain_text_paste_mode: false,
                clear_selection: true,
                clear_hover: true,
                hide_main_now: true,
                play_success_sound: false,
                send_paste_after_clipboard: false,
                paste_hide_main: false,
                paste_backspaces: 0,
            }
        );
        assert_eq!(
            main_paste_completion_plan(MainPasteCompletionKind::Clipboard, input),
            MainPasteCompletionPlan {
                promote_item_id: Some(42),
                reset_plain_text_paste_mode: true,
                clear_selection: true,
                clear_hover: true,
                hide_main_now: false,
                play_success_sound: false,
                send_paste_after_clipboard: true,
                paste_hide_main: true,
                paste_backspaces: 0,
            }
        );

        let no_promote = MainPasteCompletionInput {
            item_id: 0,
            move_pasted_item_to_top: true,
            click_hide: false,
            paste_success_sound_enabled: false,
        };
        assert_eq!(
            main_paste_completion_plan(MainPasteCompletionKind::Clipboard, no_promote)
                .promote_item_id,
            None
        );

        assert_eq!(
            main_paste_completion_plan_with_backspaces(
                MainPasteCompletionKind::VvClipboard,
                input,
                2
            ),
            MainPasteCompletionPlan {
                promote_item_id: Some(42),
                reset_plain_text_paste_mode: false,
                clear_selection: false,
                clear_hover: false,
                hide_main_now: false,
                play_success_sound: false,
                send_paste_after_clipboard: true,
                paste_hide_main: true,
                paste_backspaces: 2,
            }
        );
        assert_eq!(
            main_paste_completion_plan(MainPasteCompletionKind::VvAsyncImage, input),
            MainPasteCompletionPlan {
                promote_item_id: Some(42),
                reset_plain_text_paste_mode: false,
                clear_selection: false,
                clear_hover: false,
                hide_main_now: false,
                play_success_sound: false,
                send_paste_after_clipboard: false,
                paste_hide_main: false,
                paste_backspaces: 0,
            }
        );
    }

    #[test]
    fn main_paste_preparation_plan_orders_fallback_candidates() {
        assert_eq!(
            main_paste_preparation_plan(MainPastePreparationInput {
                item_kind: ClipKind::Image,
                item_id: 42,
                image_payload_loaded: false,
                direct_edit_candidate: true,
                plain_text_paste_mode: false,
            }),
            MainPastePreparationPlan {
                steps: vec![
                    MainPastePreparationStep::DirectEdit,
                    MainPastePreparationStep::AsyncImage,
                    MainPastePreparationStep::Clipboard { plain_text: false },
                ]
            }
        );

        assert_eq!(
            main_paste_preparation_plan(MainPastePreparationInput {
                item_kind: ClipKind::Text,
                item_id: 7,
                image_payload_loaded: false,
                direct_edit_candidate: false,
                plain_text_paste_mode: true,
            }),
            MainPastePreparationPlan {
                steps: vec![MainPastePreparationStep::Clipboard { plain_text: true }]
            }
        );

        assert_eq!(
            main_paste_preparation_plan(MainPastePreparationInput {
                item_kind: ClipKind::Image,
                item_id: 42,
                image_payload_loaded: true,
                direct_edit_candidate: false,
                plain_text_paste_mode: false,
            }),
            MainPastePreparationPlan {
                steps: vec![MainPastePreparationStep::Clipboard { plain_text: false }]
            }
        );
    }

    #[test]
    fn main_vv_select_plan_validates_visibility_and_index() {
        let first = test_clip_item(ClipKind::Text, "first");
        let second = test_clip_item(ClipKind::Phrase, "second");
        assert_eq!(main_vv_select_plan(false, 0, &[first.clone()], 2), None);
        assert_eq!(
            main_vv_select_plan(true, 2, &[first.clone()], 2),
            Some(MainVvSelectPlan::HidePopup)
        );
        assert_eq!(
            main_vv_select_plan(true, 1, &[first, second.clone()], 0),
            Some(MainVvSelectPlan::Paste {
                item: second,
                backspaces: 0,
            })
        );
    }

    #[test]
    fn main_vv_popup_layout_describes_size_rects_and_hits() {
        let layout = MainVvPopupLayout::default();
        assert_eq!(MAIN_VV_POPUP_MAX_ITEMS, 9);
        assert_eq!(layout.height(0), 108);
        assert_eq!(layout.height(3), 168);
        assert_eq!(layout.group_rect(), UiRect::new(210, 10, 346, 34));
        assert_eq!(layout.row_rect(0), UiRect::new(12, 68, 348, 96));
        assert_eq!(layout.row_rect(2), UiRect::new(12, 128, 348, 156));

        assert_eq!(layout.hit_test(220, 18, 3), MainVvPopupHit::Group);
        assert_eq!(layout.hit_test(1, 70, 3), MainVvPopupHit::Row(0));
        assert_eq!(layout.hit_test(50, 129, 3), MainVvPopupHit::Row(2));
        assert_eq!(layout.hit_test(50, 200, 3), MainVvPopupHit::None);
    }

    #[test]
    fn main_vv_popup_render_plan_describes_text_and_rows() {
        let layout = MainVvPopupLayout::default();
        let strings = MainVvPopupRenderStrings {
            title: "VV Mode".to_string(),
            hint: "Press 1-9".to_string(),
            empty: "No records".to_string(),
        };
        let empty = layout.render_plan(UiRect::new(0, 0, 360, 108), &strings, "All", &[]);
        assert_eq!(empty.paint_commands.len(), 3);
        assert_eq!(
            empty
                .text_commands
                .iter()
                .map(|command| command.role)
                .collect::<Vec<_>>(),
            vec![
                MainVvPopupTextRole::Title,
                MainVvPopupTextRole::Hint,
                MainVvPopupTextRole::GroupName,
                MainVvPopupTextRole::GroupArrow,
                MainVvPopupTextRole::Empty,
            ]
        );
        assert_eq!(
            empty
                .text_commands
                .iter()
                .find(|command| command.role == MainVvPopupTextRole::Empty)
                .map(|command| command.text.as_str()),
            Some("No records")
        );

        let items = vec![MainVvPopupRenderItem {
            index: 3,
            label: "row label".to_string(),
        }];
        let rows = layout.render_plan(UiRect::new(0, 0, 360, 108), &strings, "All", &items);
        assert_eq!(rows.paint_commands.len(), 4);
        assert_eq!(
            rows.text_commands
                .iter()
                .find(|command| command.role == MainVvPopupTextRole::RowIndex)
                .map(|command| (command.text.as_str(), command.rect, command.color)),
            Some(("3", UiRect::new(12, 72, 36, 92), MainThemeRole::OnAccent))
        );
        assert_eq!(
            rows.text_commands
                .iter()
                .find(|command| command.role == MainVvPopupTextRole::RowPreview)
                .map(|command| (command.text.as_str(), command.rect)),
            Some(("row label", UiRect::new(46, 68, 348, 96)))
        );
    }

    #[test]
    fn main_row_external_actions_prepare_quick_search_text() {
        let mut text = test_clip_item(ClipKind::Text, "fallback text");
        text.text = Some("real text".to_string());
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QuickSearch, Some(&text), &[]),
            Some(MainRowExternalActionPlan::QuickSearch(
                "real text".to_string()
            ))
        );

        let mut files = test_clip_item(ClipKind::Files, "fallback files");
        files.file_paths = Some(vec!["C:\\a.txt".to_string(), "D:\\b.txt".to_string()]);
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QuickSearch, Some(&files), &[]),
            Some(MainRowExternalActionPlan::QuickSearch(
                "C:\\a.txt D:\\b.txt".to_string()
            ))
        );

        let image = test_clip_item(ClipKind::Image, "图片 120 x 80");
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QuickSearch, Some(&image), &[]),
            Some(MainRowExternalActionPlan::QuickSearch(
                "图片 120 x 80".to_string()
            ))
        );
    }

    #[test]
    fn main_row_external_actions_prepare_translate_and_qr_text() {
        let mut text = test_clip_item(ClipKind::Text, "fallback text");
        text.text = Some("  real text  ".to_string());
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::TextTranslate, Some(&text), &[]),
            Some(MainRowExternalActionPlan::TextTranslate(
                "real text".to_string()
            ))
        );
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QrImage, Some(&text), &[]),
            Some(MainRowExternalActionPlan::QrText("real text".to_string()))
        );

        let preview_only = test_clip_item(ClipKind::Phrase, "fallback phrase");
        assert_eq!(
            main_row_external_action_plan(
                MainRowMenuAction::TextTranslate,
                Some(&preview_only),
                &[]
            ),
            Some(MainRowExternalActionPlan::TextTranslate(
                "fallback phrase".to_string()
            ))
        );
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QrImage, Some(&preview_only), &[]),
            None
        );

        let mut files = test_clip_item(ClipKind::Files, "files");
        files.file_paths = Some(vec![
            " C:\\a.txt ".to_string(),
            "".to_string(),
            "D:\\b.txt".to_string(),
        ]);
        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QrImage, Some(&files), &[]),
            Some(MainRowExternalActionPlan::QrText(
                "C:\\a.txt\nD:\\b.txt".to_string()
            ))
        );
    }

    #[test]
    fn main_row_dialog_actions_prepare_dialog_requests() {
        let mut file = test_clip_item(ClipKind::Files, "mail source");
        file.file_paths = Some(vec![
            "C:\\mail.xlsx".to_string(),
            "C:\\ignored.xlsx".to_string(),
        ]);
        assert_eq!(
            main_row_dialog_action_plan(MainRowMenuAction::MailMerge, Some(&file)),
            Some(MainRowDialogActionPlan::MailMerge {
                excel_path: Some("C:\\mail.xlsx".to_string())
            })
        );

        let text = test_clip_item(
            ClipKind::Text,
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        );
        assert_eq!(
            main_row_dialog_action_plan(MainRowMenuAction::Edit, Some(&text)),
            Some(MainRowDialogActionPlan::EditItem {
                item_id: 1,
                title: "编辑 — abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMN".to_string()
            })
        );

        assert_eq!(
            main_row_dialog_action_plan(MainRowMenuAction::Edit, None),
            None
        );
        assert_eq!(
            main_row_dialog_action_plan(MainRowMenuAction::QuickSearch, Some(&text)),
            None
        );
    }

    #[test]
    fn main_row_current_item_actions_prepare_current_item_requests() {
        let image = test_clip_item(ClipKind::Image, "image");
        match main_row_current_item_action_plan(MainRowMenuAction::Sticker, Some(&image)) {
            Some(MainRowCurrentItemActionPlan::Sticker { item }) => {
                assert_eq!(item.kind, ClipKind::Image);
                assert_eq!(item.preview, "image");
            }
            other => panic!("unexpected plan: {:?}", other),
        }
        assert!(matches!(
            main_row_current_item_action_plan(MainRowMenuAction::SaveImage, Some(&image)),
            Some(MainRowCurrentItemActionPlan::SaveImage { .. })
        ));
        assert!(matches!(
            main_row_current_item_action_plan(MainRowMenuAction::ImageOcr, Some(&image)),
            Some(MainRowCurrentItemActionPlan::ImageOcr { .. })
        ));

        let files = test_clip_item(ClipKind::Files, "file");
        assert_eq!(
            main_row_current_item_action_plan(MainRowMenuAction::Sticker, Some(&files)).is_some(),
            false
        );
        assert!(matches!(
            main_row_current_item_action_plan(MainRowMenuAction::ImageOcr, Some(&files)),
            Some(MainRowCurrentItemActionPlan::ImageOcr { .. })
        ));

        let text = test_clip_item(ClipKind::Text, "text");
        assert!(
            main_row_current_item_action_plan(MainRowMenuAction::SaveImage, Some(&text)).is_none()
        );
    }

    #[test]
    fn main_row_data_actions_prepare_phrase_mutation_requests() {
        let current = test_clip_item(ClipKind::Text, "current");
        assert_eq!(
            main_row_data_action_plan(MainRowMenuAction::ToPhrase, Some(&current), &[], 0),
            Some(MainRowDataActionPlan::AddToPhrase {
                items: vec![current.clone()],
                refilter_current_tab: false
            })
        );

        let selected = vec![
            test_clip_item(ClipKind::Text, "selected-a"),
            test_clip_item(ClipKind::Image, "selected-b"),
        ];
        assert_eq!(
            main_row_data_action_plan(MainRowMenuAction::ToPhrase, Some(&current), &selected, 1),
            Some(MainRowDataActionPlan::AddToPhrase {
                items: selected,
                refilter_current_tab: true
            })
        );

        assert_eq!(
            main_row_data_action_plan(MainRowMenuAction::ToPhrase, None, &[], 0),
            None
        );
        assert_eq!(
            main_row_data_action_plan(MainRowMenuAction::Delete, Some(&current), &[], 0),
            None
        );
    }

    #[test]
    fn main_row_pin_data_plan_targets_selected_or_current_items() {
        let mut current = test_clip_item(ClipKind::Text, "current");
        current.id = 11;
        current.pinned = false;
        assert_eq!(
            main_row_pin_data_plan(Some(&current), &[], 0),
            Some(MainRowDataActionPlan::UpdatePinned {
                ids: vec![11],
                pinned: true,
                invalidate_tab: 0,
                clear_selection: false,
            })
        );

        let mut pinned_a = test_clip_item(ClipKind::Text, "a");
        pinned_a.id = 21;
        pinned_a.pinned = true;
        let mut pinned_b = test_clip_item(ClipKind::Text, "b");
        pinned_b.id = 22;
        pinned_b.pinned = true;
        assert_eq!(
            main_row_pin_data_plan(Some(&current), &[pinned_a, pinned_b], 1),
            Some(MainRowDataActionPlan::UpdatePinned {
                ids: vec![21, 22],
                pinned: false,
                invalidate_tab: 1,
                clear_selection: true,
            })
        );
    }

    #[test]
    fn main_row_delete_data_plans_keep_host_refresh_intent() {
        let mut current = test_clip_item(ClipKind::Text, "current");
        current.id = 31;
        let mut selected = test_clip_item(ClipKind::Text, "selected");
        selected.id = 32;
        let mut transient = test_clip_item(ClipKind::Text, "transient");
        transient.id = 0;

        assert_eq!(
            main_row_delete_items_data_plan(Some(&current), &[selected, transient.clone()]),
            Some(MainRowDataActionPlan::DeleteItems {
                ids: vec![32],
                clear_selection: true,
                preserve_scroll_anchor: true,
            })
        );
        assert_eq!(main_row_delete_items_data_plan(None, &[transient]), None);
        assert_eq!(
            main_row_delete_unpinned_data_plan(1, 1),
            MainRowDataActionPlan::DeleteUnpinned {
                category: 1,
                active_tab: 1,
                clear_selection: true,
                preserve_scroll_anchor: true,
            }
        );
    }

    #[test]
    fn main_row_group_assignment_filters_ids_and_keeps_target() {
        assert_eq!(
            main_row_group_assignment_plan(&[0, 9, -1, 12], 7, true),
            Some(MainRowDataActionPlan::AssignGroup {
                ids: vec![9, 12],
                group_id: 7,
                refilter_after_reload: true,
            })
        );

        assert_eq!(main_row_group_assignment_plan(&[0, -3], 7, false), None);
    }

    #[test]
    fn main_row_content_plan_places_text_actions_and_preview_without_host_renderer() {
        let layout = test_layout();
        let row = layout
            .render_plan(MainRenderInput {
                client_rect: UiRect::new(0, 0, 300, 620),
                visible_len: 20,
                scroll_y: 45,
                empty_state: MainEmptyStateKind::Records,
                hover_idx: -1,
                sel_idx: -1,
                selected_rows: Vec::new(),
                row_icon_kinds: vec![MainIconKind::Text; 20],
                tab_index: 0,
                hover_tab: -1,
                hover_title_button: "",
                down_title_button: "",
                search_on: false,
                active_loading: false,
                scroll_fade_alpha: 0,
                hover_scroll: false,
                scroll_to_top_visible: false,
                hover_scroll_to_top: false,
                down_scroll_to_top: false,
                title_buttons: TitleButtonVisibility::default(),
            })
            .visible_rows
            .into_iter()
            .find(|row| row.index == 2)
            .unwrap();

        let compact = layout.row_content_plan(
            &row,
            MainRowContentInput {
                pinned: false,
                show_delete: false,
                show_preview: false,
            },
        );
        assert_eq!(compact.text_rect, UiRect::new(70, 105, 252, 125));
        assert_eq!(
            compact.text_command,
            MainRowTextCommand {
                rect: compact.text_rect,
                color: MainThemeRole::Text,
                size: layout.row_text_size(),
                bold: false,
                horizontal_align: HorizontalAlign::Start,
                vertical_align: VerticalAlign::Center,
                wrap: TextWrap::NoWrap,
                font: MainFontRole::UiText
            }
        );
        assert!(compact.delete_rect.is_none());
        assert!(compact.preview_rect.is_none());
        assert!(compact.paint_commands.is_empty());
        assert!(compact.icon_commands.is_empty());

        let expanded = layout.row_content_plan(
            &row,
            MainRowContentInput {
                pinned: true,
                show_delete: true,
                show_preview: true,
            },
        );
        assert_eq!(expanded.delete_rect, Some(UiRect::new(232, 107, 248, 123)));
        assert_eq!(expanded.preview_rect, Some(UiRect::new(92, 103, 116, 127)));
        assert_eq!(expanded.text_rect, UiRect::new(126, 105, 228, 125));
        assert_eq!(expanded.text_command.rect, expanded.text_rect);
        assert_eq!(
            expanded.paint_commands,
            vec![
                MainPaintCommand::RoundRect {
                    rect: UiRect::new(230, 105, 250, 125),
                    fill: MainPaintFill::Theme(MainThemeRole::Surface),
                    stroke: Some(MainThemeRole::Stroke),
                    radius: 10,
                },
                MainPaintCommand::RoundRect {
                    rect: UiRect::new(90, 101, 118, 129),
                    fill: MainPaintFill::Theme(MainThemeRole::Surface2),
                    stroke: Some(MainThemeRole::Stroke),
                    radius: 8,
                },
            ]
        );
        assert_eq!(
            expanded
                .icon_commands
                .iter()
                .map(|command| command.kind)
                .collect::<Vec<_>>(),
            vec![MainIconKind::Pin, MainIconKind::Delete]
        );
    }

    #[test]
    fn main_render_plan_empty_state_skips_rows_and_transient_controls() {
        let layout = test_layout();
        let plan = layout.render_plan(MainRenderInput {
            client_rect: UiRect::new(0, 0, 300, 620),
            visible_len: 0,
            scroll_y: 0,
            empty_state: MainEmptyStateKind::Phrases,
            hover_idx: -1,
            sel_idx: -1,
            selected_rows: Vec::new(),
            row_icon_kinds: Vec::new(),
            tab_index: 0,
            hover_tab: -1,
            hover_title_button: "",
            down_title_button: "",
            search_on: false,
            active_loading: false,
            scroll_fade_alpha: 255,
            hover_scroll: false,
            scroll_to_top_visible: false,
            hover_scroll_to_top: false,
            down_scroll_to_top: false,
            title_buttons: TitleButtonVisibility::default(),
        });

        assert_eq!(plan.search_rect, None);
        assert_eq!(
            plan.search_host,
            MainSearchHostPlan {
                visible: false,
                outer_rect: layout.search_rect(),
                input_rect: UiRect::new(
                    layout.search_rect().left + 10,
                    layout.search_rect().top + 5,
                    layout.search_rect().right - 10,
                    layout.search_rect().bottom - 5
                ),
            }
        );
        assert!(plan.empty_state_rect.is_some());
        assert!(plan.visible_rows.is_empty());
        assert!(plan.loading_footer_rect.is_none());
        assert!(plan.scrollbar_thumb_rect.is_none());
        assert!(plan.scroll_to_top_rect.is_none());
        assert!(plan.overlay_commands.is_empty());
        assert_eq!(
            plan.text_commands,
            vec![MainTextCommand {
                role: MainTextRole::EmptyPhrases,
                layer: MainTextLayer::Content,
                rect: plan.empty_state_rect.unwrap(),
                color: MainThemeRole::TextMuted,
                size: layout.row_muted_text_size(),
                bold: false,
                horizontal_align: HorizontalAlign::Center,
                vertical_align: VerticalAlign::Center,
                wrap: TextWrap::NoWrap,
                font: MainFontRole::UiText,
            }]
        );
    }

    #[test]
    fn main_list_query_and_loading_state_are_platform_neutral() {
        assert_eq!(
            ItemsQuery::for_tab(1, true, [3, 8], "  bili  "),
            ItemsQuery {
                category: 1,
                group_id: 8,
                search_text: "bili".to_string(),
            }
        );
        assert_eq!(
            ItemsQuery::for_tab(1, false, [3, 8], "  catsxp  "),
            ItemsQuery {
                category: 1,
                group_id: 0,
                search_text: "catsxp".to_string(),
            }
        );

        let mut load = TabLoadState::default();
        let query = ItemsQuery {
            category: 0,
            group_id: 3,
            search_text: "clip".to_string(),
        };
        let seq = load.begin_request(query.clone(), true);
        assert!(load.loading);
        assert!(load.accepts_result(seq, &query));

        load.finish_request(
            Some("network".to_string()),
            Some(ItemsCursor {
                pinned: true,
                id: 9,
            }),
            false,
        );
        assert!(!load.loading);
        assert_eq!(load.error.as_deref(), Some("network"));
        assert_eq!(
            load.next_cursor,
            Some(ItemsCursor {
                pinned: true,
                id: 9
            })
        );
        assert!(!load.has_more);

        load.invalidate();
        assert!(!load.loading);
        assert!(load.query.is_none());
        assert!(load.next_cursor.is_none());
        assert!(load.has_more);
    }

    #[test]
    fn clip_item_models_history_data_without_platform_handles() {
        let group = ClipGroup {
            id: 3,
            category: 0,
            name: "Work".to_string(),
        };
        let image = ClipItem {
            id: 9,
            kind: ClipKind::Image,
            preview: "图片 120 x 80".to_string(),
            text: None,
            source_app: "browser.exe".to_string(),
            file_paths: None,
            image_bytes: Some(vec![1, 2, 3, 4]),
            image_path: Some("clip.png".to_string()),
            image_width: 120,
            image_height: 80,
            pinned: true,
            group_id: group.id,
            created_at: "2026-06-06 12:00:00".to_string(),
        };

        assert_eq!(group.category, 0);
        assert_eq!(image.kind, ClipKind::Image);
        assert_eq!(image.group_id, group.id);
        assert_eq!(image.image_bytes.as_deref(), Some(&[1, 2, 3, 4][..]));
        assert!(image.pinned);
    }

    #[test]
    fn search_query_parser_is_platform_neutral() {
        let (terms, time, app) = parse_search_query("foo bar app:\"Cats XP\" time:2026-06-06");
        assert_eq!(terms, vec!["foo".to_string(), "bar".to_string()]);
        assert_eq!(
            time,
            Some(SearchTimeFilter::ExactDay(gregorian_to_days(2026, 6, 6)))
        );
        assert_eq!(app.as_deref(), Some("cats xp"));

        let (terms, time, app) = parse_search_query("发票 应用：微信 日期：14天");
        assert_eq!(terms, vec!["发票".to_string()]);
        assert_eq!(time, Some(SearchTimeFilter::RecentDays(14)));
        assert_eq!(app.as_deref(), Some("微信"));
    }

    #[test]
    fn search_query_date_context_keeps_local_calendar_out_of_core_platform_calls() {
        let context = SearchDateContext::from_date(2026, 6, 9);

        let (_, today, _) = parse_search_query_with_context("time:today", context);
        assert_eq!(today, Some(SearchTimeFilter::ExactDay(context.current_day)));

        let (_, yesterday, _) = parse_search_query_with_context("日期：昨天", context);
        assert_eq!(
            yesterday,
            Some(SearchTimeFilter::ExactDay(context.current_day - 1))
        );

        let (_, partial_date, _) = parse_search_query_with_context("date:02-03", context);
        assert_eq!(
            partial_date,
            Some(SearchTimeFilter::ExactDay(gregorian_to_days(2026, 2, 3)))
        );
    }

    #[test]
    fn empty_records_render_input_builds_first_screen_without_host_state() {
        let layout = MainUiLayout::zsclip();
        let client_rect = UiRect::new(0, 0, layout.win_w, layout.list_y + layout.list_h + 6);
        let input = MainRenderInput::empty_records(client_rect);

        assert_eq!(input.visible_len, 0);
        assert_eq!(input.empty_state, MainEmptyStateKind::Records);
        assert_eq!(input.sel_idx, -1);
        assert!(!input.search_on);

        let plan = layout.render_plan(input);
        assert_eq!(plan.empty_state_rect.is_some(), true);
        assert!(plan.visible_rows.is_empty());
        assert_eq!(plan.search_rect, None);
        assert!(!plan.chrome_commands.is_empty());
        assert!(!plan.text_commands.is_empty());
    }

    #[test]
    fn main_tab_switch_plan_updates_list_state_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 10,
            tab_index: 0,
            hover_idx: 3,
            sel_idx: 4,
            scroll_y: 180,
            current_group_filter: 11,
            tab_group_filters: [11, 22],
            selection_anchor: 2,
            context_row: 5,
            ..ClipListState::default()
        };
        list.selected_rows.insert(2);
        list.selected_rows.insert(4);

        let plan = list.tab_switch_plan(1);
        assert_eq!(
            plan,
            MainTabSwitchPlan {
                tab_index: 1,
                current_group_filter: 22,
                scroll_y: 0,
                clear_selection: true
            }
        );

        list.apply_tab_switch_plan(plan);
        assert_eq!(list.tab_index, 1);
        assert_eq!(list.current_group_filter, 22);
        assert_eq!(list.scroll_y, 0);
        assert_eq!(list.hover_idx, -1);
        assert_eq!(list.sel_idx, -1);
        assert!(list.selected_rows.is_empty());
        assert_eq!(list.selection_anchor, -1);
        assert_eq!(list.context_row, -1);

        let fallback = list.tab_switch_plan(9);
        assert_eq!(fallback.tab_index, 0);
        assert_eq!(fallback.current_group_filter, 11);
    }

    #[test]
    fn main_group_filter_plan_updates_target_tab_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 10,
            tab_index: 0,
            hover_idx: 3,
            sel_idx: 4,
            scroll_y: 180,
            current_group_filter: 11,
            tab_group_filters: [11, 22],
            selection_anchor: 2,
            context_row: 5,
            ..ClipListState::default()
        };
        list.selected_rows.insert(2);
        list.selected_rows.insert(4);

        let plan = list.group_filter_plan(1, 88);
        assert_eq!(
            plan,
            MainGroupFilterPlan {
                tab_index: 1,
                tab_group_filters: [11, 88],
                current_group_filter: 88,
                scroll_y: 0,
                clear_selection: true
            }
        );

        list.apply_group_filter_plan(plan);
        assert_eq!(list.tab_index, 1);
        assert_eq!(list.tab_group_filters, [11, 88]);
        assert_eq!(list.current_group_filter, 88);
        assert_eq!(list.scroll_y, 0);
        assert_eq!(list.hover_idx, -1);
        assert_eq!(list.sel_idx, -1);
        assert!(list.selected_rows.is_empty());
        assert_eq!(list.selection_anchor, -1);
        assert_eq!(list.context_row, -1);

        let fallback = list.group_filter_plan(99, 0);
        assert_eq!(fallback.tab_index, 0);
        assert_eq!(fallback.tab_group_filters, [0, 88]);
    }

    #[test]
    fn main_group_filter_menu_plan_marks_checked_entry_without_host_menu() {
        let groups = vec![
            ClipGroup {
                id: 7,
                category: 0,
                name: "工作".to_string(),
            },
            ClipGroup {
                id: 9,
                category: 0,
                name: "临时".to_string(),
            },
        ];

        assert_eq!(
            main_group_filter_menu_plan(0, &groups).entries[0],
            MainGroupFilterMenuEntry::All { checked: true }
        );

        let plan = main_group_filter_menu_plan(9, &groups);
        assert_eq!(
            plan.entries,
            vec![
                MainGroupFilterMenuEntry::All { checked: false },
                MainGroupFilterMenuEntry::Separator,
                MainGroupFilterMenuEntry::Group {
                    index: 0,
                    group_id: 7,
                    label: "工作".to_string(),
                    checked: false,
                },
                MainGroupFilterMenuEntry::Group {
                    index: 1,
                    group_id: 9,
                    label: "临时".to_string(),
                    checked: true,
                },
            ]
        );
    }

    #[test]
    fn main_keyboard_selection_plan_moves_and_extends_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 5,
            sel_idx: -1,
            ..ClipListState::default()
        };

        let first = list.keyboard_move_selection_plan(1, false).unwrap();
        assert_eq!(first.sel_idx, 0);
        assert!(first.selected_rows.is_empty());
        assert_eq!(first.selection_anchor, -1);
        list.apply_selection_plan(first);
        assert_eq!(list.sel_idx, 0);
        assert!(list.selected_rows.is_empty());

        let second = list.keyboard_move_selection_plan(1, false).unwrap();
        list.apply_selection_plan(second);
        assert_eq!(list.sel_idx, 1);
        assert_eq!(list.selection_anchor, -1);

        let extend = list.keyboard_move_selection_plan(1, true).unwrap();
        assert_eq!(extend.sel_idx, 2);
        assert_eq!(extend.selection_anchor, 1);
        assert_eq!(
            extend.selected_rows.iter().copied().collect::<Vec<_>>(),
            vec![1, 2]
        );
        list.apply_selection_plan(extend);
        assert_eq!(list.selected_visible_rows(), vec![1, 2]);

        let shrink = list.keyboard_move_selection_plan(-1, true).unwrap();
        assert_eq!(shrink.sel_idx, 1);
        assert_eq!(shrink.selection_anchor, 1);
        assert_eq!(
            shrink.selected_rows.iter().copied().collect::<Vec<_>>(),
            vec![1]
        );
        list.apply_selection_plan(shrink);
        assert_eq!(list.selected_visible_rows(), vec![1]);

        let clamp = list.keyboard_move_selection_plan(99, false).unwrap();
        assert_eq!(clamp.sel_idx, 2);
        list.apply_selection_plan(clamp);
        let clamp = list.keyboard_move_selection_plan(99, false).unwrap();
        assert_eq!(clamp.sel_idx, 3);
        list.apply_selection_plan(clamp);
        let clamp = list.keyboard_move_selection_plan(99, false).unwrap();
        assert_eq!(clamp.sel_idx, 4);

        let empty = ClipListState::default();
        assert!(empty.keyboard_move_selection_plan(1, false).is_none());
    }

    #[test]
    fn main_select_all_selection_plan_selects_visible_rows_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 4,
            sel_idx: 2,
            selection_anchor: 3,
            ..ClipListState::default()
        };
        list.selected_rows.insert(3);

        let plan = list.select_all_selection_plan();
        assert_eq!(plan.sel_idx, 2);
        assert_eq!(plan.selection_anchor, 0);
        assert_eq!(
            plan.selected_rows.iter().copied().collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        list.apply_selection_plan(plan);
        assert_eq!(list.sel_idx, 2);
        assert_eq!(list.selection_anchor, 0);
        assert_eq!(list.selected_visible_rows(), vec![0, 1, 2, 3]);

        let empty = ClipListState {
            visible_len: 0,
            sel_idx: -1,
            ..ClipListState::default()
        };
        let plan = empty.select_all_selection_plan();
        assert!(plan.selected_rows.is_empty());
        assert_eq!(plan.selection_anchor, 0);
    }

    #[test]
    fn main_escape_shortcut_plan_prioritizes_selection_search_then_window() {
        let mut list = ClipListState {
            visible_len: 4,
            sel_idx: 2,
            search_on: true,
            ..ClipListState::default()
        };
        list.selected_rows.insert(1);
        assert_eq!(
            list.escape_shortcut_plan(),
            MainShortcutEscapePlan::ClearSelection
        );

        list.selected_rows.clear();
        assert_eq!(
            list.escape_shortcut_plan(),
            MainShortcutEscapePlan::CloseSearch
        );

        list.search_on = false;
        assert_eq!(
            list.escape_shortcut_plan(),
            MainShortcutEscapePlan::HideWindow
        );
    }

    #[test]
    fn main_activate_selection_plan_uses_combined_paste_only_for_multi_selection() {
        let mut list = ClipListState {
            visible_len: 4,
            sel_idx: -1,
            ..ClipListState::default()
        };
        assert_eq!(
            list.activate_selection_plan(),
            MainActivateSelectionPlan::PasteSelection
        );

        list.context_row = 2;
        assert_eq!(
            list.activate_selection_plan(),
            MainActivateSelectionPlan::PasteSelection
        );

        list.context_row = -1;
        list.sel_idx = 1;
        assert_eq!(
            list.activate_selection_plan(),
            MainActivateSelectionPlan::PasteSelection
        );

        list.selected_rows.insert(3);
        assert_eq!(
            list.activate_selection_plan(),
            MainActivateSelectionPlan::CopySelectionThenPaste
        );
    }

    #[test]
    fn main_shortcut_row_command_plan_uses_focused_row_as_context() {
        let mut list = ClipListState {
            visible_len: 5,
            sel_idx: 3,
            context_row: 1,
            selection_anchor: 2,
            ..ClipListState::default()
        };
        list.selected_rows.insert(2);

        let plan = list.shortcut_row_command_plan(MainShortcutRowCommand::TogglePin);
        assert_eq!(plan.context_row, 3);
        assert_eq!(plan.command, MainShortcutRowCommand::TogglePin);

        list.apply_shortcut_row_command_plan(plan);
        assert_eq!(list.context_row, 3);
        assert_eq!(list.sel_idx, 3);
        assert_eq!(list.selection_anchor, 2);
        assert_eq!(list.selected_visible_rows(), vec![2, 3]);
    }

    #[test]
    fn main_context_menu_state_plan_tracks_row_context_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 10,
            sel_idx: 5,
            selection_anchor: 2,
            ..ClipListState::default()
        };
        list.selected_rows = BTreeSet::from([2, 3, 4, 5]);

        let plan = list.context_menu_state_plan(4, false, false).unwrap();
        assert_eq!(
            plan,
            MainContextMenuStatePlan {
                row: 4,
                sel_idx: 4,
                selected_rows: BTreeSet::from([2, 3, 4, 5]),
                selection_anchor: 2,
                context_row: 4,
                context_selection_count: 4,
            }
        );
        list.apply_context_menu_state_plan(plan);
        assert_eq!(list.context_row, 4);
        assert_eq!(list.selected_visible_rows(), vec![2, 3, 4, 5]);

        let selection_plan = list.context_row_selection_plan().unwrap();
        assert_eq!(selection_plan, MainContextRowSelectionPlan { sel_idx: 4 });
        list.sel_idx = 1;
        list.apply_context_row_selection_plan(selection_plan);
        assert_eq!(list.sel_idx, 4);

        let plan = list.context_menu_state_plan(8, false, false).unwrap();
        assert_eq!(plan.selected_rows, BTreeSet::new());
        assert_eq!(plan.selection_anchor, 8);
        assert_eq!(plan.context_selection_count, 1);
        list.apply_context_menu_state_plan(plan);
        assert_eq!(list.selected_visible_rows(), vec![8]);
        assert_eq!(list.context_row, 8);

        let finish = list.context_menu_finish_plan();
        assert_eq!(finish, MainContextMenuFinishPlan { context_row: -1 });
        list.apply_context_menu_finish_plan(finish);
        assert_eq!(list.context_row, -1);

        assert_eq!(list.context_menu_state_plan(-1, false, false), None);
        assert_eq!(list.context_menu_state_plan(10, false, false), None);
        assert_eq!(list.context_row_selection_plan(), None);
    }

    #[test]
    fn main_scroll_to_top_release_plan_resets_scroll_without_host_actions() {
        let mut list = ClipListState {
            scroll_y: 240,
            ..ClipListState::default()
        };

        let plan = list.scroll_to_top_release_plan(true);
        assert_eq!(
            plan,
            MainScrollToTopReleasePlan {
                down_scroll_to_top: false,
                scroll_y: 0,
                show_scrollbar_feedback: true,
            }
        );
        list.apply_scroll_to_top_release_plan(plan);
        assert_eq!(list.scroll_y, 0);

        list.scroll_y = 180;
        let plan = list.scroll_to_top_release_plan(false);
        assert_eq!(
            plan,
            MainScrollToTopReleasePlan {
                down_scroll_to_top: false,
                scroll_y: 180,
                show_scrollbar_feedback: false,
            }
        );
        list.apply_scroll_to_top_release_plan(plan);
        assert_eq!(list.scroll_y, 180);
    }

    #[test]
    fn main_scroll_update_plan_applies_position_without_host_actions() {
        let mut list = ClipListState {
            scroll_y: 120,
            ..ClipListState::default()
        };

        let unchanged = list.scroll_position_update_plan(120);
        assert_eq!(
            unchanged,
            MainScrollUpdate {
                scroll_y: 120,
                changed: false,
            }
        );
        list.apply_scroll_update(unchanged);
        assert_eq!(list.scroll_y, 120);

        let changed = list.scroll_position_update_plan(360);
        assert_eq!(
            changed,
            MainScrollUpdate {
                scroll_y: 360,
                changed: true,
            }
        );
        list.apply_scroll_update(changed);
        assert_eq!(list.scroll_y, 360);
    }

    #[test]
    fn main_search_filter_apply_plan_resets_focus_and_scroll_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 8,
            sel_idx: 5,
            scroll_y: 240,
            selection_anchor: 2,
            ..ClipListState::default()
        };
        list.selected_rows = BTreeSet::from([2, 3, 4, 5]);

        let plan = list.search_filter_apply_plan();
        assert_eq!(
            plan,
            MainSearchFilterApplyPlan {
                sel_idx: 0,
                scroll_y: 0,
            }
        );
        list.apply_search_filter_plan(plan);
        assert_eq!(list.sel_idx, 0);
        assert_eq!(list.scroll_y, 0);
        assert_eq!(list.selected_rows, BTreeSet::from([2, 3, 4, 5]));
        assert_eq!(list.selection_anchor, 2);
    }

    #[test]
    fn main_search_reset_plan_clears_query_and_selection_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 8,
            search_on: true,
            search_text: "clip".to_string(),
            sel_idx: 5,
            hover_idx: 4,
            context_row: 3,
            selection_anchor: 2,
            ..ClipListState::default()
        };
        list.selected_rows = BTreeSet::from([2, 3, 4, 5]);

        let plan = list.search_reset_plan().unwrap();
        assert_eq!(
            plan,
            MainSearchResetPlan {
                search_on: false,
                search_text: String::new(),
                clear_selection: true,
            }
        );
        list.apply_search_reset_plan(plan);
        assert!(!list.search_on);
        assert!(list.search_text.is_empty());
        assert_eq!(list.sel_idx, -1);
        assert_eq!(list.hover_idx, -1);
        assert_eq!(list.context_row, -1);
        assert!(list.selected_rows.is_empty());
        assert_eq!(list.selection_anchor, -1);
        assert_eq!(list.search_reset_plan(), None);

        let text_only = ClipListState {
            search_text: "stale".to_string(),
            ..ClipListState::default()
        };
        assert!(text_only.search_reset_plan().is_some());
    }

    #[test]
    fn main_row_release_state_plan_clears_press_and_updates_focus_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 6,
            sel_idx: 1,
            ..ClipListState::default()
        };

        let accepted = MainRowRelease {
            down_row: 2,
            release_row: 2,
            accepted: true,
        };
        let plan = list.row_release_state_plan(accepted);
        assert_eq!(
            plan,
            MainRowReleaseStatePlan {
                down_row: -1,
                down_x: 0,
                down_y: 0,
                sel_idx: 2,
            }
        );
        list.apply_row_release_state_plan(plan);
        assert_eq!(list.sel_idx, 2);

        let rejected = MainRowRelease {
            down_row: 3,
            release_row: -1,
            accepted: false,
        };
        let plan = list.row_release_state_plan(rejected);
        assert_eq!(
            plan,
            MainRowReleaseStatePlan {
                down_row: -1,
                down_x: 0,
                down_y: 0,
                sel_idx: 2,
            }
        );
        list.apply_row_release_state_plan(plan);
        assert_eq!(list.sel_idx, 2);
    }

    #[test]
    fn main_pointer_up_press_clear_plan_clears_press_without_changing_focus() {
        let list = ClipListState {
            visible_len: 6,
            sel_idx: 4,
            ..ClipListState::default()
        };

        assert_eq!(
            list.pointer_up_press_clear_plan(),
            MainPointerUpPressClearPlan {
                down_row: -1,
                down_x: 0,
                down_y: 0,
            }
        );
        assert_eq!(list.sel_idx, 4);
    }

    #[test]
    fn main_row_pointer_down_focus_plan_updates_focus_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 4,
            sel_idx: 1,
            ..ClipListState::default()
        };

        let plan = list.row_pointer_down_focus_plan(2).unwrap();
        assert_eq!(plan, MainRowPointerDownFocusPlan { sel_idx: 2 });
        list.apply_row_pointer_down_focus_plan(plan);
        assert_eq!(list.sel_idx, 2);

        assert_eq!(list.row_pointer_down_focus_plan(-1), None);
        assert_eq!(list.row_pointer_down_focus_plan(4), None);
    }

    #[test]
    fn main_row_double_click_state_plan_focuses_then_clears_without_host_actions() {
        let mut list = ClipListState {
            visible_len: 5,
            sel_idx: -1,
            hover_idx: 3,
            ..ClipListState::default()
        };

        let plan = list.row_double_click_state_plan(2).unwrap();
        assert_eq!(
            plan,
            MainRowDoubleClickStatePlan {
                paste_sel_idx: 2,
                finish_sel_idx: -1,
                finish_hover_idx: -1,
            }
        );
        list.apply_row_double_click_focus_plan(plan);
        assert_eq!(list.sel_idx, 2);
        assert_eq!(list.hover_idx, 3);

        list.apply_row_double_click_finish_plan(plan);
        assert_eq!(list.sel_idx, -1);
        assert_eq!(list.hover_idx, -1);

        assert_eq!(list.row_double_click_state_plan(-1), None);
        assert_eq!(list.row_double_click_state_plan(5), None);
    }

    #[test]
    fn main_clip_list_selection_state_is_platform_neutral() {
        let mut list = ClipListState::default();
        list.apply_visible_len(10);
        list.apply_primary_pointer_selection(2, true, false);
        list.apply_primary_pointer_selection(4, true, false);
        assert_eq!(list.selected_visible_rows(), vec![2, 4]);
        assert_eq!(list.selected_count(), 2);

        list.apply_primary_pointer_selection(6, false, true);
        assert_eq!(list.selected_visible_rows(), vec![4, 5, 6]);
        assert!(list.row_is_selected(5));

        list.apply_context_pointer_selection(5, false, false);
        assert_eq!(list.selected_visible_rows(), vec![4, 5, 6]);
        list.apply_context_pointer_selection(8, false, false);
        assert_eq!(list.selected_visible_rows(), vec![8]);

        list.context_row = 3;
        list.clear_selection();
        list.context_row = 3;
        assert_eq!(list.context_selection_count(), 1);
        list.apply_visible_len(2);
        assert!(list.selected_source_indices().is_empty());
    }
}
