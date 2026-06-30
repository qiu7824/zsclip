use serde::{Deserialize, Serialize};

use crate::zsui::capability::{CapabilityStatus, CapabilitySupport, HostCapabilities};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSpec {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub visible: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub transparent: bool,
}

pub type Window = WindowSpec;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowNativeOptions {
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub resizable: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub transparent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowResolvedSpec {
    pub requested: WindowSpec,
    pub effective: WindowSpec,
    pub degraded_capabilities: Vec<String>,
}

impl WindowSpec {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 900,
            height: 620,
            min_width: None,
            min_height: None,
            visible: true,
            resizable: true,
            decorations: true,
            always_on_top: false,
            transparent: false,
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn native_options(&self) -> WindowNativeOptions {
        WindowNativeOptions {
            min_width: self.min_width,
            min_height: self.min_height,
            resizable: self.resizable,
            decorations: self.decorations,
            always_on_top: self.always_on_top,
            transparent: self.transparent,
        }
    }

    pub fn resolve_for(&self, capabilities: &HostCapabilities) -> WindowResolvedSpec {
        let mut effective = self.clone();
        if capabilities.window_resizing.status == CapabilityStatus::Unsupported {
            effective.resizable = true;
            effective.min_width = None;
            effective.min_height = None;
        }
        if capabilities.window_decorations.status == CapabilityStatus::Unsupported {
            effective.decorations = true;
        }
        if capabilities.window_always_on_top.status == CapabilityStatus::Unsupported {
            effective.always_on_top = false;
        }
        if capabilities.window_transparency.status == CapabilityStatus::Unsupported {
            effective.transparent = false;
        }

        WindowResolvedSpec {
            requested: self.clone(),
            effective,
            degraded_capabilities: self.degraded_capabilities(capabilities),
        }
    }

    pub fn degraded_capabilities(&self, capabilities: &HostCapabilities) -> Vec<String> {
        let mut degraded = Vec::new();
        push_if_degraded(
            &mut degraded,
            "windows",
            &capabilities.windows,
            "native window creation",
            "record the declaration and keep application state alive",
        );
        push_if_degraded(
            &mut degraded,
            "window_resizing",
            &capabilities.window_resizing,
            if self.resizable {
                "resizable window"
            } else {
                "fixed-size window"
            },
            if self.resizable {
                "host may create a fixed-size or custom window"
            } else {
                "host may leave the window resizable"
            },
        );
        push_if_degraded(
            &mut degraded,
            "window_decorations",
            &capabilities.window_decorations,
            if self.decorations {
                "native decorated window"
            } else {
                "undecorated window"
            },
            if self.decorations {
                "host may use custom chrome or an undecorated surface"
            } else {
                "host may keep native decorations"
            },
        );
        if self.always_on_top {
            push_if_degraded(
                &mut degraded,
                "window_always_on_top",
                &capabilities.window_always_on_top,
                "always-on-top window",
                "host may present a normal z-order window",
            );
        }
        if self.transparent {
            push_if_degraded(
                &mut degraded,
                "window_transparency",
                &capabilities.window_transparency,
                "transparent window",
                "host may present an opaque native window",
            );
        }
        degraded
    }
}

fn push_if_degraded(
    degraded: &mut Vec<String>,
    capability: &'static str,
    support: &CapabilitySupport,
    requested: &'static str,
    fallback: &'static str,
) {
    if support.status == CapabilityStatus::Supported {
        return;
    }
    degraded.push(format!(
        "{capability}: requested {requested}; fallback {fallback}; {}",
        support.detail
    ));
}
