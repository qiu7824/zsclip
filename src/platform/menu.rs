use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{ExcludeClipRect, HBRUSH, HDC, HFONT, HGDIOBJ},
    UI::Controls::{
        DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_DISABLED, ODS_GRAYED, ODS_SELECTED, ODT_MENU,
    },
    UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
    UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetMenuItemCount, InsertMenuItemW,
        SetMenuInfo, TrackPopupMenu, HMENU, MENUINFO, MENUITEMINFOW, MFS_CHECKED, MFS_DISABLED,
        MFT_OWNERDRAW, MFT_SEPARATOR, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING,
        MIIM_DATA, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU,
        MIM_APPLYTOSUBMENUS, MIM_BACKGROUND, MIM_STYLE, MNS_NOCHECK, TPM_BOTTOMALIGN,
        TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TPM_TOPALIGN, WM_DRAWITEM,
        WM_MEASUREITEM,
    },
};

use crate::app_core::{NativePopupMenuEntry, NativePopupMenuHost, NativePopupMenuPlacement};

use super::{
    appearance, dpi, gdi as platform_gdi, gdiplus, string::to_wide, system_parameters, window,
};

const DT_LEFT: u32 = 0x0000;
const DT_CENTER: u32 = 0x0001;
const DT_RIGHT: u32 = 0x0002;
const DT_VCENTER: u32 = 0x0004;
const DT_SINGLELINE: u32 = 0x0020;
const DT_CALCRECT: u32 = 0x0400;
const DT_NOPREFIX: u32 = 0x0800;
const DT_END_ELLIPSIS: u32 = 0x0000_8000;
const TRANSPARENT_BK: i32 = 1;
const ANTIALIASED_QUALITY: u32 = 4;
const CLEARTYPE_QUALITY: u32 = 5;
const ICON_FONT: &str = "Segoe MDL2 Assets";
const GLYPH_CHECK: &str = "\u{E73E}";
const GLYPH_CHEVRON_RIGHT: &str = "\u{E76C}";

pub(crate) fn create_popup() -> HMENU {
    unsafe { CreatePopupMenu() }
}

pub(crate) fn append_raw(menu: HMENU, flags: u32, id_or_submenu: usize, text: *const u16) -> bool {
    unsafe { AppendMenuW(menu, flags, id_or_submenu, text) != 0 }
}

pub(crate) fn track_popup_raw(
    menu: HMENU,
    flags: u32,
    x: i32,
    y: i32,
    reserved: i32,
    owner: HWND,
    rect: *const RECT,
) -> usize {
    unsafe { TrackPopupMenu(menu, flags, x, y, reserved, owner, rect) as usize }
}

