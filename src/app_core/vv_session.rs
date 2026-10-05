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

/// Cleaning the trigger is best-effort after the session and paste target are authorized.
/// An unavailable IME must not turn an explicitly selected candidate into a no-op.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VvSelectionCleanup {
    NoTextTrigger,
    RemoveLiteralTrigger,
    CompositionCancelled,
    PreserveUnobservedInput,
}

impl VvSelectionCleanup {
    pub(crate) const fn backspaces(self) -> u8 {
        match self {
            Self::RemoveLiteralTrigger => 2,
            _ => 0,
        }
    }
}

pub(crate) fn vv_selection_cleanup(
    triggered_by_text: bool,
    literal_trigger_confirmed: bool,
    cancel_exact_composition: impl FnOnce() -> bool,
) -> VvSelectionCleanup {
    if !triggered_by_text {
        VvSelectionCleanup::NoTextTrigger
    } else if literal_trigger_confirmed {
        VvSelectionCleanup::RemoveLiteralTrigger
    } else if cancel_exact_composition() {
        VvSelectionCleanup::CompositionCancelled
    } else {
        VvSelectionCleanup::PreserveUnobservedInput
    }
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

    pub fn cancel_selection(&mut self, id: u64) -> bool {
        if self.id != id || self.phase != VvPhase::Selected {
            return false;
        }
        self.cancel();
        true
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

    /// The visible popup commits an explicit selection. A keyboard request need
    /// not have changed the hook phase before its posted UI message is handled.
    pub fn accept_selection(&mut self, id: u64, index: usize, visible_count: usize) -> bool {
        if self.id != id || !matches!(self.phase, VvPhase::Visible | VvPhase::Selected)
            || index >= visible_count.min(9)
        {
            return false;
        }
        self.candidates = visible_count.min(9);
        self.phase = VvPhase::Selected;
        true
    }

    // Consumed key ownership survives cancellation and lasts through the physical release.
    pub fn key(&mut self, vk: u32, down: bool, modifiers: bool, same_target: bool) -> VvKeyResult {
        self.key_inner(vk, down, modifiers, same_target, true)
    }

    /// Windows selection is committed by the popup UI, which owns the rendered
    /// candidate list. The hook only requests it and owns the physical key cycle.
    pub fn key_request(&mut self, vk: u32, down: bool, modifiers: bool, same_target: bool) -> VvKeyResult {
        self.key_inner(vk, down, modifiers, same_target, false)
    }

    fn key_inner(&mut self, vk: u32, down: bool, modifiers: bool, same_target: bool, commit_selection: bool) -> VvKeyResult {
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
            // Some IMEs switch language on Shift release. Invalidate even when
            // the corresponding press was missed, while forwarding Shift itself.
            if !commit_selection && matches!(vk, 0x10 | 0xa0 | 0xa1)
                && (self.active() || self.phase == VvPhase::Selected)
            {
                self.cancel();
                result.action = VvKeyAction::Hide;
            }
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
            if commit_selection { self.phase = VvPhase::Selected; }
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
    fn confirmed_english_trigger_removes_two_characters_without_ime_calls() {
        let plan =
            vv_selection_cleanup(true, true, || panic!("literal trigger must not cancel IME"));
        assert_eq!(plan, VvSelectionCleanup::RemoveLiteralTrigger);
        assert_eq!(plan.backspaces(), 2);
    }

    #[test]
    fn exact_native_composition_can_be_cancelled_without_backspacing() {
        let mut calls = 0;
        let plan = vv_selection_cleanup(true, false, || {
            calls += 1;
            true
        });
        assert_eq!(calls, 1);
        assert_eq!(plan, VvSelectionCleanup::CompositionCancelled);
        assert_eq!(plan.backspaces(), 0);
    }

    #[test]
    fn native_or_unknown_ime_cleanup_failure_still_produces_a_paste_plan() {
        // Missing context, unavailable composition, non-exact composition and failed
        // cancellation all use the same host result; none authorize destructive cleanup.
        for failure in [
            "native-unobservable",
            "unknown-context",
            "non-vv-composition",
            "cancel-failed",
        ] {
            let mut calls = 0;
            let plan = vv_selection_cleanup(true, false, || {
                calls += 1;
                false
            });
            assert_eq!(calls, 1, "{failure}");
            assert_eq!(
                plan,
                VvSelectionCleanup::PreserveUnobservedInput,
                "{failure}"
            );
            assert_eq!(plan.backspaces(), 0, "{failure}");
        }
    }

    #[test]
    fn non_text_invocation_preserves_input_without_ime_calls() {
        let plan = vv_selection_cleanup(false, false, || {
            panic!("no trigger belongs to this session")
        });
        assert_eq!(plan, VvSelectionCleanup::NoTextTrigger);
        assert_eq!(plan.backspaces(), 0);
    }

    #[test]
    fn failed_selection_releases_session_but_owns_the_selected_key_until_release() {
        let mut s = session();
        s.show(s.id, 1);
        assert_eq!(
            s.key(0x31, true, false, true).action,
            VvKeyAction::Select(0)
        );
        assert!(s.cancel_selection(s.id));
        assert_eq!(s.phase, VvPhase::Cancelled);
        assert!(s.key(0x31, true, false, true).consume);
        assert!(s.key(0x31, false, false, true).consume);
        let first_v = s.key(0x56, true, false, true);
        assert!(!first_v.consume);
        assert_eq!(first_v.action, VvKeyAction::None);
    }

    #[test]
    fn failed_selection_callback_cannot_cancel_a_newer_or_visible_session() {
        let mut s = session();
        let old = s.id;
        s.show(old, 1);
        s.select(old, 0);
        let next = s.begin(10, 11, true);
        assert!(!s.cancel_selection(old));
        assert_eq!(s.phase, VvPhase::Pending);
        s.show(next, 1);
        assert!(!s.cancel_selection(next));
        assert_eq!(s.phase, VvPhase::Visible);
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
    fn posted_digit_request_is_committed_by_visible_ui_and_keeps_key_ownership() {
        let mut s = session();
        s.show(s.id, 2);
        let id = s.id;
        assert_eq!(s.key_request(0x31, true, false, true).action, VvKeyAction::Select(0));
        assert_eq!(s.phase, VvPhase::Visible);
        assert!(s.key_request(0x31, true, false, true).consume);
        assert!(s.accept_selection(id, 0, 2));
        assert!(s.accept_selection(id, 0, 2));
        assert_eq!(s.phase, VvPhase::Selected);
        assert!(s.key_request(0x31, false, false, true).consume);
    }

    #[test]
    fn ui_selection_does_not_revive_cancelled_or_superseded_requests() {
        let mut s = session();
        let id = s.id;
        assert!(!s.accept_selection(id, 0, 2));
        s.show(id, 2);
        assert!(!s.accept_selection(id, 2, 2));
        s.key_request(0x31, true, false, true);
        assert_eq!(s.key_request(0x1b, true, false, true).action, VvKeyAction::Hide);
        assert!(!s.accept_selection(id, 0, 2));
        assert!(s.key_request(0x31, false, false, true).consume);
        let next = s.begin(10, 11, true);
        s.show(next, 2);
        assert!(!s.accept_selection(id, 0, 2));
        assert!(s.accept_selection(next, 1, 2));
    }

    #[test]
    fn modifier_down_cancels_each_live_phase_and_forwards_the_whole_cycle() {
        for phase in [VvPhase::Pending, VvPhase::Visible, VvPhase::Selected] {
            for key in [0x10, 0xa0, 0xa1, 0x11, 0xa2, 0xa3, 0x12, 0xa4, 0xa5, 0x5b, 0x5c] {
                let mut s = session_in_phase(phase);
                let id = s.id;
                let down = s.key_request(key, true, true, true);
                assert!(!down.consume, "modifier {key:x} in {phase:?} must reach its target");
                assert_eq!(down.action, VvKeyAction::Hide);
                assert_eq!(s.phase, VvPhase::Cancelled);
                assert!(!s.show(id, 2), "a queued show must not survive a mode change");
                assert!(!s.accept_selection(id, 0, 2), "a queued selection must not revive cancellation");
                let held = s.key_request(key, true, true, true);
                assert!(held.repeat);
                assert!(!held.consume);
                assert_eq!(held.action, VvKeyAction::None);
                let up = s.key_request(key, false, false, true);
                assert!(!up.consume);
                assert_eq!(up.action, VvKeyAction::None);
                let text = s.key_request(0x4a, true, false, true);
                assert!(!text.consume);
                assert_eq!(text.action, VvKeyAction::None);
            }
        }
    }

    fn session_in_phase(phase: VvPhase) -> VvInputSession {
        let mut s = session();
        if phase != VvPhase::Pending { assert!(s.show(s.id, 2)); }
        if phase == VvPhase::Selected { assert!(s.accept_selection(s.id, 0, 2)); }
        assert_eq!(s.phase, phase);
        s
    }

    #[test]
    fn missed_shift_down_still_cancels_on_generic_left_or_right_shift_up() {
        for phase in [VvPhase::Pending, VvPhase::Visible, VvPhase::Selected] {
            for key in [0x10, 0xa0, 0xa1] {
                let mut s = session_in_phase(phase);
                let id = s.id;
                let up = s.key_request(key, false, false, true);
                assert!(!up.consume);
                assert_eq!(up.action, VvKeyAction::Hide);
                assert_eq!(s.phase, VvPhase::Cancelled);
                assert!(!s.show(id, 2));
                assert!(!s.accept_selection(id, 0, 2));
                assert_eq!(s.key_request(key, false, false, true).action, VvKeyAction::None);
            }
        }
    }

    #[test]
    fn shift_cancellation_preserves_owned_digit_and_escape_releases() {
        for shift_down in [false, true] {
            let mut s = session_in_phase(VvPhase::Visible);
            assert_eq!(s.key_request(0x31, true, false, true).action, VvKeyAction::Select(0));
            assert!(s.accept_selection(s.id, 0, 2));
            assert_eq!(s.key_request(0xa0, shift_down, shift_down, true).action, VvKeyAction::Hide);
            assert!(s.key_request(0x31, true, false, true).consume);
            assert!(s.key_request(0x31, false, false, true).consume);
            assert!(!s.key_request(0x31, true, false, true).consume);

            let mut s = session_in_phase(VvPhase::Visible);
            assert!(s.key_request(0x1b, true, false, true).consume);
            assert!(!s.key_request(0xa1, shift_down, shift_down, true).consume);
            assert!(s.key_request(0x1b, true, false, true).consume);
            assert!(s.key_request(0x1b, false, false, true).consume);
            assert!(!s.key_request(0x1b, true, false, true).consume);
        }
    }

    #[test]
    fn other_key_releases_do_not_cancel_windows_or_native_session_behavior() {
        for phase in [VvPhase::Pending, VvPhase::Visible, VvPhase::Selected] {
            for key in [0x11, 0x12, 0x5b, 0x41] {
                let mut s = session_in_phase(phase);
                let up = s.key_request(key, false, false, true);
                assert!(!up.consume);
                assert_eq!(up.action, VvKeyAction::None);
                assert_eq!(s.phase, phase);
            }
            let mut s = session_in_phase(phase);
            assert_eq!(s.key(0x10, false, false, true).action, VvKeyAction::None);
            assert_eq!(s.phase, phase);
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
