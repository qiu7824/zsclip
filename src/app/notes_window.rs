//! On-demand native notes window. No editor or database stays resident after close.
use super::prelude::*;
use crate::notes_store::{NoteSummary, NotesStore};
use crate::platform::string::to_wide;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows_sys::Win32::Graphics::Gdi::{HBRUSH, HFONT, HGDIOBJ};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;

const CLASS: &str = "ZSClipNotes";
const ID_NEW: usize = 8101;
const ID_DELETE: usize = 8102;
const ID_LIST: usize = 8103;
const ID_EDITOR: usize = 8104;
const SAVE_TIMER: usize = 8105;
const EM_LIMITTEXT_NOTES: u32 = 0x00C5;
static WINDOW: AtomicIsize = AtomicIsize::new(0);
static TRANSITION: AtomicBool = AtomicBool::new(false);

struct Transition;
impl Drop for Transition {
    fn drop(&mut self) {
        TRANSITION.store(false, Ordering::Release);
    }
}

struct NotesWindow {
    store: NotesStore,
    rows: Vec<NoteSummary>,
    selected: Option<i64>,
    dirty: bool,
    loading: bool,
    list: HWND,
    editor: HWND,
    status: HWND,
    new_button: HWND,
    delete_button: HWND,
    font: HFONT,
    background: HBRUSH,
    control_background: HBRUSH,
    list_background: HBRUSH,
    failed: bool,
}

impl Drop for NotesWindow {
    fn drop(&mut self) {
        for object in [
            self.font as HGDIOBJ,
            self.background as HGDIOBJ,
            self.control_background as HGDIOBJ,
            self.list_background as HGDIOBJ,
        ] {
            if !object.is_null() {
                platform_gdi::delete_object(object);
            }
        }
    }
}

fn window() -> HWND {
    WINDOW.load(Ordering::Acquire) as HWND
}
unsafe fn state(hwnd: HWND) -> *mut NotesWindow {
    platform_window::user_data(hwnd) as *mut NotesWindow
}

pub(super) fn is_visible() -> bool {
    let hwnd = window();
    platform_window::exists(hwnd)
        && platform_window::is_visible(hwnd)
        && !platform_window::is_minimized(hwnd)
}

pub(super) fn is_editor_window(hwnd: HWND) -> bool {
    let notes = window();
    platform_window::exists(notes) && platform_window::root_ancestor(hwnd) == notes
}

unsafe fn status(hwnd: HWND, text: &str, failed: bool) {
    let ptr = state(hwnd);
    if !ptr.is_null() {
        (*ptr).failed = failed;
        platform_window::set_text((*ptr).status, text);
        platform_gdi::invalidate_rect((*ptr).status, null(), 0);
    }
}

unsafe fn report(hwnd: HWND, error: &str) {
    status(
        hwnd,
        &format!("{} {error}", tr("未保存：", "Not saved:")),
        true,
    );
    platform_dialog::WindowsDialogHost::new().show_message(
        hwnd,
        tr("便签", "Notes"),
        error,
        NativeDialogLevel::Error,
    );
}

unsafe fn refresh_list(hwnd: HWND) -> Result<(), String> {
    let ptr = state(hwnd);
    let rows = (*ptr).store.list()?;
    let list = (*ptr).list;
    let top = platform_window::send_message(list, LB_GETTOPINDEX, 0, 0);
    (*ptr).loading = true;
    platform_window::send_message(list, WM_SETREDRAW, 0, 0);
    platform_window::send_message(list, LB_RESETCONTENT, 0, 0);
    for note in &rows {
        let title = if note.title.is_empty() {
            tr("未命名便签", "Untitled")
        } else {
            &note.title
        };
        platform_window::send_message(list, LB_ADDSTRING, 0, to_wide(title).as_ptr() as isize);
    }
    let selected = rows
        .iter()
        .position(|note| Some(note.id) == (*ptr).selected);
    (*ptr).rows = rows;
    if let Some(index) = selected {
        platform_window::send_message(list, LB_SETCURSEL, index, 0);
    }
    platform_window::send_message(list, LB_SETTOPINDEX, top.max(0) as usize, 0);
    platform_window::send_message(list, WM_SETREDRAW, 1, 0);
    (*ptr).loading = false;
    platform_gdi::invalidate_rect(list, null(), 0);
    Ok(())
}

