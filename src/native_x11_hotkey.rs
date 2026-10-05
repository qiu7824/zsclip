use crate::native_hotkey::{NativeHotkeyAction, NativeHotkeyBindings, NativeHotkeyCycles};
use std::collections::{HashMap, HashSet};

/// X11 may represent repeat as a release followed by a press with the same
/// server timestamp. Delay final release ownership until that pair can arrive.
#[derive(Default)]
struct HotkeyKeyCycles {
    logical: NativeHotkeyCycles,
    held: HashSet<u8>,
    releases: HashMap<u8, (u32, u64)>,
}

impl HotkeyKeyCycles {
    fn press(
        &mut self,
        bindings: &NativeHotkeyBindings,
        code: u8,
        label: &str,
        modifiers: u8,
        time: u32,
    ) -> crate::native_hotkey::NativeHotkeyDecision {
        if let Some((released_at, _)) = self.releases.remove(&code) {
            if released_at != time {
                self.finish_release(code);
            }
        }
        let decision = self.logical.handle(
            bindings,
            u16::from(code),
            label,
            modifiers,
            true,
            self.held.contains(&code),
        );
        if decision.consume {
            self.held.insert(code);
        }
        decision
    }

    fn release(&mut self, code: u8, time: u32, now: u64) -> bool {
        if !self.held.contains(&code) {
            return false;
        }
        self.releases.insert(code, (time, now));
        true
    }

    fn finish_release(&mut self, code: u8) {
        self.held.remove(&code);
        self.logical.handle(
            &NativeHotkeyBindings::default(),
            u16::from(code),
            "",
            0,
            false,
            false,
        );
    }

    fn finish_releases(&mut self, now: u64) {
        let finished = self
            .releases
            .iter()
            .filter(|(_, (_, seen))| now.saturating_sub(*seen) >= 12)
            .map(|(&code, _)| code)
            .collect::<Vec<_>>();
        for code in finished {
            self.releases.remove(&code);
            self.finish_release(code);
        }
    }
}

fn key_symbol(label: &str) -> Option<u32> {
    if label.len() == 1 {
        let value = label.as_bytes()[0];
        if value.is_ascii_alphanumeric() {
            return Some(u32::from(value.to_ascii_lowercase()));
        }
    }
    Some(match label {
        "Space" => 0x20,
        "Enter" => 0xff0d,
        "Tab" => 0xff09,
        "Esc" => 0xff1b,
        "Backspace" => 0xff08,
        "Delete" => 0xffff,
        "Insert" => 0xff63,
        "Up" => 0xff52,
        "Down" => 0xff54,
        "Left" => 0xff51,
        "Right" => 0xff53,
        "Home" => 0xff50,
        "End" => 0xff57,
        "PageUp" => 0xff55,
        "PageDown" => 0xff56,
        _ => return None,
    })
}

#[derive(Default)]
pub(crate) struct X11HotkeyRegistration {
    pub(crate) registered: usize,
    pub(crate) errors: Vec<String>,
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use crate::native_hotkey::{ALT, CTRL, SHIFT, SUPER};
    use std::time::Instant;
    use x11rb::connection::Connection;
    use x11rb::protocol::xkb::{ConnectionExt as _, PerClientFlag, ID};
    use x11rb::protocol::xproto::{ConnectionExt, EventMask, GrabMode, KeyPressEvent, ModMask};
    use x11rb::protocol::Event;
    use x11rb::rust_connection::RustConnection;

    pub(crate) fn enable_x11_detectable_autorepeat(
        connection: &RustConnection,
    ) -> Result<(), String> {
        let xkb = connection
            .xkb_use_extension(1, 0)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        if !xkb.supported {
            return Err("X11 key ownership requires XKB detectable autorepeat".into());
        }
        let flag = PerClientFlag::DETECTABLE_AUTO_REPEAT;
        let flags = connection
            .xkb_per_client_flags(
                ID::USE_CORE_KBD.into(),
                flag,
                flag,
                0u8.into(),
                0u8.into(),
                0u8.into(),
            )
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        if u32::from(flags.supported) & u32::from(flag) == 0
            || u32::from(flags.value) & u32::from(flag) == 0
        {
            return Err(
                "X11 server did not enable detectable autorepeat; key ownership remains disabled"
                    .into(),
            );
        }
        Ok(())
    }

