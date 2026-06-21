use crate::app_core::{
    MainHotkeyKey, MainHotkeyModifiers, MainPointerModifiers, ShortcutKey, ShortcutModifiers,
};
use crate::platform::input as platform_input;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::Input::KeyboardAndMouse::{
        VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT, VK_LEFT,
        VK_LWIN, VK_MENU, VK_NEXT, VK_NUMPAD1, VK_NUMPAD9, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_RWIN,
        VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
    },
};

pub(crate) const MOD_ALT: u32 = 0x0001;
pub(crate) const MOD_CONTROL: u32 = 0x0002;
pub(crate) const MOD_SHIFT: u32 = 0x0004;
pub(crate) const MOD_WIN: u32 = 0x0008;
const MOD_NOREPEAT: u32 = 0x4000;

pub(crate) const ERROR_HOTKEY_ALREADY_REGISTERED: u32 = 1409;

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterHotKey(hwnd: HWND, id: i32, fsmodifiers: u32, vk: u32) -> i32;
    fn UnregisterHotKey(hwnd: HWND, id: i32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
}

pub(crate) fn register(hwnd: isize, id: i32, modifiers: u32, vk: u32) -> Result<(), u32> {
    let ok = unsafe { RegisterHotKey(hwnd as HWND, id, modifiers | MOD_NOREPEAT, vk) };
    if ok != 0 {
        Ok(())
    } else {
        Err(unsafe { GetLastError() })
    }
}

pub(crate) fn unregister(hwnd: isize, id: i32) {
    unsafe {
        UnregisterHotKey(hwnd as HWND, id);
    }
}

pub(crate) fn mods_from_spec(modifiers: MainHotkeyModifiers) -> u32 {
    let mut mods = 0;
    if modifiers.ctrl {
        mods |= MOD_CONTROL;
    }
    if modifiers.alt {
        mods |= MOD_ALT;
    }
    if modifiers.shift {
        mods |= MOD_SHIFT;
    }
    if modifiers.meta {
        mods |= MOD_WIN;
    }
    mods
}

pub(crate) fn vk_from_spec(key: MainHotkeyKey) -> u32 {
    match key {
        MainHotkeyKey::Char(ch) => ch as u32,
        MainHotkeyKey::Space => VK_SPACE as u32,
        MainHotkeyKey::Enter => VK_RETURN as u32,
        MainHotkeyKey::Tab => VK_TAB as u32,
        MainHotkeyKey::Escape => VK_ESCAPE as u32,
        MainHotkeyKey::Backspace => VK_BACK as u32,
        MainHotkeyKey::Delete => VK_DELETE as u32,
        MainHotkeyKey::Insert => VK_INSERT as u32,
        MainHotkeyKey::Up => VK_UP as u32,
        MainHotkeyKey::Down => VK_DOWN as u32,
        MainHotkeyKey::Left => VK_LEFT as u32,
        MainHotkeyKey::Right => VK_RIGHT as u32,
        MainHotkeyKey::Home => VK_HOME as u32,
        MainHotkeyKey::End => VK_END as u32,
        MainHotkeyKey::PageUp => VK_PRIOR as u32,
        MainHotkeyKey::PageDown => VK_NEXT as u32,
    }
}

pub(crate) fn shortcut_key_from_vk(vk: u32) -> ShortcutKey {
    match vk {
        x if x == VK_UP as u32 => ShortcutKey::Up,
        x if x == VK_DOWN as u32 => ShortcutKey::Down,
        x if x == VK_RETURN as u32 => ShortcutKey::Enter,
        0x41 => ShortcutKey::A,
        0x43 => ShortcutKey::C,
        x if x == VK_DELETE as u32 => ShortcutKey::Delete,
        x if x == VK_ESCAPE as u32 => ShortcutKey::Escape,
        0x50 => ShortcutKey::P,
        0x46 => ShortcutKey::F,
        _ => ShortcutKey::Other(vk),
    }
}

pub(crate) fn key_label_from_vk(vk: u32) -> Option<&'static str> {
    match vk {
        0x41..=0x5A => Some(match vk {
            0x41 => "A",
            0x42 => "B",
            0x43 => "C",
            0x44 => "D",
            0x45 => "E",
            0x46 => "F",
            0x47 => "G",
            0x48 => "H",
            0x49 => "I",
            0x4A => "J",
            0x4B => "K",
            0x4C => "L",
            0x4D => "M",
            0x4E => "N",
            0x4F => "O",
            0x50 => "P",
            0x51 => "Q",
            0x52 => "R",
            0x53 => "S",
            0x54 => "T",
            0x55 => "U",
            0x56 => "V",
            0x57 => "W",
            0x58 => "X",
            0x59 => "Y",
            _ => "Z",
        }),
        0x30..=0x39 => Some(match vk {
            0x30 => "0",
            0x31 => "1",
            0x32 => "2",
            0x33 => "3",
            0x34 => "4",
            0x35 => "5",
            0x36 => "6",
            0x37 => "7",
            0x38 => "8",
            _ => "9",
        }),
        x if x == VK_SPACE as u32 => Some("Space"),
        x if x == VK_RETURN as u32 => Some("Enter"),
        x if x == VK_TAB as u32 => Some("Tab"),
        x if x == VK_ESCAPE as u32 => Some("Esc"),
        x if x == VK_BACK as u32 => Some("Backspace"),
        x if x == VK_DELETE as u32 => Some("Delete"),
        x if x == VK_INSERT as u32 => Some("Insert"),
        x if x == VK_UP as u32 => Some("Up"),
        x if x == VK_DOWN as u32 => Some("Down"),
        x if x == VK_LEFT as u32 => Some("Left"),
        x if x == VK_RIGHT as u32 => Some("Right"),
        x if x == VK_HOME as u32 => Some("Home"),
        x if x == VK_END as u32 => Some("End"),
        x if x == VK_PRIOR as u32 => Some("PageUp"),
        x if x == VK_NEXT as u32 => Some("PageDown"),
        _ => None,
    }
}