unsafe fn flush(hwnd: HWND) -> Result<(), String> {
    let ptr = state(hwnd);
    if ptr.is_null() || !(*ptr).dirty {
        return Ok(());
    }
    let Some(id) = (*ptr).selected else {
        return Err(tr("请先新建或选择一条便签。", "Create or select a note first.").into());
    };
    let body = platform_window::text((*ptr).editor);
    (*ptr).store.save(id, &body)?;
    (*ptr).dirty = false;
    timer::stop(hwnd, SAVE_TIMER);
    refresh_list(hwnd)?;
    status(
        hwnd,
        tr(
            "已自动保存 · 仅保存在本机",
            "Saved automatically · Local only",
        ),
        false,
    );
    Ok(())
}

unsafe fn select_note(hwnd: HWND, id: i64) -> Result<(), String> {
    flush(hwnd)?;
    let ptr = state(hwnd);
    let body = (*ptr).store.read(id)?;
    (*ptr).loading = true;
    let replaced = platform_window::set_text((*ptr).editor, &body);
    (*ptr).loading = false;
    if !replaced {
        return Err(tr(
            "无法载入便签内容，原内容仍保留。",
            "Unable to load the note; the previous content is retained.",
        )
        .into());
    }
    (*ptr).selected = Some(id);
    (*ptr).dirty = false;
    platform_window::set_enabled((*ptr).editor, true);
    platform_window::set_enabled((*ptr).delete_button, true);
    (*ptr).store.select(id)?;
    refresh_list(hwnd)?;
    status(
        hwnd,
        tr(
            "首行作为标题 · 自动保存 · Ctrl+N 新建",
            "First line is the title · Autosave · Ctrl+N for a new note",
        ),
        false,
    );
    Ok(())
}

unsafe fn new_note(hwnd: HWND) -> Result<(), String> {
    flush(hwnd)?;
    let ptr = state(hwnd);
    let id = (*ptr).store.create()?;
    select_note(hwnd, id)?;
    platform_input::set_focus((*ptr).editor);
    Ok(())
}

unsafe fn delete_note(hwnd: HWND) -> Result<(), String> {
    flush(hwnd)?;
    let ptr = state(hwnd);
    let Some(id) = (*ptr).selected else {
        return Ok(());
    };
    let accepted = platform_dialog::WindowsDialogHost::new().confirm(
        hwnd,
        tr("删除便签", "Delete note"),
        tr("删除选中的便签？", "Delete the selected note?"),
        NativeDialogLevel::Question,
        NativeDialogButtons::YesNo,
    );
    if accepted != NativeDialogResponse::Yes {
        return Ok(());
    }
    (*ptr).store.delete(id)?;
    (*ptr).selected = None;
    refresh_list(hwnd)?;
    if let Some(next) = (*ptr).rows.first().map(|note| note.id) {
        select_note(hwnd, next)?;
    } else {
        (*ptr).loading = true;
        platform_window::set_text((*ptr).editor, "");
        platform_window::set_enabled((*ptr).editor, false);
        platform_window::set_enabled((*ptr).delete_button, false);
        (*ptr).loading = false;
        status(
            hwnd,
            tr("点击“新建”开始记录", "Choose New to start a note"),
            false,
        );
    }
    Ok(())
}

pub(super) unsafe fn try_close() -> Result<(), String> {
    let hwnd = window();
    if !platform_window::exists(hwnd) {
        return Ok(());
    }
    if !platform_window::is_enabled(hwnd) {
        return Err(tr("请先关闭便签中的对话框。", "Close the note's dialog first.").into());
    }
    let ptr = state(hwnd);
    if !ptr.is_null() && WindowsImeHost::new().composition_active((*ptr).editor) {
        return Err(tr(
            "请先完成当前输入，再收起便签。",
            "Finish the current input composition before closing the note.",
        )
        .into());
    }
    flush(hwnd)?;
    platform_window::destroy(hwnd);
    schedule_hidden_memory_reclaim_after_activity();
    Ok(())
}

pub(super) unsafe fn toggle() {
    if TRANSITION.swap(true, Ordering::AcqRel) {
        return;
    }
    let _guard = Transition;
    if is_visible() && platform_window::root_ancestor(platform_window::foreground()) == window() {
        if let Err(error) = try_close() {
            report(window(), &error);
        }
    } else {
        show_impl(true);
    }
}

pub(super) unsafe fn open_or_focus() {
    if TRANSITION.swap(true, Ordering::AcqRel) {
        return;
    }
    let _guard = Transition;
    show_impl(true);
}

#[cfg(test)]
pub(super) unsafe fn open_hidden_for_test() -> HWND {
    show_impl(false);
    window()
}

