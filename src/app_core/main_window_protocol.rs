use super::{clamp_window_pos_to_rect, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainWindowVisibilityInput {
    pub(crate) main_visible: bool,
    pub(crate) quick_visible: bool,
    pub(crate) main_edge_hidden: bool,
    pub(crate) quick_edge_hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainWindowVisibilityStep {
    TryRestoreQuickEdge,
    HideQuick,
    TryRestoreMainEdge,
    HideMain,
    ShowMain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MainWindowVisibilityPlan {
    pub(crate) steps: Vec<MainWindowVisibilityStep>,
}

pub(crate) fn main_window_toggle_visibility_plan(
    input: MainWindowVisibilityInput,
) -> MainWindowVisibilityPlan {
    let mut steps = Vec::new();
    if input.quick_visible {
        if input.quick_edge_hidden {
            steps.push(MainWindowVisibilityStep::TryRestoreQuickEdge);
        }
        steps.push(MainWindowVisibilityStep::HideQuick);
    }
    if input.main_edge_hidden {
        steps.push(MainWindowVisibilityStep::TryRestoreMainEdge);
    }
    steps.push(if input.main_visible {
        MainWindowVisibilityStep::HideMain
    } else {
        MainWindowVisibilityStep::ShowMain
    });
    MainWindowVisibilityPlan { steps }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainWindowHotkeyVisibilityInput {
    pub(crate) main_visible: bool,
    pub(crate) quick_visible: bool,
    pub(crate) main_edge_hidden: bool,
    pub(crate) quick_edge_hidden: bool,
    pub(crate) plain_text_paste_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainWindowHotkeyVisibilityStep {
    TryRestoreQuickEdge,
    HideQuick,
    TryRestoreMainEdge,
    HideMain,
    ShowQuick { plain_text_paste_mode: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MainWindowHotkeyVisibilityPlan {
    pub(crate) steps: Vec<MainWindowHotkeyVisibilityStep>,
}

pub(crate) fn main_window_hotkey_visibility_plan(
    input: MainWindowHotkeyVisibilityInput,
) -> MainWindowHotkeyVisibilityPlan {
    let mut steps = Vec::new();
    if input.quick_visible {
        if input.quick_edge_hidden {
            steps.push(MainWindowHotkeyVisibilityStep::TryRestoreQuickEdge);
        }
        steps.push(MainWindowHotkeyVisibilityStep::HideQuick);
    } else if input.main_visible {
        if input.main_edge_hidden {
            steps.push(MainWindowHotkeyVisibilityStep::TryRestoreMainEdge);
        }
        steps.push(MainWindowHotkeyVisibilityStep::HideMain);
    } else {
        steps.push(MainWindowHotkeyVisibilityStep::ShowQuick {
            plain_text_paste_mode: input.plain_text_paste_mode,
        });
    }
    MainWindowHotkeyVisibilityPlan { steps }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainWindowPositionMode {
    Mouse,
    Fixed,
    Last,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainWindowPositionInput {
    pub(crate) mode: MainWindowPositionMode,
    pub(crate) by_hotkey: bool,
    pub(crate) cursor_x: i32,
    pub(crate) cursor_y: i32,
    pub(crate) mouse_dx: i32,
    pub(crate) mouse_dy: i32,
    pub(crate) fixed_x: i32,
    pub(crate) fixed_y: i32,
    pub(crate) last_x: i32,
    pub(crate) last_y: i32,
    pub(crate) bounds: UiRect,
    pub(crate) win_w: i32,
    pub(crate) win_h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainWindowPositionAnchor {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainWindowPositionPlan {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

pub(crate) fn main_window_position_anchor(
    input: MainWindowPositionInput,
) -> MainWindowPositionAnchor {
    let mouse = || {
        (
            input.cursor_x + input.mouse_dx,
            input.cursor_y + input.mouse_dy,
        )
    };
    let (x, y) = match input.mode {
        MainWindowPositionMode::Fixed => (input.fixed_x, input.fixed_y),
        MainWindowPositionMode::Last if input.last_x >= 0 && input.last_y >= 0 => {
            (input.last_x, input.last_y)
        }
        MainWindowPositionMode::Mouse => mouse(),
        MainWindowPositionMode::Last if input.by_hotkey => mouse(),
        MainWindowPositionMode::Center if input.by_hotkey => mouse(),
        _ => (
            input.bounds.left + ((input.bounds.width() - input.win_w) / 2),
            input.bounds.top + ((input.bounds.height() - input.win_h) / 3),
        ),
    };
    MainWindowPositionAnchor { x, y }
}

pub(crate) fn main_window_position_plan(input: MainWindowPositionInput) -> MainWindowPositionPlan {
    let anchor = main_window_position_anchor(input);
    let (x, y) =
        clamp_window_pos_to_rect(anchor.x, anchor.y, input.bounds, input.win_w, input.win_h);
    MainWindowPositionPlan {
        x,
        y,
        width: input.win_w,
        height: input.win_h,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainEdgeRestorePositionInput {
    pub(crate) edge_auto_hide: bool,
    pub(crate) edge_hidden_side: i32,
    pub(crate) edge_docked_left: i32,
    pub(crate) edge_docked_top: i32,
    pub(crate) edge_docked_right: i32,
    pub(crate) edge_docked_bottom: i32,
    pub(crate) edge_restore_x: i32,
    pub(crate) edge_restore_y: i32,
    pub(crate) last_window_x: i32,
    pub(crate) last_window_y: i32,
}

pub(crate) fn main_edge_restore_position(
    input: MainEdgeRestorePositionInput,
) -> Option<MainWindowPositionAnchor> {
    if !input.edge_auto_hide {
        return None;
    }
    if input.edge_hidden_side >= 0
        && input.edge_docked_right > input.edge_docked_left
        && input.edge_docked_bottom > input.edge_docked_top
    {
        return Some(MainWindowPositionAnchor {
            x: input.edge_restore_x,
            y: input.edge_restore_y,
        });
    }
    (input.last_window_x >= 0 && input.last_window_y >= 0).then_some(MainWindowPositionAnchor {
        x: input.last_window_x,
        y: input.last_window_y,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainRememberWindowPositionInput {
    pub(crate) edge_auto_hide: bool,
    pub(crate) edge_hidden: bool,
    pub(crate) edge_restore_x: i32,
    pub(crate) edge_restore_y: i32,
    pub(crate) window_left: i32,
    pub(crate) window_top: i32,
}

pub(crate) fn main_remember_window_position(
    input: MainRememberWindowPositionInput,
) -> MainWindowPositionAnchor {
    if input.edge_auto_hide && input.edge_hidden {
        MainWindowPositionAnchor {
            x: input.edge_restore_x,
            y: input.edge_restore_y,
        }
    } else {
        MainWindowPositionAnchor {
            x: input.window_left,
            y: input.window_top,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainShowWindowKind {
    Main,
    Quick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainShowWindowDockAction {
    NoteMovedForEdgeHide,
    ClearEdgeDockState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainHotkeyPassthroughPlan {
    Clear,
    UseForegroundSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainShowWindowStateInput {
    pub(crate) kind: MainShowWindowKind,
    pub(crate) by_hotkey: bool,
    pub(crate) edge_auto_hide: bool,
    pub(crate) foreground_snapshot_available: bool,
    pub(crate) plain_text_paste_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainShowWindowStatePlan {
    pub(crate) reset_edge_hidden_state: bool,
    pub(crate) dock_action: MainShowWindowDockAction,
    pub(crate) hotkey_passthrough: MainHotkeyPassthroughPlan,
    pub(crate) plain_text_paste_mode: bool,
}

pub(crate) fn main_show_window_state_plan(
    input: MainShowWindowStateInput,
) -> MainShowWindowStatePlan {
    let hotkey_passthrough = match input.kind {
        MainShowWindowKind::Main => MainHotkeyPassthroughPlan::Clear,
        MainShowWindowKind::Quick if input.by_hotkey && input.foreground_snapshot_available => {
            MainHotkeyPassthroughPlan::UseForegroundSnapshot
        }
        MainShowWindowKind::Quick => MainHotkeyPassthroughPlan::Clear,
    };
    MainShowWindowStatePlan {
        reset_edge_hidden_state: true,
        dock_action: if input.edge_auto_hide {
            MainShowWindowDockAction::NoteMovedForEdgeHide
        } else {
            MainShowWindowDockAction::ClearEdgeDockState
        },
        hotkey_passthrough,
        plain_text_paste_mode: match input.kind {
            MainShowWindowKind::Main => false,
            MainShowWindowKind::Quick => input.plain_text_paste_mode,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainHotkeyModifiers {
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
    pub(crate) meta: bool,
}

impl MainHotkeyModifiers {
    pub(crate) const fn meta() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainHotkeyKey {
    Char(char),
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Insert,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainHotkeySpec {
    pub(crate) modifiers: MainHotkeyModifiers,
    pub(crate) key: MainHotkeyKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainHotkeyRegistrationInput<'a> {
    pub(crate) enabled: bool,
    pub(crate) already_registered: bool,
    pub(crate) mod_label: &'a str,
    pub(crate) key_label: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainHotkeyRegistrationPlan {
    pub(crate) unregister_existing: bool,
    pub(crate) register: Option<MainHotkeySpec>,
}

fn main_hotkey_modifiers_from_label(label: &str) -> MainHotkeyModifiers {
    match label.trim() {
        "Ctrl" => MainHotkeyModifiers {
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
        "Alt" => MainHotkeyModifiers {
            ctrl: false,
            alt: true,
            shift: false,
            meta: false,
        },
        "Shift" => MainHotkeyModifiers {
            ctrl: false,
            alt: false,
            shift: true,
            meta: false,
        },
        "Ctrl+Alt" => MainHotkeyModifiers {
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
        },
        "Ctrl+Shift" => MainHotkeyModifiers {
            ctrl: true,
            alt: false,
            shift: true,
            meta: false,
        },
        "Alt+Shift" => MainHotkeyModifiers {
            ctrl: false,
            alt: true,
            shift: true,
            meta: false,
        },
        "Ctrl+Alt+Shift" => MainHotkeyModifiers {
            ctrl: true,
            alt: true,
            shift: true,
            meta: false,
        },
        _ => MainHotkeyModifiers::meta(),
    }
}

fn main_hotkey_key_from_label(label: &str) -> MainHotkeyKey {
    let key = label.trim();
    if key.len() == 1 {
        let ch = key.as_bytes()[0] as char;
        if ch.is_ascii_alphabetic() {
            return MainHotkeyKey::Char(ch.to_ascii_uppercase());
        }
        if ch.is_ascii_digit() {
            return MainHotkeyKey::Char(ch);
        }
    }
    match key {
        "Space" => MainHotkeyKey::Space,
        "Enter" => MainHotkeyKey::Enter,
        "Tab" => MainHotkeyKey::Tab,
        "Esc" => MainHotkeyKey::Escape,
        "Backspace" => MainHotkeyKey::Backspace,
        "Delete" => MainHotkeyKey::Delete,
        "Insert" => MainHotkeyKey::Insert,
        "Up" => MainHotkeyKey::Up,
        "Down" => MainHotkeyKey::Down,
        "Left" => MainHotkeyKey::Left,
        "Right" => MainHotkeyKey::Right,
        "Home" => MainHotkeyKey::Home,
        "End" => MainHotkeyKey::End,
        "PageUp" => MainHotkeyKey::PageUp,
        "PageDown" => MainHotkeyKey::PageDown,
        _ => MainHotkeyKey::Char('V'),
    }
}

pub(crate) fn main_hotkey_spec_from_labels(mod_label: &str, key_label: &str) -> MainHotkeySpec {
    MainHotkeySpec {
        modifiers: main_hotkey_modifiers_from_label(mod_label),
        key: main_hotkey_key_from_label(key_label),
    }
}

pub(crate) fn main_hotkey_registration_plan(
    input: MainHotkeyRegistrationInput<'_>,
) -> MainHotkeyRegistrationPlan {
    MainHotkeyRegistrationPlan {
        unregister_existing: input.already_registered,
        register: input
            .enabled
            .then(|| main_hotkey_spec_from_labels(input.mod_label, input.key_label)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainShowSearchAction {
    ShowPersistent,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainShowPrepareInput {
    pub(crate) shared_tab_changed: bool,
    pub(crate) persistent_search_box: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainShowPreparePlan {
    pub(crate) clear_selection: bool,
    pub(crate) reset_scroll: bool,
    pub(crate) refilter: bool,
    pub(crate) search_action: MainShowSearchAction,
}

pub(crate) fn main_show_prepare_plan(input: MainShowPrepareInput) -> MainShowPreparePlan {
    MainShowPreparePlan {
        clear_selection: input.shared_tab_changed,
        reset_scroll: input.shared_tab_changed,
        refilter: input.shared_tab_changed,
        search_action: if input.persistent_search_box {
            MainShowSearchAction::ShowPersistent
        } else {
            MainShowSearchAction::Reset
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainSearchVisibilityRequest {
    Open,
    Close,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainSearchVisibilityAction {
    Open,
    ClosePersistent,
    CloseReset,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainSearchVisibilityInput {
    pub(crate) request: MainSearchVisibilityRequest,
    pub(crate) search_on: bool,
    pub(crate) search_text_empty: bool,
    pub(crate) persistent_search_box: bool,
    pub(crate) main_window_noactivate: bool,
    pub(crate) quick_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainSearchVisibilityPlan {
    pub(crate) action: MainSearchVisibilityAction,
    pub(crate) search_on: bool,
    pub(crate) activate_window: bool,
    pub(crate) stop_debounce_timer: bool,
    pub(crate) clear_search_text: bool,
    pub(crate) clear_selection: bool,
    pub(crate) refilter: bool,
    pub(crate) relayout: bool,
    pub(crate) invalidate: bool,
}

pub(crate) fn main_search_visibility_plan(
    input: MainSearchVisibilityInput,
) -> MainSearchVisibilityPlan {
    let request = match input.request {
        MainSearchVisibilityRequest::Toggle if input.search_on => {
            MainSearchVisibilityRequest::Close
        }
        MainSearchVisibilityRequest::Toggle => MainSearchVisibilityRequest::Open,
        request => request,
    };
    match request {
        MainSearchVisibilityRequest::Open => MainSearchVisibilityPlan {
            action: MainSearchVisibilityAction::Open,
            search_on: true,
            activate_window: input.main_window_noactivate || input.quick_window,
            stop_debounce_timer: false,
            clear_search_text: false,
            clear_selection: false,
            refilter: false,
            relayout: true,
            invalidate: true,
        },
        MainSearchVisibilityRequest::Close if input.persistent_search_box => {
            MainSearchVisibilityPlan {
                action: MainSearchVisibilityAction::ClosePersistent,
                search_on: true,
                activate_window: false,
                stop_debounce_timer: false,
                clear_search_text: false,
                clear_selection: false,
                refilter: false,
                relayout: true,
                invalidate: true,
            }
        }
        MainSearchVisibilityRequest::Close if !input.search_on && input.search_text_empty => {
            MainSearchVisibilityPlan {
                action: MainSearchVisibilityAction::Noop,
                search_on: input.search_on,
                activate_window: false,
                stop_debounce_timer: false,
                clear_search_text: false,
                clear_selection: false,
                refilter: false,
                relayout: false,
                invalidate: false,
            }
        }
        MainSearchVisibilityRequest::Close => MainSearchVisibilityPlan {
            action: MainSearchVisibilityAction::CloseReset,
            search_on: false,
            activate_window: false,
            stop_debounce_timer: true,
            clear_search_text: true,
            clear_selection: true,
            refilter: true,
            relayout: true,
            invalidate: true,
        },
        MainSearchVisibilityRequest::Toggle => unreachable!(),
    }
}
