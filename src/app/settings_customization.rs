use super::prelude::*;
use crate::app_core::customization::{
    row_action_order, validate_category_tabs, CategoryTab, CategoryTabMode, QuickRowAction, MAX_CATEGORY_TABS,
};
use crate::platform::string::to_wide;
use windows_sys::Win32::Graphics::Gdi::{HBRUSH, HFONT};

pub(super) const OPEN_CATEGORIES: isize = 6220;
pub(super) const OPEN_QUICK_ACTIONS: isize = 6221;
const LIST: usize = 8201;
const NAME: usize = 8202;
const SOURCE: usize = 8203;
const GROUP: usize = 8204;
const KIND: usize = 8205;
const VISIBLE: usize = 8206;
const ADD: usize = 8207;
const REMOVE: usize = 8208;
const UP: usize = 8209;
const DOWN: usize = 8210;
const SAVE: usize = 8211;
const CANCEL: usize = 8212;

struct Editor {
    settings: AppSettings,
    categories: bool,
    selected: usize,
    loading: bool,
    accepted: bool,
    list: HWND,
    name: HWND,
    source: HWND,
    group: HWND,
    kind: HWND,
    visible: HWND,
    status: HWND,
    group_ids: Vec<i64>,
    kind_keys: Vec<&'static str>,
    actions: Vec<QuickRowAction>,
    font: HFONT,
    brush: HBRUSH,
    control_brush: HBRUSH,
    surface_brush: HBRUSH,
}
impl Drop for Editor {
    fn drop(&mut self) {
        platform_gdi::delete_object(self.font as _);
        platform_gdi::delete_object(self.brush as _);
        platform_gdi::delete_object(self.control_brush as _);
        platform_gdi::delete_object(self.surface_brush as _);
    }
}
unsafe fn data(hwnd: HWND) -> *mut Editor {
    platform_window::user_data(hwnd) as *mut Editor
}
unsafe fn combo_add(hwnd: HWND, label: &str) {
    platform_window::send_message(hwnd, CB_ADDSTRING, 0, to_wide(label).as_ptr() as isize);
}
unsafe fn selection(hwnd: HWND) -> usize {
    platform_window::send_message(hwnd, CB_GETCURSEL, 0, 0).max(0) as usize
}
unsafe fn checked(hwnd: HWND) -> bool {
    platform_window::user_data(hwnd) != 0
}
unsafe fn check(hwnd: HWND, value: bool) {
    platform_window::set_user_data(hwnd, value as isize);
    platform_gdi::invalidate_rect(hwnd, null(), 0);
}
fn action_enabled(settings: &AppSettings, action: QuickRowAction) -> bool {
    match action {
        QuickRowAction::Delete => settings.quick_delete_button,
        QuickRowAction::Pin => settings.quick_pin_button,
        QuickRowAction::Edit => settings.quick_edit_button,
    }
}
unsafe fn commit(editor: &mut Editor) {
    if editor.loading {
        return;
    }
    if editor.categories {
        if let Some(tab) = editor.settings.category_tabs.get_mut(editor.selected) {
            tab.name = platform_window::text(editor.name).trim().to_string();
            let mode = selection(editor.source);
            tab.mode = if mode == 0 { CategoryTabMode::Manual } else { CategoryTabMode::Filter };
            tab.source = mode.saturating_sub(1).min(1);
            tab.group_id = if tab.is_manual() { 0 } else { editor
                .group_ids
                .get(selection(editor.group))
                .copied()
                .unwrap_or(0) };
            tab.kind = if tab.is_manual() { "all".into() } else { editor
                .kind_keys
                .get(selection(editor.kind))
                .copied()
                .unwrap_or("all")
                .into() };
            tab.visible = checked(editor.visible);
        }
    } else if let Some(action) = editor.actions.get(editor.selected).copied() {
        let value = checked(editor.visible);
        match action {
            QuickRowAction::Delete => editor.settings.quick_delete_button = value,
            QuickRowAction::Pin => editor.settings.quick_pin_button = value,
            QuickRowAction::Edit => editor.settings.quick_edit_button = value,
        }
    }
}
unsafe fn refresh_list(editor: &mut Editor) {
    editor.loading = true;
    platform_window::send_message(editor.list, WM_SETREDRAW, 0, 0);
    platform_window::send_message(editor.list, LB_RESETCONTENT, 0, 0);
    let labels: Vec<String> = if editor.categories {
        editor
            .settings
            .category_tabs
            .iter()
            .map(|t| {
                format!(
                    "{}{}{}",
                    if t.name.is_empty() {
                        "未命名"
                    } else {
                        &t.name
                    },
                    if t.is_manual() { "（手动）" } else { "（自动筛选）" },
                    if t.visible { "" } else { "（隐藏）" }
                )
            })
            .collect()
    } else {
        editor
            .actions
            .iter()
            .map(|&a| {
                format!(
                    "{}{}",
                    a.label(),
                    if action_enabled(&editor.settings, a) {
                        ""
                    } else {
                        "（隐藏）"
                    }
                )
            })
            .collect()
    };
    for label in labels {
        platform_window::send_message(
            editor.list,
            LB_ADDSTRING,
            0,
            to_wide(&label).as_ptr() as isize,
        );
    }
    platform_window::send_message(editor.list, LB_SETCURSEL, editor.selected, 0);
    platform_window::send_message(editor.list, WM_SETREDRAW, 1, 0);
    platform_gdi::invalidate_rect(editor.list, null(), 0);
    editor.loading = false;
}
unsafe fn load_fields(editor: &mut Editor) {
    editor.loading = true;
    if editor.categories {
        let tab = editor.settings.category_tabs[editor.selected].clone();
        platform_window::set_text(editor.name, &tab.name);
        platform_window::send_message(editor.source, CB_SETCURSEL, if tab.is_manual() { 0 } else { tab.source + 1 }, 0);
        platform_window::set_enabled(editor.group, !tab.is_manual());
        platform_window::set_enabled(editor.kind, !tab.is_manual());
        editor.group_ids = vec![0];
        platform_window::send_message(editor.group, CB_RESETCONTENT, 0, 0);
        combo_add(editor.group, "全部分组");
        for group in db_load_groups(tab.source as i64) {
            editor.group_ids.push(group.id);
            combo_add(editor.group, &group.name);
        }
        if tab.group_id > 0 && !editor.group_ids.contains(&tab.group_id) {
            editor.group_ids.push(tab.group_id);
            combo_add(editor.group, "原分组已删除");
        }
        platform_window::send_message(
            editor.group,
            CB_SETCURSEL,
            editor
                .group_ids
                .iter()
                .position(|&id| id == tab.group_id)
                .unwrap_or(0),
            0,
        );
        editor.kind_keys = if tab.source == 1 {
            vec!["all", "phrase"]
        } else {
            vec!["all", "text", "image", "files"]
        };
        platform_window::send_message(editor.kind, CB_RESETCONTENT, 0, 0);
        for key in &editor.kind_keys {
            combo_add(
                editor.kind,
                match *key {
                    "text" => "文本",
                    "image" => "图片",
                    "files" => "文件",
                    "phrase" => "短语",
                    _ => "全部类型",
                },
            );
        }
        platform_window::send_message(
            editor.kind,
            CB_SETCURSEL,
            editor
                .kind_keys
                .iter()
                .position(|key| *key == tab.kind)
                .unwrap_or(0),
            0,
        );
        check(editor.visible, tab.visible);
    } else {
        check(
            editor.visible,
            action_enabled(&editor.settings, editor.actions[editor.selected]),
        );
    }
    editor.loading = false;
}
unsafe fn error(hwnd: HWND, message: &str) {
    let ptr = data(hwnd);
    if !ptr.is_null() {
        platform_window::set_text((*ptr).status, message);
        platform_gdi::invalidate_rect((*ptr).status, null(), 0);
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let ptr = data(hwnd);
    if ptr.is_null() {
        return platform_window::default_window_proc(hwnd, msg, w, l);
    }
    let editor = &mut *ptr;
    match msg {
        WM_DRAWITEM if l != 0 => {
            let item = &*(l as *const DRAWITEMSTRUCT);
            super::themed_controls::draw_item(
                item,
                item.CtlID as usize == SAVE,
                (item.CtlID as usize == VISIBLE).then(|| checked(item.hwndItem)),
            )
        }
        WM_MEASUREITEM if l != 0 => {
            let item = &mut *(l as *mut windows_sys::Win32::UI::Controls::MEASUREITEMSTRUCT);
            item.itemHeight = (24 * platform_dpi::layout_dpi_for_window(hwnd).max(96) / 96) as u32;
            1
        }
        WM_CLOSE => {
            platform_window::destroy(hwnd);
            0
        }
        WM_NCDESTROY => {
            platform_window::set_user_data(hwnd, 0);
            platform_window::default_window_proc(hwnd, msg, w, l)
        }
        WM_COMMAND => {
            if editor.loading {
                return 0;
            }
            let id = w & 0xffff;
            let code = (w >> 16) & 0xffff;
            if id == LIST && code == LBN_SELCHANGE as usize {
                let next = platform_window::send_message(editor.list, LB_GETCURSEL, 0, 0);
                if next >= 0 {
                    commit(editor);
                    editor.selected = next as usize;
                    refresh_list(editor);
                    load_fields(editor);
                }
                return 0;
            }
            if id == SOURCE && code == CBN_SELCHANGE as usize {
                commit(editor);
                let tab = &mut editor.settings.category_tabs[editor.selected];
                tab.group_id = 0;
                tab.kind = "all".into();
                load_fields(editor);
                return 0;
            }
            match id {
                VISIBLE if code == BN_CLICKED as usize => {
                    check(editor.visible, !checked(editor.visible));
                }
                ADD if editor.categories => {
                    commit(editor);
                    if editor.settings.category_tabs.len() >= MAX_CATEGORY_TABS {
                        error(hwnd, "最多可设置 32 个分类标签。");
                        return 0;
                    }
                    editor.settings.category_tabs.push(CategoryTab {
                        id: format!(
                            "category-{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos()
                        ),
                        ..Default::default()
                    });
                    editor.selected = editor.settings.category_tabs.len() - 1;
                    refresh_list(editor);
                    load_fields(editor);
                    platform_input::set_focus(editor.name);
                }
                REMOVE if editor.categories => {
                    commit(editor);
                    if editor.settings.category_tabs.len() == 1 {
                        error(hwnd, "至少保留一个分类标签。");
                        return 0;
                    }
                    editor.settings.category_tabs.remove(editor.selected);
                    editor.selected = editor.selected.min(editor.settings.category_tabs.len() - 1);
                    refresh_list(editor);
                    load_fields(editor);
                }
                UP | DOWN => {
                    commit(editor);
                    let count = if editor.categories {
                        editor.settings.category_tabs.len()
                    } else {
                        editor.actions.len()
                    };
                    let next = if id == UP {
                        editor.selected.saturating_sub(1)
                    } else {
                        (editor.selected + 1).min(count - 1)
                    };
                    if editor.categories {
                        editor.settings.category_tabs.swap(editor.selected, next);
                    } else {
                        editor.actions.swap(editor.selected, next);
                    }
                    editor.selected = next;
                    refresh_list(editor);
                    load_fields(editor);
                }
                SAVE => {
                    commit(editor);
                    if editor.categories {
                        if let Err(message) = validate_category_tabs(&editor.settings.category_tabs)
                        {
                            error(hwnd, message);
                            return 0;
                        }
                    }
                    editor.settings.quick_action_order = editor
                        .actions
                        .iter()
                        .map(|a| a.key())
                        .collect::<Vec<_>>()
                        .join(",");
                    editor.accepted = true;
                    platform_window::destroy(hwnd);
                }
                CANCEL => {
                    platform_window::destroy(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORSTATIC => {
            let theme = Theme::default();
            platform_gdi::set_text_color(
                w as HDC,
                if l as HWND == editor.status {
                    if platform_appearance::is_dark_mode() {
                        rgb(255, 160, 160)
                    } else {
                        rgb(180, 40, 40)
                    }
                } else {
                    theme.text
                },
            );
            let (color, brush) = if msg == WM_CTLCOLOREDIT {
                (theme.control_bg, editor.control_brush)
            } else if msg == WM_CTLCOLORLISTBOX {
                (theme.surface, editor.surface_brush)
            } else {
                (theme.bg, editor.brush)
            };
            platform_gdi::set_bk_color(w as HDC, color);
            brush as isize
        }
        WM_ERASEBKGND => {
            if let Some(rect) = platform_window::client_rect(hwnd) {
                platform_gdi::fill_rect(w as HDC, &rect, editor.brush);
            }
            1
        }
        _ => platform_window::default_window_proc(hwnd, msg, w, l),
    }
}

unsafe fn show_impl(
    parent: HWND,
    settings: AppSettings,
    categories: bool,
    visible: bool,
    ready: impl FnOnce(HWND),
) -> Option<AppSettings> {
    let class = to_wide("ZSClipCustomization");
    platform_window::register_class_ex(&WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(proc),
        hInstance: platform_window::module_handle(),
        hCursor: platform_window::arrow_cursor(),
        lpszClassName: class.as_ptr(),
        ..zeroed()
    });
    let dpi = platform_dpi::layout_dpi_for_window(parent).max(96) as i32;
    let px = |n: i32| n * dpi / 96;
    let work = platform_monitor::nearest_work_rect_for_window(parent);
    let width = px(700);
    let height = px(if categories { 545 } else { 375 });
    let hwnd = platform_window::create_window_ex(
        WS_EX_DLGMODALFRAME,
        class.as_ptr(),
        to_wide(if categories {
            "分类标签"
        } else {
            "快捷按钮"
        })
        .as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN,
        work.left + (work.right - work.left - width) / 2,
        work.top + (work.bottom - work.top - height) / 2,
        width,
        height,
        parent,
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    if hwnd.is_null() {
        return None;
    }
    let font = platform_gdi::create_font_w(
        -px(14),
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(ui_text_font_family()).as_ptr(),
    );
    let mut editor = Box::new(Editor {
        actions: row_action_order(&settings.quick_action_order),
        settings,
        categories,
        selected: 0,
        loading: true,
        accepted: false,
        list: null_mut(),
        name: null_mut(),
        source: null_mut(),
        group: null_mut(),
        kind: null_mut(),
        visible: null_mut(),
        status: null_mut(),
        group_ids: vec![],
        kind_keys: vec![],
        font,
        brush: platform_gdi::create_solid_brush(Theme::default().bg),
        control_brush: platform_gdi::create_solid_brush(Theme::default().control_bg),
        surface_brush: platform_gdi::create_solid_brush(Theme::default().surface),
    });
    platform_window::set_user_data(hwnd, (&mut *editor as *mut Editor) as isize);
    let create =
        |class: &str, text: &str, style: u32, id: usize, x: i32, y: i32, w: i32, h: i32| {
            let style = if class == "BUTTON" {
                (style & !0xf) | BS_OWNERDRAW as u32
            } else if class == "COMBOBOX" {
                style | CBS_OWNERDRAWFIXED as u32 | CBS_HASSTRINGS as u32
            } else {
                style
            };
            let child = platform_window::create_window_ex(
                0,
                to_wide(class).as_ptr(),
                to_wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                px(x),
                px(y),
                px(w),
                px(h),
                hwnd,
                id as _,
                platform_window::module_handle(),
                null(),
            );
            platform_window::send_message(child, WM_SETFONT, font as usize, 1);
            platform_appearance::apply_dark_mode_to_window(child);
            platform_appearance::set_window_theme(
                child,
                if platform_appearance::is_dark_mode() {
                    "DarkMode_Explorer"
                } else {
                    "Explorer"
                },
            );
            child
        };
    create(
        "STATIC",
        if categories {
            "手动标签新建为空，可右键添加记录；自动筛选按来源、分组和类型显示。"
        } else {
            "选择按钮设置显示状态；上移、下移调整从左到右的顺序。"
        },
        0,
        0,
        16,
        16,
        650,
        26,
    );
    let list_h = if categories { 340 } else { 170 };
    editor.list = create(
        "LISTBOX",
        "",
        WS_TABSTOP
            | WS_BORDER
            | WS_VSCROLL
            | LBS_NOTIFY as u32
            | LBS_OWNERDRAWFIXED as u32
            | LBS_HASSTRINGS as u32,
        LIST,
        16,
        50,
        230,
        list_h,
    );
    let button_y = if categories { 404 } else { 234 };
    create("BUTTON", "上移", WS_TABSTOP, UP, 16, button_y, 108, 30);
    create("BUTTON", "下移", WS_TABSTOP, DOWN, 138, button_y, 108, 30);
    if categories {
        create("BUTTON", "新增标签", WS_TABSTOP, ADD, 16, 444, 108, 30);
        create("BUTTON", "删除标签", WS_TABSTOP, REMOVE, 138, 444, 108, 30);
        create("STATIC", "名称", 0, 0, 272, 52, 100, 24);
        editor.name = create(
            "EDIT",
            "",
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            NAME,
            272,
            80,
            390,
            32,
        );
        platform_window::send_message(editor.name, 0x00C5, 32, 0);
        create("STATIC", "内容方式", 0, 0, 272, 128, 150, 24);
        editor.source = create(
            "COMBOBOX",
            "",
            WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            SOURCE,
            272,
            155,
            390,
            150,
        );
        combo_add(editor.source, "手动添加（新标签默认）");
        combo_add(editor.source, "自动筛选：复制记录");
        combo_add(editor.source, "自动筛选：常用短语");
        create("STATIC", "分组", 0, 0, 272, 203, 150, 24);
        editor.group = create(
            "COMBOBOX",
            "",
            WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            GROUP,
            272,
            230,
            390,
            180,
        );
        create("STATIC", "文件类型", 0, 0, 272, 278, 150, 24);
        editor.kind = create(
            "COMBOBOX",
            "",
            WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            KIND,
            272,
            305,
            390,
            160,
        );
    }
    editor.visible = create(
        "BUTTON",
        if categories {
            "显示此标签"
        } else {
            "显示此按钮"
        },
        WS_TABSTOP | BS_AUTOCHECKBOX as u32,
        VISIBLE,
        272,
        if categories { 366 } else { 82 },
        260,
        30,
    );
    editor.status = create(
        "STATIC",
        "",
        0,
        8220,
        272,
        if categories { 411 } else { 184 },
        390,
        48,
    );
    let footer = if categories { 474 } else { 290 };
    create(
        "BUTTON",
        "确定",
        WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        SAVE,
        448,
        footer,
        100,
        32,
    );
    create("BUTTON", "取消", WS_TABSTOP, CANCEL, 562, footer, 100, 32);
    refresh_list(&mut editor);
    load_fields(&mut editor);
    crate::settings_ui_host::refresh_settings_control_themes(hwnd);
    platform_appearance::apply_dark_mode_to_window(hwnd);
    let parent_enabled = platform_window::is_enabled(parent);
    if parent_enabled && visible {
        platform_window::set_enabled(parent, false);
    }
    if visible {
        platform_window::show(hwnd);
        platform_window::set_foreground(hwnd);
        platform_input::set_focus(editor.list);
    }
    ready(hwnd);
    let mut message: MSG = zeroed();
    while platform_window::exists(hwnd) {
        let result = platform_window::get_message(&mut message);
        if result <= 0 {
            if result == 0 {
                platform_window::post_quit_message(message.wParam as i32);
            }
            break;
        }
        if platform_window::root_ancestor(message.hwnd) == hwnd
            && message.message == WM_KEYDOWN
            && message.wParam == 27
        {
            platform_window::destroy(hwnd);
            continue;
        }
        if windows_sys::Win32::UI::WindowsAndMessaging::IsDialogMessageW(hwnd, &message) == 0 {
            platform_window::translate_message(&message);
            platform_window::dispatch_message(&message);
        }
    }
    if platform_window::exists(hwnd) {
        platform_window::destroy(hwnd);
    }
    if parent_enabled && visible && platform_window::exists(parent) {
        platform_window::set_enabled(parent, true);
        platform_window::set_foreground(parent);
    }
    editor.accepted.then(|| editor.settings.clone())
}

pub(super) unsafe fn open_from_settings(hwnd: HWND, id: isize) {
    let ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if ptr.is_null() {
        return;
    }
    let generation = (*ptr).app_data_generation;
    let draft = (*ptr).draft.clone();
    if let Some(updated) = show_impl(hwnd, draft, id == OPEN_CATEGORIES, true, |_| {}) {
        let ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
        if !ptr.is_null() && (*ptr).app_data_generation == generation {
            (*ptr).draft.category_tabs = updated.category_tabs;
            (*ptr).draft.quick_delete_button = updated.quick_delete_button;
            (*ptr).draft.quick_pin_button = updated.quick_pin_button;
            (*ptr).draft.quick_edit_button = updated.quick_edit_button;
            (*ptr).draft.quick_action_order = updated.quick_action_order;
            platform_gdi::invalidate_rect(hwnd, null(), 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe fn command(hwnd: HWND, id: usize) {
        platform_window::send_message(hwnd, WM_COMMAND, id, 0);
    }
    unsafe fn select_row(hwnd: HWND, index: usize) {
        let list = platform_window::child(hwnd, LIST as i32);
        platform_window::send_message(list, LB_SETCURSEL, index, 0);
        platform_window::send_message(
            hwnd,
            WM_COMMAND,
            LIST | ((LBN_SELCHANGE as usize) << 16),
            list as isize,
        );
    }
    #[test]
    fn native_category_editor_adds_renames_deletes_and_saves_without_touching_history() {
        crate::db_runtime::with_test_db(|| unsafe {
            let result = show_impl(null_mut(), AppSettings::default(), true, false, |hwnd| {
                platform_window::set_text(platform_window::child(hwnd, NAME as i32), "我的收集箱");
                command(hwnd, ADD);
                platform_window::set_text(platform_window::child(hwnd, NAME as i32), "项目照片");
                let source = platform_window::child(hwnd, SOURCE as i32);
                platform_window::send_message(source, CB_SETCURSEL, 1, 0);
                platform_window::send_message(hwnd, WM_COMMAND, SOURCE | ((CBN_SELCHANGE as usize)<<16), source as isize);
                platform_window::send_message(
                    platform_window::child(hwnd, KIND as i32),
                    CB_SETCURSEL,
                    2,
                    0,
                );
                command(hwnd, UP);
                select_row(hwnd, 2);
                command(hwnd, REMOVE);
                platform_window::post_hwnd_message(hwnd, WM_COMMAND, SAVE, 0);
            })
            .expect("valid editor save");
            assert_eq!(result.category_tabs.len(), 2);
            assert_eq!(result.category_tabs[0].name, "我的收集箱");
            assert_eq!(result.category_tabs[1].name, "项目照片");
            assert_eq!(result.category_tabs[1].kind, "image");
            assert!(!result.category_tabs[1].is_manual());
            assert!(show_impl(null_mut(), result.clone(), true, false, |hwnd| {
                platform_window::set_text(platform_window::child(hwnd, NAME as i32), "取消的修改");
                platform_window::post_hwnd_message(hwnd, WM_COMMAND, CANCEL, 0);
            })
            .is_none());
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn native_new_tag_is_manual_and_ignores_automatic_filter_fields() {
        crate::db_runtime::with_test_db(|| unsafe {
            let result = show_impl(null_mut(), AppSettings::default(), true, false, |hwnd| {
                command(hwnd, ADD);
                assert!(!platform_window::is_enabled(platform_window::child(hwnd, GROUP as i32)));
                assert!(!platform_window::is_enabled(platform_window::child(hwnd, KIND as i32)));
                platform_window::post_hwnd_message(hwnd, WM_COMMAND, SAVE, 0);
            }).unwrap();
            assert!(result.category_tabs[2].is_manual());
            assert_eq!(result.category_tabs[2].group_id, 0);
            assert_eq!(result.category_tabs[2].kind, "all");
            Ok(())
        }).unwrap();
    }
    #[test]
    fn native_quick_button_editor_updates_visibility_and_order() {
        unsafe {
            let result = show_impl(null_mut(), AppSettings::default(), false, false, |hwnd| {
                select_row(hwnd, 1);
                command(hwnd, VISIBLE);
                command(hwnd, UP);
                select_row(hwnd, 2);
                command(hwnd, VISIBLE);
                let save = platform_window::child(hwnd, SAVE as i32);
                assert_eq!(
                    platform_window::window_style(save) & 0xf,
                    BS_OWNERDRAW as u32
                );
                platform_window::post_hwnd_message(hwnd, WM_COMMAND, SAVE, 0);
            })
            .unwrap();
            assert!(
                result.quick_delete_button && result.quick_pin_button && result.quick_edit_button
            );
            assert_eq!(result.quick_action_order, "pin,delete,edit");
        }
    }
}