unsafe fn show_impl(visible: bool) {
    let existing = window();
    if platform_window::exists(existing) {
        platform_window::restore(existing);
        platform_window::set_foreground(existing);
        let ptr = state(existing);
        if !ptr.is_null() {
            platform_input::set_focus((*ptr).editor);
        }
        return;
    }
    let store = match NotesStore::open(&data_dir().join("notes.sqlite3")) {
        Ok(store) => store,
        Err(error) => {
            platform_dialog::WindowsDialogHost::new().show_message(
                main_window_hwnd(),
                tr("便签", "Notes"),
                &error,
                NativeDialogLevel::Error,
            );
            return;
        }
    };
    let class = to_wide(CLASS);
    let definition = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        hInstance: platform_window::module_handle(),
        lpfnWndProc: Some(wnd_proc),
        hCursor: platform_window::arrow_cursor(),
        lpszClassName: class.as_ptr(),
        ..zeroed()
    };
    platform_window::register_class_ex(&definition);
    let point = platform_input::cursor_pos().unwrap_or(POINT { x: 100, y: 100 });
    let work = platform_monitor::nearest_work_rect_for_point(point);
    let scale = platform_dpi::layout_dpi_for_point(point).max(96) as i32;
    let width = (720 * scale / 96).min(work.right - work.left);
    let height = (500 * scale / 96).min(work.bottom - work.top);
    let hwnd = platform_window::create_window_ex(
        WS_EX_APPWINDOW,
        class.as_ptr(),
        to_wide(tr("便签", "Notes")).as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        point
            .x
            .clamp(work.left, (work.right - width).max(work.left)),
        point
            .y
            .clamp(work.top, (work.bottom - height).max(work.top)),
        width,
        height,
        main_window_hwnd(),
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    if hwnd.is_null() {
        return;
    }
    let boxed = Box::new(NotesWindow {
        store,
        rows: Vec::new(),
        selected: None,
        dirty: false,
        loading: true,
        list: null_mut(),
        editor: null_mut(),
        status: null_mut(),
        new_button: null_mut(),
        delete_button: null_mut(),
        font: null_mut(),
        background: null_mut(),
        control_background: null_mut(),
        list_background: null_mut(),
        failed: false,
    });
    platform_window::set_user_data(hwnd, Box::into_raw(boxed) as isize);
    WINDOW.store(hwnd as isize, Ordering::Release);
    let icon =
        platform_window::send_message(main_window_hwnd(), WM_GETICON, ICON_SMALL as usize, 0);
    if icon != 0 {
        platform_window::send_message(hwnd, WM_SETICON, ICON_SMALL as usize, icon);
    }
    let ptr = state(hwnd);
    let create = |class: &str, label: &str, style: u32, id: usize| {
        platform_window::create_window_ex(
            0,
            to_wide(class).as_ptr(),
            to_wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            0,
            0,
            0,
            0,
            hwnd,
            id as _,
            platform_window::module_handle(),
            null(),
        )
    };
    (*ptr).new_button = create("BUTTON", tr("新建", "New"), WS_TABSTOP|BS_OWNERDRAW as u32, ID_NEW);
    (*ptr).delete_button = create("BUTTON", tr("删除", "Delete"), WS_TABSTOP|BS_OWNERDRAW as u32, ID_DELETE);
    (*ptr).list = create(
        "LISTBOX",
        "",
        WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32 | LBS_OWNERDRAWFIXED as u32 | LBS_HASSTRINGS as u32,
        ID_LIST,
    );
    (*ptr).editor = create(
        "EDIT",
        "",
        WS_TABSTOP
            | WS_VSCROLL
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_WANTRETURN as u32,
        ID_EDITOR,
    );
    (*ptr).status = create("STATIC", "", 0, 0);
    if [
        (*ptr).list,
        (*ptr).editor,
        (*ptr).status,
        (*ptr).new_button,
        (*ptr).delete_button,
    ]
    .iter()
    .any(|handle| handle.is_null())
    {
        platform_window::destroy(hwnd);
        return;
    }
    platform_window::send_message((*ptr).editor, EM_LIMITTEXT_NOTES, 262144, 0);
    refresh_theme(hwnd);
    layout(hwnd);
    let result = refresh_list(hwnd).and_then(|_| {
        let selected = (*ptr)
            .store
            .selected()
            .filter(|id| (*ptr).rows.iter().any(|note| note.id == *id))
            .or_else(|| (*ptr).rows.first().map(|note| note.id));
        match selected {
            Some(id) => select_note(hwnd, id),
            None => new_note(hwnd),
        }
    });
    if let Err(error) = result {
        (*ptr).loading = false;
        platform_window::set_enabled((*ptr).editor, false);
        platform_window::set_enabled((*ptr).delete_button, false);
        report(hwnd, &error);
    }
    if visible {
        platform_window::show(hwnd);
        platform_window::set_foreground(hwnd);
        platform_input::set_focus((*ptr).editor);
    }
}