pub(crate) fn modifier_label_from_state(
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
) -> Option<String> {
    if win && !ctrl && !alt && !shift {
        return Some("Win".to_string());
    }
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("+"))
    }
}

pub(crate) fn modifier_label_from_pressed_state() -> Option<String> {
    modifier_label_from_state(
        platform_input::is_key_down(VK_CONTROL as u32),
        platform_input::is_key_down(VK_MENU as u32),
        platform_input::is_key_down(VK_SHIFT as u32),
        platform_input::is_key_down(VK_LWIN as u32) || platform_input::is_key_down(VK_RWIN as u32),
    )
}

pub(crate) fn shortcut_modifiers_from_state(
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
) -> ShortcutModifiers {
    ShortcutModifiers {
        ctrl,
        shift,
        alt,
        meta,
    }
}

pub(crate) fn shortcut_modifiers_from_pressed_state() -> ShortcutModifiers {
    shortcut_modifiers_from_state(
        platform_input::is_key_down(VK_CONTROL as u32),
        platform_input::is_key_down(VK_SHIFT as u32),
        platform_input::is_key_down(VK_MENU as u32),
        platform_input::is_key_down(VK_LWIN as u32) || platform_input::is_key_down(VK_RWIN as u32),
    )
}

pub(crate) fn pointer_modifiers_from_state(ctrl: bool, shift: bool) -> MainPointerModifiers {
    MainPointerModifiers { ctrl, shift }
}

pub(crate) fn pointer_modifiers_from_pressed_state() -> MainPointerModifiers {
    pointer_modifiers_from_state(
        platform_input::is_key_down(VK_CONTROL as u32),
        platform_input::is_key_down(VK_SHIFT as u32),
    )
}

pub(crate) fn ctrl_shift_from_pressed_state() -> (bool, bool) {
    (
        platform_input::is_key_down(VK_CONTROL as u32),
        platform_input::is_key_down(VK_SHIFT as u32),
    )
}

pub(crate) fn command_modifier_from_state(ctrl: bool, alt: bool, meta: bool) -> bool {
    ctrl || alt || meta
}

pub(crate) fn command_modifier_pressed() -> bool {
    command_modifier_from_state(
        platform_input::is_key_down(VK_CONTROL as u32),
        platform_input::is_key_down(VK_MENU as u32),
        platform_input::is_key_down(VK_LWIN as u32) || platform_input::is_key_down(VK_RWIN as u32),
    )
}

pub(crate) fn control_pressed() -> bool {
    platform_input::is_key_down(VK_CONTROL as u32)
}

pub(crate) fn shift_pressed() -> bool {
    platform_input::is_key_down(VK_SHIFT as u32)
}

pub(crate) fn digit_index_1_to_9_from_vk(vk: u32) -> Option<usize> {
    match vk {
        0x31..=0x39 => Some((vk - 0x31) as usize),
        x if x >= VK_NUMPAD1 as u32 && x <= VK_NUMPAD9 as u32 => {
            Some((x - VK_NUMPAD1 as u32) as usize)
        }
        _ => None,
    }
}

pub(crate) fn is_modifier_vk(vk: u32) -> bool {
    matches!(
        vk,
        x if x == VK_SHIFT as u32
            || x == VK_CONTROL as u32
            || x == VK_MENU as u32
            || x == VK_LWIN as u32
            || x == VK_RWIN as u32
    )
}

pub(crate) fn is_enter_vk(vk: u32) -> bool {
    vk == VK_RETURN as u32
}

pub(crate) fn is_escape_vk(vk: u32) -> bool {
    vk == VK_ESCAPE as u32
}

pub(crate) fn is_backspace_vk(vk: u32) -> bool {
    vk == VK_BACK as u32
}

pub(crate) fn is_find_vk(vk: u32) -> bool {
    vk == b'F' as u32
}

pub(crate) fn escape_wparam() -> usize {
    VK_ESCAPE as usize
}

