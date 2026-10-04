//! Input ownership for a non-activating VV session. No text or platform state is stored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VvPhase {
    #[default]
    Idle,
    Pending,
    Visible,
    Cancelled,
    Selected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VvKeyAction {
    None,
    Hide,
    Select(usize),
    Navigate(i32),
    Scroll(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VvKeyResult {
    pub consume: bool,
    pub repeat: bool,
    pub action: VvKeyAction,
}

pub(crate) struct VvInputSession {
    pub id: u64,
    pub phase: VvPhase,
    pub target: usize,
    pub focus: usize,
    pub triggered_by_text: bool,
    pub candidates: usize,
    pressed: [bool; 256],
    owned: [bool; 256],
}

impl Default for VvInputSession {
    fn default() -> Self {
        Self {
            id: 0,
            phase: VvPhase::Idle,
            target: 0,
            focus: 0,
            triggered_by_text: false,
            candidates: 0,
            pressed: [false; 256],
            owned: [false; 256],
        }
    }
}

impl VvInputSession {
    pub fn active(&self) -> bool {
        matches!(self.phase, VvPhase::Pending | VvPhase::Visible)
    }

    pub fn begin(&mut self, target: usize, focus: usize, triggered_by_text: bool) -> u64 {
        self.id = self.id.wrapping_add(1).max(1);
        self.phase = VvPhase::Pending;
        self.target = target;
        self.focus = focus;
        self.triggered_by_text = triggered_by_text;
        self.candidates = 0;
        self.id
    }

    pub fn cancel(&mut self) {
        self.phase = VvPhase::Cancelled;
        self.candidates = 0;
    }

    pub fn matches(&self, id: u64, target: usize, focus: usize) -> bool {
        self.id == id && self.target == target && self.focus == focus && target != 0 && focus != 0
    }

    pub fn show(&mut self, id: u64, count: usize) -> bool {
        if self.id != id || self.phase != VvPhase::Pending {
            return false;
        }
        self.phase = VvPhase::Visible;
        self.candidates = count.min(9);
        true
    }

    pub fn select(&mut self, id: u64, index: usize) -> bool {
        if self.id != id || self.phase != VvPhase::Visible || index >= self.candidates {
            return false;
        }
        self.phase = VvPhase::Selected;
        true
    }

    // Consumed key ownership survives cancellation and lasts through the physical release.
    pub fn key(&mut self, vk: u32, down: bool, modifiers: bool, same_target: bool) -> VvKeyResult {
        let slot = vk as usize;
        let repeat = down && self.pressed.get(slot).copied().unwrap_or(false);
        if let Some(pressed) = self.pressed.get_mut(slot) {
            *pressed = down;
        }
        let mut result = VvKeyResult {
            consume: false,
            repeat,
            action: VvKeyAction::None,
        };
        if let Some(owned) = self.owned.get_mut(slot) {
            if *owned {
                result.consume = true;
                if !down {
                    *owned = false;
                }
                return result;
            }
        }
        if !down {
            return result;
        }
        if self.phase == VvPhase::Selected {
            self.cancel();
            result.action = VvKeyAction::Hide;
            return result;
        }
        if !self.active() {
            return result;
        }
        if !same_target || modifiers {
            self.cancel();
            result.action = VvKeyAction::Hide;
            return result;
        }
        let digit = match vk {
            0x31..=0x39 => Some((vk - 0x31) as usize),
            0x61..=0x69 => Some((vk - 0x61) as usize),
            _ => None,
        };
        result.action = if vk == 0x1b {
            self.cancel();
            result.consume = true;
            VvKeyAction::Hide
        } else if let Some(index) =
            digit.filter(|&i| self.phase == VvPhase::Visible && i < self.candidates)
        {
            self.phase = VvPhase::Selected;
            result.consume = true;
            VvKeyAction::Select(index)
        } else if self.phase == VvPhase::Visible && self.candidates > 0 && matches!(vk, 0x26 | 0x28)
        {
            result.consume = true;
            VvKeyAction::Navigate(if vk == 0x26 { -1 } else { 1 })
        } else if self.phase == VvPhase::Visible && matches!(vk, 0x21 | 0x22) {
            result.consume = true;
            VvKeyAction::Scroll(if vk == 0x21 { -1 } else { 1 })
        } else {
            self.cancel();
            VvKeyAction::Hide
        };
        if result.consume {
            if let Some(owned) = self.owned.get_mut(slot) {
                *owned = true;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> VvInputSession {
        let mut s = VvInputSession::default();
        s.begin(10, 11, true);
        s
    }
    #[test]
    fn escape_owns_down_repeat_up_then_releases_next_press() {
        let mut s = session();
        assert_eq!(s.key(0x1b, true, false, true).action, VvKeyAction::Hide);
        assert!(s.key(0x1b, true, false, true).consume);
        assert!(s.key(0x1b, false, false, true).consume);
        assert!(!s.key(0x1b, true, false, true).consume);
        assert!(!s.key(b'J' as u32, true, false, true).consume);
    }
    #[test]
    fn cancellation_invalidates_pending_and_old_session_messages() {
        let mut s = session();
        let old = s.id;
        assert!(!s.key(b'A' as u32, true, false, true).consume);
        assert!(!s.show(old, 9));
        s.begin(10, 11, true);
        assert!(!s.show(old, 9));
        assert!(!s.matches(s.id, 10, 12));
    }
    #[test]
    fn only_visible_unmodified_valid_digit_selects_once() {
        for key in [0x31, 0x61] {
            let mut s = session();
            s.show(s.id, 1);
            assert_eq!(s.key(key, true, false, true).action, VvKeyAction::Select(0));
            assert_eq!(s.key(key, true, false, true).action, VvKeyAction::None);
            assert!(s.key(key, false, false, true).consume);
        }
        for (key, mods, same) in [
            (0x31, true, true),
            (0x39, false, true),
            (0x31, false, false),
            (0x08, false, true),
        ] {
            let mut s = session();
            s.show(s.id, 1);
            let r = s.key(key, true, mods, same);
            assert!(!r.consume);
            assert_eq!(r.action, VvKeyAction::Hide);
        }
    }
    #[test]
    fn trigger_autorepeat_and_navigation_are_distinct() {
        let mut s = VvInputSession::default();
        assert!(!s.key(0x56, true, false, true).repeat);
        assert!(s.key(0x56, true, false, true).repeat);
        s.key(0x56, false, false, true);
        assert!(!s.key(0x56, true, false, true).repeat);
        s.begin(10, 11, true);
        s.show(s.id, 3);
        assert_eq!(
            s.key(0x28, true, false, true).action,
            VvKeyAction::Navigate(1)
        );
        assert!(s.key(0x28, false, false, true).consume);
    }
}