    pub(crate) struct X11HotkeyActivation {
        pub(crate) action: NativeHotkeyAction,
        pub(crate) target: Option<crate::linux_app::LinuxNativeVvTarget>,
    }

    pub(crate) struct X11HotkeyRegistry {
        connection: RustConnection,
        root: u32,
        desired: NativeHotkeyBindings,
        registered: NativeHotkeyBindings,
        labels: HashMap<u8, String>,
        grabs: Vec<(u8, u16)>,
        alt_mask: u16,
        super_mask: u16,
        lock_mask: u16,
        original_focus: Option<u32>,
        pending: Vec<X11HotkeyActivation>,
        cycle_cancelled: bool,
        cycles: HotkeyKeyCycles,
        started: Instant,
        map_changed: bool,
    }

    impl X11HotkeyRegistry {
        pub(crate) fn connect() -> Result<Self, String> {
            let (connection, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
            let root = connection.setup().roots[screen].root;
            enable_x11_detectable_autorepeat(&connection)?;
            Ok(Self {
                connection,
                root,
                desired: NativeHotkeyBindings::default(),
                registered: NativeHotkeyBindings::default(),
                labels: HashMap::new(),
                grabs: Vec::new(),
                alt_mask: 0,
                super_mask: 0,
                lock_mask: u16::from(ModMask::LOCK),
                original_focus: None,
                pending: Vec::new(),
                cycle_cancelled: false,
                cycles: HotkeyKeyCycles::default(),
                started: Instant::now(),
                map_changed: false,
            })
        }

        fn remove_passive_grabs(&mut self) -> Result<(), String> {
            let mut failure = None;
            for (code, modifiers) in self.grabs.drain(..) {
                if let Err(error) =
                    self.connection
                        .ungrab_key(code, self.root, ModMask::from(modifiers))
                {
                    failure = Some(error.to_string());
                }
            }
            self.connection.flush().map_err(|e| e.to_string())?;
            // The round trip also brings every event ordered before removal
            // into this connection's queue, without releasing an active grab.
            self.connection
                .get_input_focus()
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?;
            failure.map_or(Ok(()), Err)
        }

        pub(crate) fn replace_bindings(
            &mut self,
            desired: NativeHotkeyBindings,
        ) -> X11HotkeyRegistration {
            let mut report = X11HotkeyRegistration {
                registered: 0,
                errors: desired.errors.clone(),
            };
            if let Err(error) = self.remove_passive_grabs() {
                report
                    .errors
                    .push(format!("Unable to release global shortcuts: {error}"));
            }
            // Events already accepted by the old grabs still own their keyup,
            // but a settings change must not execute those queued actions.
            if let Err(error) = self.poll_events() {
                report.errors.push(error);
            }
            self.pending.clear();
            self.cycle_cancelled = !self.cycles.held.is_empty();
            self.desired = desired;
            self.registered = NativeHotkeyBindings::default();
            self.labels.clear();
            self.map_changed = false;
            if let Err(error) = self.install_bindings(&mut report) {
                report.errors.push(error);
            }
            report
        }

        fn install_bindings(&mut self, report: &mut X11HotkeyRegistration) -> Result<(), String> {
            if self.desired.bindings.is_empty() {
                return Ok(());
            }
            let min = self.connection.setup().min_keycode;
            let mapping = self
                .connection
                .get_keyboard_mapping(min, self.connection.setup().max_keycode - min + 1)
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?;
            if mapping.keysyms_per_keycode == 0 {
                return Err("X11 keyboard map is empty".into());
            }
            let keycode = |symbol: u32| {
                mapping
                    .keysyms
                    .chunks(usize::from(mapping.keysyms_per_keycode))
                    .position(|symbols| symbols.first() == Some(&symbol))
                    .map(|index| min + index as u8)
            };
            let modifiers = self
                .connection
                .get_modifier_mapping()
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?;
            let modifier_mask = |symbols: &[u32]| {
                modifiers
                    .keycodes
                    .chunks(usize::from(modifiers.keycodes_per_modifier().max(1)))
                    .enumerate()
                    .filter(|(_, codes)| {
                        symbols
                            .iter()
                            .filter_map(|symbol| keycode(*symbol))
                            .any(|code| codes.contains(&code))
                    })
                    .fold(0u16, |mask, (index, _)| mask | (1u16 << index))
            };
            self.alt_mask = modifier_mask(&[0xffe9, 0xffea]);
            self.super_mask = modifier_mask(&[0xffeb, 0xffec]);
            let num_lock_mask = modifier_mask(&[0xff7f]);
            self.lock_mask = u16::from(ModMask::LOCK) | num_lock_mask;
            let mut locks = vec![0, u16::from(ModMask::LOCK), num_lock_mask, self.lock_mask];
            locks.sort_unstable();
            locks.dedup();
            for binding in &self.desired.bindings {
                let Some(code) = key_symbol(&binding.key).and_then(keycode) else {
                    report.errors.push(format!(
                        "{}: key is unavailable in the X11 keyboard layout",
                        binding.key
                    ));
                    continue;
                };
                let Some(base_mask) = self.x11_modifiers(binding.modifiers) else {
                    report.errors.push(format!(
                        "{}: requested modifier is unavailable in the X11 keyboard layout",
                        binding.key
                    ));
                    continue;
                };
                let mut installed = Vec::new();
                let mut failed = None;
                for lock in &locks {
                    let mask = base_mask | lock;
                    match self.connection.grab_key(
                        false,
                        self.root,
                        ModMask::from(mask),
                        code,
                        GrabMode::ASYNC,
                        GrabMode::ASYNC,
                    ) {
                        Ok(cookie) => match cookie.check() {
                            Ok(()) => installed.push((code, mask)),
                            Err(error) => {
                                failed = Some(error.to_string());
                                break;
                            }
                        },
                        Err(error) => {
                            failed = Some(error.to_string());
                            break;
                        }
                    }
                }
                if let Some(error) = failed {
                    for (code, mask) in installed {
                        let _ = self
                            .connection
                            .ungrab_key(code, self.root, ModMask::from(mask));
                    }
                    report.errors.push(format!("{}: global shortcut registration failed (it may be owned by another application): {error}", binding.key));
                } else {
                    self.grabs.extend(installed);
                    self.labels.insert(code, binding.key.clone());
                    self.registered.bindings.push(binding.clone());
                    report.registered += 1;
                }
            }
            self.connection.flush().map_err(|e| e.to_string())
        }

        fn x11_modifiers(&self, logical: u8) -> Option<u16> {
            if (logical & ALT != 0 && self.alt_mask.count_ones() != 1)
                || (logical & SUPER != 0 && self.super_mask.count_ones() != 1)
            {
                return None;
            }
            Some(
                if logical & CTRL != 0 {
                    u16::from(ModMask::CONTROL)
                } else {
                    0
                } | if logical & SHIFT != 0 {
                    u16::from(ModMask::SHIFT)
                } else {
                    0
                } | if logical & ALT != 0 { self.alt_mask } else { 0 }
                    | if logical & SUPER != 0 {
                        self.super_mask
                    } else {
                        0
                    },
            )
        }

        fn logical_modifiers(&self, state: u16) -> u8 {
            let state = state & 0xff & !self.lock_mask;
            let known = u16::from(ModMask::CONTROL)
                | u16::from(ModMask::SHIFT)
                | self.alt_mask
                | self.super_mask;
            if state & !known != 0 {
                return u8::MAX;
            }
            (if state & u16::from(ModMask::CONTROL) != 0 {
                CTRL
            } else {
                0
            }) | (if state & u16::from(ModMask::SHIFT) != 0 {
                SHIFT
            } else {
                0
            }) | (if state & self.alt_mask != 0 { ALT } else { 0 })
                | (if state & self.super_mask != 0 {
                    SUPER
                } else {
                    0
                })
        }

        fn forward(&self, mut event: KeyPressEvent, release: bool) -> Result<(), String> {
            let Some(focus) = self.original_focus.filter(|focus| *focus > 1) else {
                return Ok(());
            };
            let current = self
                .connection
                .get_input_focus()
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?
                .focus;
            if current != focus {
                return Err("External focus changed while a shortcut key was held".into());
            }
            event.event = focus;
            event.child = 0;
            self.connection
                .send_event(
                    false,
                    focus,
                    if release {
                        EventMask::KEY_RELEASE
                    } else {
                        EventMask::KEY_PRESS
                    },
                    event,
                )
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
            self.connection.flush().map_err(|e| e.to_string())
        }

        fn poll_events(&mut self) -> Result<Vec<X11HotkeyActivation>, String> {
            let now = self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            let mut actions = Vec::new();
            while let Some(event) = self
                .connection
                .poll_for_event()
                .map_err(|e| e.to_string())?
            {
                match event {
                    Event::KeyPress(event) => {
                        let label = self.labels.get(&event.detail).cloned().unwrap_or_default();
                        let modifiers = self.logical_modifiers(u16::from(event.state));
                        let decision = self.cycles.press(
                            &self.registered,
                            event.detail,
                            &label,
                            modifiers,
                            event.time,
                        );
                        if let Some(action) = decision.action {
                            let focus = self
                                .connection
                                .get_input_focus()
                                .map_err(|e| e.to_string())?
                                .reply()
                                .map_err(|e| e.to_string())?
                                .focus;
                            if self.original_focus.is_none() {
                                self.original_focus = (focus > 1).then_some(focus);
                            }
                            if self.original_focus == Some(focus) && !self.cycle_cancelled {
                                let target = crate::linux_app::capture_linux_native_vv_target();
                                let after = self
                                    .connection
                                    .get_input_focus()
                                    .map_err(|e| e.to_string())?
                                    .reply()
                                    .map_err(|e| e.to_string())?
                                    .focus;
                                if after == focus {
                                    self.pending.push(X11HotkeyActivation { action, target });
                                } else {
                                    self.pending.clear();
                                    self.cycle_cancelled = true;
                                }
                            } else {
                                self.pending.clear();
                                self.cycle_cancelled = true;
                            }
                        } else if !decision.consume {
                            // Core passive grabs route other keys here until
                            // the trigger is released. Keep the old focus until
                            // then; a delivery failure cancels the queued action.
                            if let Err(error) = self.forward(event, false) {
                                self.pending.clear();
                                self.cycle_cancelled = true;
                                eprintln!("ZSClip X11 shortcut key forwarding failed: {error}");
                            }
                        }
                    }
                    Event::KeyRelease(event) => {
                        if self.cycles.release(event.detail, event.time, now) {
                            // Detectable autorepeat guarantees this is a real
                            // release; the implicit server grab ends here.
                            self.cycles.releases.remove(&event.detail);
                            self.cycles.finish_release(event.detail);
                        } else if let Err(error) = self.forward(event, true) {
                            self.pending.clear();
                            self.cycle_cancelled = true;
                            eprintln!("ZSClip X11 shortcut key forwarding failed: {error}");
                        }
                    }
                    Event::MappingNotify(_) => self.map_changed = true,
                    _ => {}
                }
            }
            if self.original_focus.is_some() {
                let current_focus = self
                    .connection
                    .get_input_focus()
                    .map_err(|e| e.to_string())?
                    .reply()
                    .map_err(|e| e.to_string())?
                    .focus;
                if self.original_focus != Some(current_focus) {
                    self.pending.clear();
                    self.cycle_cancelled = true;
                }
                if self.cycles.held.is_empty() && !self.cycle_cancelled {
                    actions.append(&mut self.pending);
                } else if self.cycle_cancelled {
                    self.pending.clear();
                }
            }
            if self.cycles.held.is_empty() {
                self.pending.clear();
                self.original_focus = None;
                self.cycle_cancelled = false;
            }
            Ok(actions)
        }

        pub(crate) fn poll(
            &mut self,
        ) -> Result<(Vec<X11HotkeyActivation>, Option<X11HotkeyRegistration>), String> {
            let mut actions = self.poll_events()?;
            let report = if self.map_changed {
                actions.clear();
                Some(self.replace_bindings(self.desired.clone()))
            } else {
                None
            };
            Ok((actions, report))
        }
    }