pub(crate) fn find_wparam() -> usize {
    b'F' as usize
}

pub(crate) fn escape_key_u8() -> u8 {
    VK_ESCAPE as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_specs_map_to_win32_modifiers_and_vk_codes() {
        let modifiers = MainHotkeyModifiers {
            ctrl: true,
            alt: true,
            shift: false,
            meta: true,
        };
        assert_eq!(mods_from_spec(modifiers), MOD_CONTROL | MOD_ALT | MOD_WIN);
        assert_eq!(vk_from_spec(MainHotkeyKey::Char('V')), 'V' as u32);
        assert_eq!(vk_from_spec(MainHotkeyKey::Space), VK_SPACE as u32);
        assert_eq!(vk_from_spec(MainHotkeyKey::PageDown), VK_NEXT as u32);
    }

    #[test]
    fn main_window_virtual_keys_map_to_shortcut_keys() {
        assert_eq!(shortcut_key_from_vk(VK_UP as u32), ShortcutKey::Up);
        assert_eq!(shortcut_key_from_vk(VK_DOWN as u32), ShortcutKey::Down);
        assert_eq!(shortcut_key_from_vk(VK_RETURN as u32), ShortcutKey::Enter);
        assert_eq!(shortcut_key_from_vk(0x41), ShortcutKey::A);
        assert_eq!(shortcut_key_from_vk(0x43), ShortcutKey::C);
        assert_eq!(shortcut_key_from_vk(VK_DELETE as u32), ShortcutKey::Delete);
        assert_eq!(shortcut_key_from_vk(VK_ESCAPE as u32), ShortcutKey::Escape);
        assert_eq!(shortcut_key_from_vk(0x50), ShortcutKey::P);
        assert_eq!(shortcut_key_from_vk(0x46), ShortcutKey::F);
        assert_eq!(shortcut_key_from_vk(0), ShortcutKey::Other(0));
    }

    #[test]
    fn settings_hotkey_virtual_keys_map_to_option_labels() {
        assert_eq!(key_label_from_vk(0x41), Some("A"));
        assert_eq!(key_label_from_vk(0x39), Some("9"));
        assert_eq!(key_label_from_vk(VK_TAB as u32), Some("Tab"));
        assert_eq!(key_label_from_vk(VK_NEXT as u32), Some("PageDown"));
        assert_eq!(key_label_from_vk(0), None);
    }

    #[test]
    fn settings_hotkey_modifier_state_maps_to_option_labels() {
        assert_eq!(
            modifier_label_from_state(true, false, false, false),
            Some("Ctrl".to_string())
        );
        assert_eq!(
            modifier_label_from_state(true, true, true, false),
            Some("Ctrl+Alt+Shift".to_string())
        );
        assert_eq!(
            modifier_label_from_state(false, false, false, true),
            Some("Win".to_string())
        );
        assert_eq!(modifier_label_from_state(false, false, false, false), None);
        assert_eq!(
            modifier_label_from_state(true, false, false, true),
            Some("Ctrl".to_string())
        );
    }

    #[test]
    fn virtual_key_helpers_identify_vv_digits_and_modifiers() {
        assert_eq!(digit_index_1_to_9_from_vk(0x31), Some(0));
        assert_eq!(digit_index_1_to_9_from_vk(0x39), Some(8));
        assert_eq!(digit_index_1_to_9_from_vk(VK_NUMPAD1 as u32), Some(0));
        assert_eq!(digit_index_1_to_9_from_vk(VK_NUMPAD9 as u32), Some(8));
        assert_eq!(digit_index_1_to_9_from_vk(0x30), None);

        assert!(is_modifier_vk(VK_SHIFT as u32));
        assert!(is_modifier_vk(VK_CONTROL as u32));
        assert!(is_modifier_vk(VK_LWIN as u32));
        assert!(!is_modifier_vk(0x41));

        assert!(is_enter_vk(VK_RETURN as u32));
        assert!(is_escape_vk(VK_ESCAPE as u32));
        assert!(is_backspace_vk(VK_BACK as u32));
        assert!(is_find_vk(b'F' as u32));
        assert_eq!(escape_wparam(), VK_ESCAPE as usize);
        assert_eq!(find_wparam(), b'F' as usize);
        assert_eq!(escape_key_u8(), VK_ESCAPE as u8);
    }

    #[test]
    fn pressed_modifier_state_maps_to_core_modifier_types() {
        assert_eq!(
            shortcut_modifiers_from_state(true, false, true, false),
            ShortcutModifiers {
                ctrl: true,
                shift: false,
                alt: true,
                meta: false,
            }
        );
        assert_eq!(
            pointer_modifiers_from_state(false, true),
            MainPointerModifiers {
                ctrl: false,
                shift: true,
            }
        );
        assert!(command_modifier_from_state(true, false, false));
        assert!(command_modifier_from_state(false, true, false));
        assert!(command_modifier_from_state(false, false, true));
        assert!(!command_modifier_from_state(false, false, false));
    }
}