unsafe fn refresh_theme(hwnd: HWND) {
    let ptr = state(hwnd);
    if ptr.is_null() {
        return;
    }
    let th = Theme::default();
    for object in [
        (*ptr).font as HGDIOBJ,
        (*ptr).background as HGDIOBJ,
        (*ptr).control_background as HGDIOBJ,
        (*ptr).list_background as HGDIOBJ,
    ] {
        if !object.is_null() {
            platform_gdi::delete_object(object);
        }
    }
    let dpi = platform_dpi::layout_dpi_for_window(hwnd).max(96) as i32;
    (*ptr).font = platform_gdi::create_font_w(
        -14 * dpi / 96,
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
    (*ptr).background = platform_gdi::create_solid_brush(th.bg);
    (*ptr).control_background = platform_gdi::create_solid_brush(th.control_bg);
    (*ptr).list_background = platform_gdi::create_solid_brush(th.surface);
    for child in [
        (*ptr).new_button,
        (*ptr).delete_button,
        (*ptr).list,
        (*ptr).editor,
        (*ptr).status,
    ] {
        platform_window::send_message(child, WM_SETFONT, (*ptr).font as usize, 1);
        platform_appearance::apply_dark_mode_to_window(child);
        platform_appearance::set_window_theme(child, if platform_appearance::is_dark_mode(){if child==(*ptr).editor{"DarkMode_CFD"}else{"DarkMode_Explorer"}}else{"Explorer"});
    }
    platform_window::send_message((*ptr).list,LB_SETITEMHEIGHT,0,(24*dpi/96) as isize);
    platform_appearance::apply_dark_mode_to_window(hwnd);
    platform_gdi::invalidate_rect(hwnd, null(), 0);
}

pub(super) fn refresh_appearance() {
    if platform_window::exists(window()) {
        platform_window::post_hwnd_message(window(), WM_THEMECHANGED, 0, 0);
    }
}

unsafe fn layout(hwnd: HWND) {
    let ptr = state(hwnd);
    if ptr.is_null() {
        return;
    }
    let Some(rect) = platform_window::client_rect(hwnd) else {
        return;
    };
    let dpi = platform_dpi::layout_dpi_for_window(hwnd).max(96) as i32;
    let px = |v: i32| v * dpi / 96;
    let width = rect.right;
    let height = rect.bottom;
    let left = (width / 3).clamp(px(150), px(230));
    platform_window::move_window((*ptr).new_button, px(8), px(8), px(84), px(30), true);
    platform_window::move_window((*ptr).delete_button, px(100), px(8), px(84), px(30), true);
    platform_window::move_window(
        (*ptr).list,
        px(8),
        px(46),
        (left - px(16)).max(1),
        (height - px(80)).max(1),
        true,
    );
    platform_window::move_window(
        (*ptr).editor,
        left,
        px(46),
        (width - left - px(8)).max(1),
        (height - px(80)).max(1),
        true,
    );
    platform_window::move_window(
        (*ptr).status,
        px(8),
        (height - px(26)).max(0),
        (width - px(16)).max(1),
        px(22),
        true,
    );
}

pub(super) unsafe fn route_message(message: &MSG) -> bool {
    let hwnd = window();
    if !platform_window::exists(hwnd)
        || platform_window::root_ancestor(message.hwnd) != hwnd
        || message.message != WM_KEYDOWN
        || !platform_input::is_message_key_down(VK_CONTROL as u32)
    {
        return false;
    }
    let result = match message.wParam as u32 {
        0x53 => flush(hwnd),
        0x4E => new_note(hwnd),
        0x56 => {
            let ptr = state(hwnd);
            let ready = if (*ptr).selected.is_none() || !platform_window::is_enabled((*ptr).editor)
            {
                new_note(hwnd)
            } else {
                Ok(())
            };
            ready.map(|_| {
                platform_input::set_focus((*ptr).editor);
                platform_window::send_message((*ptr).editor, WM_PASTE, 0, 0);
            })
        }
        _ => return false,
    };
    if let Err(error) = result {
        report(hwnd, &error);
    }
    true
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCDESTROY {
        timer::stop(hwnd, SAVE_TIMER);
        let ptr = state(hwnd);
        platform_window::set_user_data(hwnd, 0);
        if !ptr.is_null() {
            drop(Box::from_raw(ptr));
        }
        let _ = WINDOW.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire);
        return platform_window::default_window_proc(hwnd, msg, w, l);
    }
    let ptr = state(hwnd);
    match msg {
        WM_MEASUREITEM if l!=0 => {
            let measure=&mut *(l as *mut windows_sys::Win32::UI::Controls::MEASUREITEMSTRUCT);
            measure.itemHeight=24*platform_dpi::layout_dpi_for_window(hwnd).max(96)/96;1
        }
        WM_DRAWITEM if l!=0 => {
            let item=&*(l as *const DRAWITEMSTRUCT);
            super::themed_controls::draw_item(item,item.CtlID as usize==ID_NEW,None)
        }
        WM_CLOSE => {
            if let Err(error) = try_close() {
                report(hwnd, &error);
            }
            0
        }
        WM_QUERYENDSESSION => flush(hwnd).is_ok() as isize,
        WM_SIZE if !ptr.is_null() => {
            if w == SIZE_MINIMIZED as usize {
                if let Err(error) = flush(hwnd) {
                    status(hwnd, &error, true);
                }
                schedule_hidden_memory_reclaim_after_activity();
            } else {
                layout(hwnd);
            }
            0
        }
        WM_GETMINMAXINFO if l != 0 => {
            let info = &mut *(l as *mut MINMAXINFO);
            let dpi = platform_dpi::layout_dpi_for_window(hwnd).max(96) as i32;
            info.ptMinTrackSize.x = 480 * dpi / 96;
            info.ptMinTrackSize.y = 320 * dpi / 96;
            0
        }
        WM_THEMECHANGED if !ptr.is_null() => {
            refresh_theme(hwnd);
            0
        }
        WM_DPICHANGED if l != 0 => {
            let rect = &*(l as *const RECT);
            platform_window::set_pos(
                hwnd,
                null_mut(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            refresh_theme(hwnd);
            0
        }
        WM_COMMAND if !ptr.is_null() => {
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if (*ptr).loading {
                return 0;
            }
            let result = match id {
                ID_NEW if notification==BN_CLICKED as usize => new_note(hwnd),
                ID_DELETE if notification==BN_CLICKED as usize => delete_note(hwnd),
                ID_EDITOR if notification == EN_CHANGE as usize => {
                    (*ptr).dirty = true;
                    status(hwnd, tr("正在自动保存…", "Saving…"), false);
                    timer::start(hwnd, SAVE_TIMER, 500);
                    Ok(())
                }
                ID_LIST if notification == LBN_SELCHANGE as usize => {
                    let index = platform_window::send_message((*ptr).list, LB_GETCURSEL, 0, 0);
                    if let Some(id) = (&(*ptr).rows).get(index as usize).map(|row| row.id) {
                        select_note(hwnd, id)
                    } else {
                        Ok(())
                    }
                }
                _ => Ok(()),
            };
            if let Err(error) = result {
                let _ = refresh_list(hwnd);
                report(hwnd, &error);
            }
            0
        }
        WM_TIMER if w == SAVE_TIMER && !ptr.is_null() => {
            timer::stop(hwnd, SAVE_TIMER);
            if let Err(error) = flush(hwnd) {
                status(
                    hwnd,
                    &format!("{} {error}", tr("未保存：", "Not saved:")),
                    true,
                );
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORSTATIC if !ptr.is_null() => {
            let dc = w as HDC;
            let th = Theme::default();
            let label = msg == WM_CTLCOLORSTATIC;
            platform_gdi::set_text_color(
                dc,
                if label && (*ptr).failed {
                    rgb(210, 55, 55)
                } else {
                    th.text
                },
            );
            platform_gdi::set_bk_color(dc, if label { th.bg } else if msg==WM_CTLCOLORLISTBOX {th.surface} else { th.control_bg });
            if label {
                (*ptr).background as isize
            } else if msg==WM_CTLCOLORLISTBOX {
                (*ptr).list_background as isize
            } else {
                (*ptr).control_background as isize
            }
        }
        WM_ERASEBKGND => 1,
        WM_PAINT if !ptr.is_null() => {
            let mut paint: PAINTSTRUCT = zeroed();
            let dc = platform_gdi::begin_paint(hwnd, &mut paint);
            if let Some(rect) = platform_window::client_rect(hwnd) {
                platform_gdi::fill_rect(dc, &rect, (*ptr).background);
            }
            platform_gdi::end_paint(hwnd, &paint);
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, w, l),
    }
}