    impl Drop for X11HotkeyRegistry {
        fn drop(&mut self) {
            let _ = self.remove_passive_grabs();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::process::Command;
        use std::thread;
        use std::time::Duration;
        use x11rb::protocol::xproto::{CreateWindowAux, InputFocus, WindowClass};

        fn inject(arguments: &[&str]) {
            let output = Command::new("xdotool").args(arguments).output().unwrap();
            assert!(
                output.status.success(),
                "xdotool: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        fn pump(registry: &mut X11HotkeyRegistry, milliseconds: u64) -> Vec<X11HotkeyActivation> {
            let deadline = Instant::now() + Duration::from_millis(milliseconds);
            let mut actions = Vec::new();
            while Instant::now() < deadline {
                let (next, report) = registry.poll().unwrap();
                if let Some(report) = report {
                    assert!(report.errors.is_empty(), "{:?}", report.errors);
                }
                actions.extend(next);
                thread::sleep(Duration::from_millis(5));
            }
            actions
        }

        fn received_keys(connection: &RustConnection) -> Vec<(u8, bool)> {
            let mut result = Vec::new();
            while let Some(event) = connection.poll_for_event().unwrap() {
                match event {
                    Event::KeyPress(event) => result.push((event.detail, true)),
                    Event::KeyRelease(event) => result.push((event.detail, false)),
                    _ => {}
                }
            }
            result
        }

        /// Requires a real X11 test server and xdotool; default unit test runs
        /// deliberately do not treat this as a headless behavioral proof.
        #[test]
        #[ignore = "requires an X11 desktop and xdotool"]
        fn x11_server_preserves_owned_release_during_rebind_and_reports_conflicts() {
            let (receiver, screen) = x11rb::connect(None).unwrap();
            let root = receiver.setup().roots[screen].root;
            let window = receiver.generate_id().unwrap();
            receiver
                .create_window(
                    x11rb::COPY_DEPTH_FROM_PARENT,
                    window,
                    root,
                    20,
                    20,
                    300,
                    100,
                    0,
                    WindowClass::INPUT_OUTPUT,
                    0,
                    &CreateWindowAux::new()
                        .event_mask(EventMask::KEY_PRESS | EventMask::KEY_RELEASE),
                )
                .unwrap()
                .check()
                .unwrap();
            receiver.map_window(window).unwrap().check().unwrap();
            receiver
                .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
                .unwrap()
                .check()
                .unwrap();
            receiver.flush().unwrap();
            inject(&["windowfocus", "--sync", &window.to_string()]);
            let mut registry = X11HotkeyRegistry::connect().unwrap();
            let initial = NativeHotkeyBindings::from_settings(
                &serde_json::json!({"hotkey_mod":"Ctrl+Alt","hotkey_key":"V"}),
                false,
            );
            let report = registry.replace_bindings(initial.clone());
            assert_eq!(report.registered, 1, "{:?}", report.errors);
            assert!(report.errors.is_empty(), "{:?}", report.errors);
            let code_v = *registry
                .labels
                .iter()
                .find(|(_, label)| label.as_str() == "V")
                .unwrap()
                .0;

            inject(&["keydown", "Control_L", "Alt_L", "v"]);
            assert!(pump(&mut registry, 100).is_empty());
            assert!(registry.cycles.held.contains(&code_v));
            let disabled = NativeHotkeyBindings::from_settings(
                &serde_json::json!({"hotkey_enabled":false}),
                false,
            );
            assert!(registry.replace_bindings(disabled).errors.is_empty());
            assert!(pump(&mut registry, 650).is_empty());
            inject(&["keyup", "v", "Alt_L", "Control_L"]);
            assert!(pump(&mut registry, 80).is_empty());
            assert!(registry.cycles.held.is_empty());
            assert!(
                !received_keys(&receiver)
                    .iter()
                    .any(|(code, _)| *code == code_v),
                "owned V down/repeat/up leaked during disable"
            );

            inject(&["key", "ctrl+alt+v"]);
            assert!(pump(&mut registry, 50).is_empty());
            let received = received_keys(&receiver);
            assert!(
                received.contains(&(code_v, true)) && received.contains(&(code_v, false)),
                "disabled shortcut did not return both phases to the receiver: {received:?}"
            );

            let rebound = NativeHotkeyBindings::from_settings(
                &serde_json::json!({"hotkey_mod":"Ctrl+Alt","hotkey_key":"B"}),
                false,
            );
            assert_eq!(registry.replace_bindings(rebound.clone()).registered, 1);
            let code_b = *registry
                .labels
                .iter()
                .find(|(_, label)| label.as_str() == "B")
                .unwrap()
                .0;
            let mut contender = X11HotkeyRegistry::connect().unwrap();
            let conflict = contender.replace_bindings(rebound);
            assert_eq!(conflict.registered, 0);
            assert!(
                !conflict.errors.is_empty(),
                "an already owned shortcut must report BadAccess"
            );
            inject(&["keydown", "Control_L", "Alt_L", "b"]);
            assert!(pump(&mut registry, 750).is_empty());
            inject(&["keyup", "b", "Alt_L", "Control_L"]);
            let actions = pump(&mut registry, 80);
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0].action, NativeHotkeyAction::ShowMain);
            assert!(!received_keys(&receiver)
                .iter()
                .any(|(code, _)| *code == code_b));
            assert!(registry.replace_bindings(initial).errors.is_empty());
            inject(&["key", "ctrl+alt+b"]);
            assert!(pump(&mut registry, 50).is_empty());
            let received = received_keys(&receiver);
            assert!(received.contains(&(code_b, true)) && received.contains(&(code_b, false)));
            inject(&["keydown", "Control_L", "Alt_L", "v"]);
            assert!(pump(&mut registry, 60).is_empty());
            receiver
                .set_input_focus(InputFocus::PARENT, root, x11rb::CURRENT_TIME)
                .unwrap()
                .check()
                .unwrap();
            assert!(pump(&mut registry, 30).is_empty());
            receiver
                .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
                .unwrap()
                .check()
                .unwrap();
            assert!(pump(&mut registry, 30).is_empty());
            inject(&["keyup", "v", "Alt_L", "Control_L"]);
            assert!(
                pump(&mut registry, 60).is_empty(),
                "switching focus away and back must cancel the pending shortcut"
            );
            receiver.destroy_window(window).unwrap().check().unwrap();
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) use platform::enable_x11_detectable_autorepeat;
#[cfg(target_os = "linux")]
pub(crate) use platform::X11HotkeyRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    fn bindings() -> NativeHotkeyBindings {
        NativeHotkeyBindings::from_settings(&serde_json::json!({}), false)
    }
    #[test]
    fn repeat_does_not_reopen_and_rebinding_keeps_owned_release() {
        let mut cycles = HotkeyKeyCycles::default();
        let initial = bindings();
        assert_eq!(
            cycles
                .press(&initial, 55, "V", crate::native_hotkey::SUPER, 1)
                .action,
            Some(NativeHotkeyAction::ShowMain)
        );
        assert!(cycles.release(55, 20, 20));
        let disabled = NativeHotkeyBindings::default();
        let repeat = cycles.press(&disabled, 55, "V", 0, 20);
        assert!(repeat.consume);
        assert!(repeat.action.is_none());
        assert!(cycles.release(55, 30, 30));
        cycles.finish_releases(41);
        assert!(cycles.held.contains(&55));
        cycles.finish_releases(42);
        assert!(!cycles.held.contains(&55));
        assert!(!cycles.press(&disabled, 55, "V", 0, 40).consume);
    }
    #[test]
    fn distinct_press_after_fast_release_is_a_new_cycle() {
        let mut cycles = HotkeyKeyCycles::default();
        let bindings = bindings();
        cycles.press(&bindings, 55, "V", crate::native_hotkey::SUPER, 1);
        cycles.release(55, 2, 2);
        assert!(cycles
            .press(&bindings, 55, "V", crate::native_hotkey::SUPER, 3)
            .action
            .is_some());
        assert!(!cycles.release(37, 4, 4));
    }
    #[test]
    fn x11_has_a_symbol_for_every_configurable_key() {
        for key in crate::settings_model::HOTKEY_KEY_OPTIONS {
            assert!(key_symbol(key).is_some(), "{key}");
        }
    }
}
