#![allow(unused_imports)]

pub(crate) use zsui::{ComponentPhase, KeyState, LifecycleEvent, LifecycleState, MouseButton};

pub(crate) type UiEvent<AppEvent = super::ApplicationEvent> = zsui::UiEvent<AppEvent>;
