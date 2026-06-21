#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct UiRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl UiRect {
    pub(crate) const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub(crate) const fn offset_y(self, dy: i32) -> Self {
        Self {
            left: self.left,
            top: self.top - dy,
            right: self.right,
            bottom: self.bottom - dy,
        }
    }

    pub(crate) const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    pub(crate) const fn width(self) -> i32 {
        self.right - self.left
    }

    pub(crate) const fn height(self) -> i32 {
        self.bottom - self.top
    }

    pub(crate) const fn inflate(self, dx: i32, dy: i32) -> Self {
        Self {
            left: self.left - dx,
            top: self.top - dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }
}

pub(crate) fn clamp_window_pos_to_rect(
    x: i32,
    y: i32,
    bounds: UiRect,
    win_w: i32,
    win_h: i32,
) -> (i32, i32) {
    let max_x = bounds.left.max(bounds.right - win_w);
    let max_y = bounds.top.max(bounds.bottom - win_h);
    (bounds.left.max(x.min(max_x)), bounds.top.max(y.min(max_y)))
}

pub(crate) fn dpi_compensated_size(
    base_w: i32,
    base_h: i32,
    base_monitor_dpi: u32,
    monitor_dpi: u32,
) -> (i32, i32) {
    let base_monitor_dpi = base_monitor_dpi.max(96) as i64;
    let monitor_dpi = monitor_dpi.max(96) as i64;
    let w = (((base_w.max(1) as i64) * base_monitor_dpi) + (monitor_dpi / 2)) / monitor_dpi;
    let h = (((base_h.max(1) as i64) * base_monitor_dpi) + (monitor_dpi / 2)) / monitor_dpi;
    (w.max(1) as i32, h.max(1) as i32)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DpiCompensationState {
    base_w: i32,
    base_h: i32,
    base_monitor_dpi: u32,
    last_monitor_dpi: u32,
    applying: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DpiCompensationPlan {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) monitor_dpi: u32,
}

impl DpiCompensationState {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn is_applying(self) -> bool {
        self.applying
    }

    pub(crate) fn set_applying(&mut self, applying: bool) {
        self.applying = applying;
    }

    pub(crate) fn set_base(&mut self, width: i32, height: i32, monitor_dpi: u32) {
        self.base_w = width.max(1);
        self.base_h = height.max(1);
        self.base_monitor_dpi = monitor_dpi.max(96);
        self.last_monitor_dpi = self.base_monitor_dpi;
    }

    pub(crate) fn ensure_base(&mut self, width: i32, height: i32, monitor_dpi: u32) -> bool {
        if self.base_monitor_dpi == 0 || self.base_w <= 0 || self.base_h <= 0 {
            self.set_base(width, height, monitor_dpi);
            true
        } else {
            false
        }
    }

    pub(crate) fn target_size(self, monitor_dpi: u32) -> Option<(i32, i32)> {
        if self.base_monitor_dpi == 0 || self.base_w <= 0 || self.base_h <= 0 {
            None
        } else {
            Some(dpi_compensated_size(
                self.base_w,
                self.base_h,
                self.base_monitor_dpi,
                monitor_dpi,
            ))
        }
    }

    pub(crate) fn already_at_target(
        self,
        monitor_dpi: u32,
        current_w: i32,
        current_h: i32,
        target_w: i32,
        target_h: i32,
        tolerance: i32,
    ) -> bool {
        self.last_monitor_dpi == monitor_dpi.max(96)
            && (current_w - target_w).abs() <= tolerance
            && (current_h - target_h).abs() <= tolerance
    }

    pub(crate) fn finish_resize(&mut self, monitor_dpi: u32) {
        self.applying = false;
        self.last_monitor_dpi = monitor_dpi.max(96);
    }

    pub(crate) fn resize_plan(
        &mut self,
        current: UiRect,
        bounds: UiRect,
        monitor_dpi: u32,
        tolerance: i32,
    ) -> Option<DpiCompensationPlan> {
        let cur_w = current.right - current.left;
        let cur_h = current.bottom - current.top;
        if cur_w <= 0 || cur_h <= 0 {
            return None;
        }
        let monitor_dpi = monitor_dpi.max(96);
        if self.ensure_base(cur_w, cur_h, monitor_dpi) {
            return None;
        }
        let (mut target_w, mut target_h) = self.target_size(monitor_dpi)?;
        target_w = target_w.min((bounds.right - bounds.left).max(1)).max(1);
        target_h = target_h.min((bounds.bottom - bounds.top).max(1)).max(1);
        if self.already_at_target(monitor_dpi, cur_w, cur_h, target_w, target_h, tolerance) {
            return None;
        }
        let center_x = current.left + cur_w / 2;
        let center_y = current.top + cur_h / 2;
        let (x, y) = clamp_window_pos_to_rect(
            center_x - target_w / 2,
            center_y - target_h / 2,
            bounds,
            target_w,
            target_h,
        );
        Some(DpiCompensationPlan {
            x,
            y,
            width: target_w,
            height: target_h,
            monitor_dpi,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ComponentId(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Size {
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl Size {
    pub(crate) fn clamp_non_negative(self) -> Self {
        Self {
            width: self.width.max(0),
            height: self.height.max(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl Rect {
    pub(crate) fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayoutInput {
    pub(crate) bounds: Rect,
    pub(crate) scale: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LayoutOutput {
    pub(crate) bounds: Rect,
    pub(crate) children: Vec<LayoutNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LayoutNode {
    pub(crate) component: ComponentId,
    pub(crate) bounds: Rect,
}

pub(crate) trait LayoutProtocol {
    fn layout(&mut self, input: LayoutInput) -> LayoutOutput;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedUiProtocol {
    Command,
    LayoutProtocol,
    Component,
}

impl SharedUiProtocol {
    pub(crate) const fn protocol_name(self) -> &'static str {
        match self {
            Self::Command => "Command",
            Self::LayoutProtocol => "LayoutProtocol",
            Self::Component => "Component",
        }
    }
}

pub(crate) const SHARED_NON_HOST_UI_PROTOCOLS: [SharedUiProtocol; 3] = [
    SharedUiProtocol::Command,
    SharedUiProtocol::LayoutProtocol,
    SharedUiProtocol::Component,
];
