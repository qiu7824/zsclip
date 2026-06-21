use super::{ApplicationEvent, Command, Point, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComponentPhase {
    New,
    Mounted,
    Active,
    Suspended,
    Unmounted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecycleEvent {
    Mount,
    Resume,
    Suspend,
    Unmount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LifecycleState {
    phase: ComponentPhase,
}

impl LifecycleState {
    pub(crate) fn new() -> Self {
        Self {
            phase: ComponentPhase::New,
        }
    }

    pub(crate) fn phase(self) -> ComponentPhase {
        self.phase
    }

    pub(crate) fn apply(&mut self, event: LifecycleEvent) -> bool {
        let next = match (self.phase, event) {
            (ComponentPhase::New, LifecycleEvent::Mount) => ComponentPhase::Mounted,
            (ComponentPhase::Mounted | ComponentPhase::Suspended, LifecycleEvent::Resume) => {
                ComponentPhase::Active
            }
            (ComponentPhase::Active, LifecycleEvent::Suspend) => ComponentPhase::Suspended,
            (
                ComponentPhase::Mounted | ComponentPhase::Active | ComponentPhase::Suspended,
                LifecycleEvent::Unmount,
            ) => ComponentPhase::Unmounted,
            _ => return false,
        };
        self.phase = next;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyState {
    Down,
    Up,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiEvent<AppEvent = ApplicationEvent> {
    Application(AppEvent),
    Lifecycle(LifecycleEvent),
    PointerMove {
        position: Point,
    },
    PointerHover {
        position: Point,
    },
    PointerLeave,
    PointerCancel,
    PointerButton {
        position: Point,
        button: MouseButton,
        pressed: bool,
        click_count: u8,
    },
    MouseWheel {
        delta: i32,
    },
    Key {
        code: u32,
        state: KeyState,
        system: bool,
    },
    TextInput(String),
    Command(Command),
    ControlCommand {
        control_id: u32,
        notification: u16,
    },
    ControlSelectionChanged {
        control_id: u32,
        index: usize,
    },
    GlobalHotkey {
        id: i32,
    },
    ClipboardChanged,
    Timer {
        id: u64,
    },
    WindowSize {
        size: Size,
        minimized: bool,
    },
    AppActivationChanged {
        active: bool,
    },
    SystemMetricsChanged,
    WindowMoved,
    WindowMoveCompleted,
    CloseRequested,
    ThemeChanged,
    DpiChanged {
        dpi: u32,
    },
}