pub(crate) fn destroy(menu: HMENU) {
    if menu.is_null() {
        return;
    }
    unsafe {
        DestroyMenu(menu);
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

/// Mirrors the light/dark tones of `win_native_style::Theme` and the settings
/// dropdown popup so context menus read as part of the same app.
#[derive(Clone, Copy)]
struct MenuPalette {
    background: u32,
    hover: u32,
    text: u32,
    text_muted: u32,
    text_disabled: u32,
    separator: u32,
}

impl MenuPalette {
    fn current() -> Self {
        if appearance::is_dark_mode() {
            Self {
                background: rgb(44, 44, 44),
                hover: rgb(60, 60, 60),
                text: rgb(255, 255, 255),
                text_muted: rgb(162, 162, 162),
                text_disabled: rgb(120, 120, 120),
                separator: rgb(64, 64, 64),
            }
        } else {
            Self {
                background: rgb(255, 255, 255),
                hover: rgb(237, 237, 237),
                text: rgb(28, 28, 28),
                text_muted: rgb(96, 96, 96),
                text_disabled: rgb(160, 160, 160),
                separator: rgb(229, 229, 229),
            }
        }
    }
}

#[derive(Clone, Copy)]
struct MenuMetrics {
    dpi: i32,
}

impl MenuMetrics {
    fn px(self, value: i32) -> i32 {
        (value * self.dpi + 48) / 96
    }
    fn item_height(self) -> i32 {
        self.px(30)
    }
    fn separator_height(self) -> i32 {
        self.px(9)
    }
    fn highlight_inset_x(self) -> i32 {
        self.px(4)
    }
    fn highlight_inset_y(self) -> i32 {
        self.px(2)
    }
    fn text_left(self, check_column: bool) -> i32 {
        if check_column {
            self.px(36)
        } else {
            self.px(16)
        }
    }
    fn right_area(self, submenu: bool) -> i32 {
        if submenu {
            self.px(32)
        } else {
            self.px(20)
        }
    }
    fn min_width(self) -> i32 {
        self.px(148)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuItemKind {
    Command,
    Submenu,
    Separator,
}

struct MenuItemData {
    label: Vec<u16>,
    shortcut: Vec<u16>,
    kind: MenuItemKind,
    checked: bool,
    check_column: bool,
}

fn split_label(label: &str) -> (Vec<u16>, Vec<u16>) {
    match label.split_once('\t') {
        Some((text, shortcut)) => (
            text.encode_utf16().collect(),
            shortcut.trim().encode_utf16().collect(),
        ),
        None => (label.encode_utf16().collect(), Vec::new()),
    }
}

/// A themed popup menu: items are owner-drawn (taller rows, rounded hover,
/// app palette, dark mode) while keeping their text for accessibility.
struct OwnerDrawPopupMenu {
    root: HMENU,
    items: Vec<Box<MenuItemData>>,
    background: HBRUSH,
    text_font: HFONT,
    icon_font: HFONT,
    check_font: HFONT,
    palette: MenuPalette,
    metrics: MenuMetrics,
}

impl OwnerDrawPopupMenu {
    fn new(dpi: u32) -> Option<Self> {
        let root = create_popup();
        if root.is_null() {
            return None;
        }
        let palette = MenuPalette::current();
        let metrics = MenuMetrics {
            dpi: dpi.max(96) as i32,
        };
        let text_font = create_menu_font(
            system_parameters::system_ui_text_font_family(),
            metrics.px(12),
            CLEARTYPE_QUALITY,
        );
        // Grayscale anti-aliasing keeps thin icon glyphs free of colour fringes.
        let icon_font = create_menu_font(ICON_FONT, metrics.px(10), ANTIALIASED_QUALITY);
        let check_font = create_menu_font(ICON_FONT, metrics.px(12), ANTIALIASED_QUALITY);
        Some(Self {
            root,
            items: Vec::new(),
            background: platform_gdi::create_solid_brush(palette.background),
            text_font,
            icon_font,
            check_font,
            palette,
            metrics,
        })
    }

    unsafe fn append_entries(&mut self, menu: HMENU, entries: &[NativePopupMenuEntry]) {
        let check_column = entries.iter().any(|entry| {
            matches!(
                entry,
                NativePopupMenuEntry::Command { checked: true, .. }
            )
        });
        for entry in entries {
            let mut info: MENUITEMINFOW = core::mem::zeroed();
            info.cbSize = core::mem::size_of::<MENUITEMINFOW>() as u32;
            info.fMask = MIIM_FTYPE | MIIM_DATA;
            info.fType = MFT_OWNERDRAW;
            let mut text_buf = Vec::new();
            let data = match entry {
                NativePopupMenuEntry::Command {
                    id,
                    label,
                    enabled,
                    checked,
                } => {
                    info.fMask |= MIIM_ID | MIIM_STATE | MIIM_STRING;
                    info.wID = *id as u32;
                    if !enabled {
                        info.fState |= MFS_DISABLED;
                    }
                    if *checked {
                        info.fState |= MFS_CHECKED;
                    }
                    text_buf = to_wide(label);
                    let (label, shortcut) = split_label(label);
                    MenuItemData {
                        label,
                        shortcut,
                        kind: MenuItemKind::Command,
                        checked: *checked,
                        check_column,
                    }
                }
                NativePopupMenuEntry::Submenu {
                    label,
                    enabled,
                    entries,
                } => {
                    let submenu = create_popup();
                    if submenu.is_null() {
                        continue;
                    }
                    appearance::apply_theme_to_menu(submenu as _);
                    self.append_entries(submenu, entries);
                    info.fMask |= MIIM_SUBMENU | MIIM_STATE | MIIM_STRING;
                    info.hSubMenu = submenu;
                    if !enabled {
                        info.fState |= MFS_DISABLED;
                    }
                    text_buf = to_wide(label);
                    let (label, shortcut) = split_label(label);
                    MenuItemData {
                        label,
                        shortcut,
                        kind: MenuItemKind::Submenu,
                        checked: false,
                        check_column,
                    }
                }
                NativePopupMenuEntry::Separator => {
                    info.fType |= MFT_SEPARATOR;
                    MenuItemData {
                        label: Vec::new(),
                        shortcut: Vec::new(),
                        kind: MenuItemKind::Separator,
                        checked: false,
                        check_column,
                    }
                }
            };
            let data = Box::new(data);
            info.dwItemData = &*data as *const MenuItemData as usize;
            if !text_buf.is_empty() {
                info.dwTypeData = text_buf.as_mut_ptr();
                info.cch = (text_buf.len() - 1) as u32;
            }
            let position = GetMenuItemCount(menu).max(0) as u32;
            if InsertMenuItemW(menu, position, 1, &info) != 0 {
                self.items.push(data);
            } else if !info.hSubMenu.is_null() {
                destroy(info.hSubMenu);
            }
        }
    }

    unsafe fn apply_menu_info(&self) {
        let mut info: MENUINFO = core::mem::zeroed();
        info.cbSize = core::mem::size_of::<MENUINFO>() as u32;
        info.fMask = MIM_BACKGROUND | MIM_STYLE | MIM_APPLYTOSUBMENUS;
        info.dwStyle = MNS_NOCHECK;
        info.hbrBack = self.background;
        SetMenuInfo(self.root, &info);
    }

    fn owns(&self, item_data: usize) -> Option<&MenuItemData> {
        self.items
            .iter()
            .find(|item| &***item as *const MenuItemData as usize == item_data)
            .map(|item| &**item)
    }

    unsafe fn text_width(&self, text: &[u16], font: HFONT) -> i32 {
        if text.is_empty() {
            return 0;
        }
        let dc = platform_gdi::get_dc(core::ptr::null_mut());
        if dc.is_null() {
            return self.metrics.px(8) * text.len() as i32;
        }
        let old = platform_gdi::select_object(dc, font as _);
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        platform_gdi::draw_text(
            dc,
            text.as_ptr(),
            text.len() as i32,
            &mut rect,
            DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
        );
        platform_gdi::select_object(dc, old);
        platform_gdi::release_dc(core::ptr::null_mut(), dc);
        rect.right - rect.left
    }

    unsafe fn measure(&self, item: &MenuItemData, mis: &mut MEASUREITEMSTRUCT) {
        let m = self.metrics;
        if item.kind == MenuItemKind::Separator {
            mis.itemHeight = m.separator_height() as u32;
            mis.itemWidth = m.min_width() as u32;
            return;
        }
        let mut width = m.text_left(item.check_column)
            + self.text_width(&item.label, self.text_font)
            + m.right_area(item.kind == MenuItemKind::Submenu);
        if !item.shortcut.is_empty() {
            width += m.px(24) + self.text_width(&item.shortcut, self.text_font);
        }
        mis.itemWidth = width.max(m.min_width()) as u32;
        mis.itemHeight = m.item_height() as u32;
    }

    unsafe fn draw_text_run(
        &self,
        dc: HDC,
        text: &[u16],
        rect: &RECT,
        font: HFONT,
        color: u32,
        align: u32,
    ) {
        if text.is_empty() {
            return;
        }
        let old = platform_gdi::select_object(dc, font as _);
        platform_gdi::set_bk_mode(dc, TRANSPARENT_BK);
        platform_gdi::set_text_color(dc, color);
        let mut rect = *rect;
        platform_gdi::draw_text(
            dc,
            text.as_ptr(),
            text.len() as i32,
            &mut rect,
            align | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        platform_gdi::select_object(dc, old);
    }

    unsafe fn draw(&self, item: &MenuItemData, dis: &DRAWITEMSTRUCT) {
        let dc = dis.hDC;
        let rc = dis.rcItem;
        let m = self.metrics;
        let p = self.palette;
        let background = platform_gdi::create_solid_brush(p.background);
        platform_gdi::fill_rect(dc, &rc, background);
        platform_gdi::delete_object(background as _);

        if item.kind == MenuItemKind::Separator {
            let y = (rc.top + rc.bottom) / 2;
            let line = RECT {
                left: rc.left + m.px(12),
                top: y,
                right: rc.right - m.px(12),
                bottom: y + 1,
            };
            let brush = platform_gdi::create_solid_brush(p.separator);
            platform_gdi::fill_rect(dc, &line, brush);
            platform_gdi::delete_object(brush as _);
            ExcludeClipRect(dc, rc.left, rc.top, rc.right, rc.bottom);
            return;
        }

        let disabled = dis.itemState & (ODS_DISABLED | ODS_GRAYED) != 0;
        let selected = dis.itemState & ODS_SELECTED != 0;
        if selected && !disabled {
            let highlight = RECT {
                left: rc.left + m.highlight_inset_x(),
                top: rc.top + m.highlight_inset_y(),
                right: rc.right - m.highlight_inset_x(),
                bottom: rc.bottom - m.highlight_inset_y(),
            };
            if !gdiplus::draw_round_rect(
                dc as _,
                highlight.left,
                highlight.top,
                highlight.right,
                highlight.bottom,
                p.hover,
                p.hover,
                m.px(4),
            ) {
                let brush = platform_gdi::create_solid_brush(p.hover);
                platform_gdi::fill_rect(dc, &highlight, brush);
                platform_gdi::delete_object(brush as _);
            }
        }

        let text_color = if disabled { p.text_disabled } else { p.text };
        let secondary_color = if disabled {
            p.text_disabled
        } else {
            p.text_muted
        };
        if item.checked {
            let check = RECT {
                left: rc.left + m.px(8),
                top: rc.top,
                right: rc.left + m.text_left(true) - m.px(4),
                bottom: rc.bottom,
            };
            let glyph = to_wide(GLYPH_CHECK);
            self.draw_text_run(
                dc,
                &glyph[..glyph.len() - 1],
                &check,
                self.check_font,
                text_color,
                DT_CENTER,
            );
        }

        let right_area = m.right_area(item.kind == MenuItemKind::Submenu);
        let mut text_rect = RECT {
            left: rc.left + m.text_left(item.check_column),
            top: rc.top,
            right: rc.right - right_area,
            bottom: rc.bottom,
        };
        if !item.shortcut.is_empty() {
            let shortcut_w = self.text_width(&item.shortcut, self.text_font);
            let shortcut_rect = RECT {
                left: text_rect.right - shortcut_w,
                top: rc.top,
                right: text_rect.right,
                bottom: rc.bottom,
            };
            self.draw_text_run(
                dc,
                &item.shortcut,
                &shortcut_rect,
                self.text_font,
                secondary_color,
                DT_RIGHT,
            );
            text_rect.right = shortcut_rect.left - m.px(16);
        }
        self.draw_text_run(
            dc,
            &item.label,
            &text_rect,
            self.text_font,
            text_color,
            DT_LEFT,
        );

        if item.kind == MenuItemKind::Submenu {
            let arrow = RECT {
                left: rc.right - right_area,
                top: rc.top,
                right: rc.right - m.px(8),
                bottom: rc.bottom,
            };
            let glyph = to_wide(GLYPH_CHEVRON_RIGHT);
            self.draw_text_run(
                dc,
                &glyph[..glyph.len() - 1],
                &arrow,
                self.icon_font,
                secondary_color,
                DT_CENTER,
            );
        }
        // Stop the menu manager from painting its own submenu arrow on top.
        ExcludeClipRect(dc, rc.left, rc.top, rc.right, rc.bottom);
    }

    /// Tracks the menu with `owner` temporarily subclassed so it can answer
    /// WM_MEASUREITEM / WM_DRAWITEM. Returns `None` if subclassing failed.
    unsafe fn track(&self, owner: HWND, flags: u32, x: i32, y: i32) -> Option<usize> {
        let subclass_id = self as *const Self as usize;
        if SetWindowSubclass(owner, Some(owner_draw_menu_subclass), subclass_id, subclass_id) == 0
        {
            return None;
        }
        let cmd = track_popup_raw(self.root, flags, x, y, 0, owner, core::ptr::null());
        RemoveWindowSubclass(owner, Some(owner_draw_menu_subclass), subclass_id);
        Some(cmd)
    }
}

impl Drop for OwnerDrawPopupMenu {
    fn drop(&mut self) {
        destroy(self.root);
        for object in [
            self.background as HGDIOBJ,
            self.text_font as HGDIOBJ,
            self.icon_font as HGDIOBJ,
            self.check_font as HGDIOBJ,
        ] {
            if !object.is_null() {
                platform_gdi::delete_object(object);
            }
        }
    }
}

fn create_menu_font(family: &str, pixel_size: i32, quality: u32) -> HFONT {
    let face = to_wide(family);
    platform_gdi::create_font_w(
        -pixel_size.max(1),
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
        quality,
        0,
        face.as_ptr(),
    )
}

unsafe extern "system" fn owner_draw_menu_subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    let menu = &*(ref_data as *const OwnerDrawPopupMenu);
    match msg {
        WM_MEASUREITEM if lparam != 0 => {
            let mis = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            if mis.CtlType == ODT_MENU {
                if let Some(item) = menu.owns(mis.itemData) {
                    menu.measure(item, mis);
                    return 1;
                }
            }
        }
        WM_DRAWITEM if lparam != 0 => {
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            if dis.CtlType == ODT_MENU {
                if let Some(item) = menu.owns(dis.itemData) {
                    menu.draw(item, dis);
                    return 1;
                }
            }
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Shows `entries` as a themed popup menu. With `TPM_RETURNCMD` in `flags`
/// the chosen command id is returned; otherwise it is posted to `owner`.
pub(crate) fn present_themed_popup_menu(
    owner: HWND,
    x: i32,
    y: i32,
    flags: u32,
    entries: &[NativePopupMenuEntry],
) -> usize {
    unsafe {
        let menu_dpi = dpi::layout_dpi_for_point(POINT { x, y });
        if let Some(mut menu) = OwnerDrawPopupMenu::new(menu_dpi) {
            appearance::apply_theme_to_menu(menu.root as _);
            let root = menu.root;
            menu.append_entries(root, entries);
            menu.apply_menu_info();
            window::set_foreground(owner);
            if let Some(cmd) = menu.track(owner, flags, x, y) {
                window::ping(owner);
                return cmd;
            }
        }
        present_native_popup_menu(owner, x, y, flags, entries)
    }
}

unsafe fn present_native_popup_menu(
    owner: HWND,
    x: i32,
    y: i32,
    flags: u32,
    entries: &[NativePopupMenuEntry],
) -> usize {
    let menu = create_popup();
    if menu.is_null() {
        return 0;
    }
    appearance::apply_theme_to_menu(menu as _);
    WindowsPopupMenuHost::append_entries(menu, entries);
    window::set_foreground(owner);
    let cmd = track_popup_raw(menu, flags, x, y, 0, owner, core::ptr::null::<RECT>());
    window::ping(owner);
    destroy(menu);
    cmd
}

pub(crate) struct WindowsPopupMenuHost;

impl WindowsPopupMenuHost {
    pub(crate) const fn new() -> Self {
        Self
    }

    fn append_entries(menu: HMENU, entries: &[NativePopupMenuEntry]) {
        for entry in entries {
            match entry {
                NativePopupMenuEntry::Command {
                    id,
                    label,
                    enabled,
                    checked,
                } => {
                    let mut flags = MF_STRING;
                    if !enabled {
                        flags |= MF_GRAYED;
                    }
                    if *checked {
                        flags |= MF_CHECKED;
                    }
                    append_raw(menu, flags, *id, to_wide(label).as_ptr());
                }
                NativePopupMenuEntry::Submenu {
                    label,
                    enabled,
                    entries,
                } => {
                    let submenu = create_popup();
                    if submenu.is_null() {
                        continue;
                    }
                    unsafe {
                        appearance::apply_theme_to_menu(submenu as _);
                    }
                    Self::append_entries(submenu, entries);
                    let mut flags = MF_POPUP;
                    if !enabled {
                        flags |= MF_GRAYED;
                    }
                    append_raw(menu, flags, submenu as usize, to_wide(label).as_ptr());
                }
                NativePopupMenuEntry::Separator => {
                    append_raw(menu, MF_SEPARATOR, 0, core::ptr::null());
                }
            }
        }
    }
}

impl NativePopupMenuHost for WindowsPopupMenuHost {
    type Owner = HWND;

    fn present_popup_menu(
        &mut self,
        owner: Self::Owner,
        x: i32,
        y: i32,
        placement: NativePopupMenuPlacement,
        entries: &[NativePopupMenuEntry],
    ) -> usize {
        let align = match placement {
            NativePopupMenuPlacement::TopLeft => TPM_TOPALIGN,
            NativePopupMenuPlacement::BottomLeft => TPM_BOTTOMALIGN,
        };
        present_themed_popup_menu(
            owner,
            x,
            y,
            TPM_RIGHTBUTTON | align | TPM_LEFTALIGN | TPM_RETURNCMD,
            entries,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_labels_split_accelerator_text_after_tab() {
        let (label, shortcut) = split_label("复制\tCtrl+C");
        assert_eq!(String::from_utf16_lossy(&label), "复制");
        assert_eq!(String::from_utf16_lossy(&shortcut), "Ctrl+C");
        let (label, shortcut) = split_label("Tom & Jerry");
        assert_eq!(String::from_utf16_lossy(&label), "Tom & Jerry");
        assert!(shortcut.is_empty());
    }

    #[test]
    fn menu_metrics_scale_with_dpi() {
        let normal = MenuMetrics { dpi: 96 };
        let large = MenuMetrics { dpi: 144 };
        assert_eq!(normal.item_height(), 30);
        assert_eq!(large.item_height(), 45);
        assert!(normal.text_left(true) > normal.text_left(false));
        assert!(normal.right_area(true) > normal.right_area(false));
    }
}
