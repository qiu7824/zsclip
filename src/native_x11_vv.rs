use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum X11VvAction {
    Select(usize),
    Cancel,
    Previous,
    Next,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct X11ObservedKey {
    pub keysym: u32,
    pub modifiers: u16,
    pub focus: u32,
    pub time: u32,
}

impl X11VvAction {
    fn repeats(self) -> bool {
        matches!(
            self,
            Self::Previous | Self::Next | Self::PageUp | Self::PageDown
        )
    }
}

#[derive(Default)]
struct OwnedKeyCycles {
    held: HashSet<u8>,
    releases: HashMap<u8, (u32, u64)>,
    primary: Option<(u8, X11VvAction)>,
}

impl OwnedKeyCycles {
    fn press(&mut self, code: u8, action: X11VvAction, time: u32) -> Vec<X11VvAction> {
        if self
            .releases
            .get(&code)
            .is_some_and(|release| release.0 == time)
        {
            self.releases.remove(&code);
        }
        let repeated = !self.held.insert(code);
        if action == X11VvAction::Cancel && !repeated {
            self.primary = None;
            return vec![X11VvAction::Cancel];
        }
        if action.repeats() {
            return vec![action];
        }
        if !repeated && self.primary.is_none() {
            self.primary = Some((code, action));
        }
        Vec::new()
    }
    fn release(&mut self, code: u8, time: u32, now: u64) {
        if self.held.contains(&code) {
            self.releases.insert(code, (time, now));
        }
    }
    fn finish_releases(&mut self, now: u64) -> Vec<X11VvAction> {
        let complete = self
            .releases
            .iter()
            .filter(|(_, (_, seen))| now.saturating_sub(*seen) >= 12)
            .map(|(&code, _)| code)
            .collect::<Vec<_>>();
        let mut actions = Vec::new();
        for code in complete {
            self.releases.remove(&code);
            self.held.remove(&code);
            if self.primary.is_some_and(|primary| primary.0 == code) {
                actions.push(self.primary.take().unwrap().1);
            }
        }
        actions
    }
    fn cancel(&mut self) {
        self.primary = None;
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::time::Instant;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask, GrabMode, KeyPressEvent,
        ModMask, PropMode,
    };
    use x11rb::protocol::Event;
    use x11rb::rust_connection::RustConnection;
    use x11rb::wrapper::ConnectionExt as _;

    pub(crate) fn observe_x11_keys(
        sender: std::sync::mpsc::Sender<X11ObservedKey>,
    ) -> Result<(), String> {
        use x11rb::protocol::record::{ConnectionExt as _, Range, Range8, CS};
        use x11rb::x11_utils::TryParse;
        let (control, _) = x11rb::connect(None).map_err(|error| error.to_string())?;
        let (data, _) = x11rb::connect(None).map_err(|error| error.to_string())?;
        control
            .record_query_version(1, 13)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        let min = control.setup().min_keycode;
        let mapping = control
            .get_keyboard_mapping(min, control.setup().max_keycode - min + 1)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        if mapping.keysyms_per_keycode == 0 {
            return Err("X11 keyboard map is empty".into());
        }
        let context = control.generate_id().map_err(|error| error.to_string())?;
        let ranges = [Range {
            device_events: Range8 { first: 2, last: 3 },
            ..Range::default()
        }];
        control
            .record_create_context(context, 0u8.into(), &[u32::from(CS::ALL_CLIENTS)], &ranges)
            .map_err(|error| error.to_string())?
            .check()
            .map_err(|error| error.to_string())?;
        control.flush().map_err(|error| error.to_string())?;
        let replies = data
            .record_enable_context(context)
            .map_err(|error| error.to_string())?;
        data.flush().map_err(|error| error.to_string())?;
        let mut last_release = None;
        for reply in replies {
            let reply = reply.map_err(|error| error.to_string())?;
            if reply.category == 4 {
                eprintln!("ZSClip GTK X11 VV observer installed");
            }
            if reply.category != 0 || reply.client_swapped {
                continue;
            }
            for bytes in reply.data.chunks_exact(32) {
                let (event, _) =
                    KeyPressEvent::try_parse(bytes).map_err(|error| error.to_string())?;
                if event.response_type & 0x7f == 3 {
                    last_release = Some((event.detail, event.time));
                    continue;
                }
                if event.response_type & 0x7f != 2
                    || last_release == Some((event.detail, event.time))
                {
                    continue;
                }
                let offset = usize::from(event.detail.saturating_sub(min))
                    * usize::from(mapping.keysyms_per_keycode);
                let keysym = mapping.keysyms.get(offset).copied().unwrap_or(0);
                let focus = control
                    .get_input_focus()
                    .map_err(|error| error.to_string())?
                    .reply()
                    .map_err(|error| error.to_string())?
                    .focus;
                if sender
                    .send(X11ObservedKey {
                        keysym,
                        modifiers: u16::from(event.state),
                        focus,
                        time: event.time,
                    })
                    .is_err()
                {
                    let _ = control.record_disable_context(context);
                    let _ = control.flush();
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub(crate) struct X11VvLease {
        connection: RustConnection,
        root: u32,
        original_focus: u32,
        bindings: HashMap<u8, X11VvAction>,
        grabs: Vec<(u8, u16)>,
        lock_mask: u16,
        cancelled: Cell<bool>,
        cycles: RefCell<OwnedKeyCycles>,
        started: Instant,
    }

    impl X11VvLease {
        pub(crate) fn acquire_before_map(popup: u32) -> Result<Self, String> {
            let (connection, screen) = x11rb::connect(None).map_err(|error| error.to_string())?;
            let root = connection.setup().roots[screen].root;
            let focus = connection
                .get_input_focus()
                .map_err(|error| error.to_string())?
                .reply()
                .map_err(|error| error.to_string())?
                .focus;
            if focus <= 1 {
                return Err("X11 has no focused editor".into());
            }
            let min = connection.setup().min_keycode;
            let count = connection.setup().max_keycode - min + 1;
            let mapping = connection
                .get_keyboard_mapping(min, count)
                .map_err(|error| error.to_string())?
                .reply()
                .map_err(|error| error.to_string())?;
            let keycode = |symbol: u32| {
                mapping
                    .keysyms
                    .chunks(mapping.keysyms_per_keycode as usize)
                    .position(|symbols| symbols.first() == Some(&symbol))
                    .map(|index| min + index as u8)
            };
            let mut bindings = HashMap::new();
            for (symbol, action) in (0..9)
                .map(|index| (u32::from(b'1') + index as u32, X11VvAction::Select(index)))
                .chain([
                    (0xff1b, X11VvAction::Cancel),
                    (0xff52, X11VvAction::Previous),
                    (0xff54, X11VvAction::Next),
                    (0xff55, X11VvAction::PageUp),
                    (0xff56, X11VvAction::PageDown),
                ])
            {
                let code = keycode(symbol)
                    .ok_or_else(|| "Required unmodified X11 key is unavailable".to_string())?;
                bindings.insert(code, action);
            }
            let modifier_map = connection
                .get_modifier_mapping()
                .map_err(|error| error.to_string())?
                .reply()
                .map_err(|error| error.to_string())?;
            let mut lock_mask = u16::from(ModMask::LOCK);
            if let Some(numlock) = keycode(0xff7f) {
                for (index, codes) in modifier_map
                    .keycodes
                    .chunks(modifier_map.keycodes_per_modifier().max(1) as usize)
                    .enumerate()
                {
                    if codes.contains(&numlock) {
                        lock_mask |= 1 << index;
                    }
                }
            }
            let mut modifiers = vec![
                0,
                u16::from(ModMask::LOCK),
                lock_mask,
                lock_mask & !u16::from(ModMask::LOCK),
            ];
            modifiers.sort_unstable();
            modifiers.dedup();
            let mut lease = Self {
                connection,
                root,
                original_focus: focus,
                bindings,
                grabs: Vec::new(),
                lock_mask,
                cancelled: Cell::new(false),
                cycles: RefCell::new(OwnedKeyCycles::default()),
                started: Instant::now(),
            };
            for (&code, _) in &lease.bindings {
                for &modifiers in &modifiers {
                    lease
                        .connection
                        .grab_key(
                            false,
                            root,
                            ModMask::from(modifiers),
                            code,
                            GrabMode::ASYNC,
                            GrabMode::ASYNC,
                        )
                        .map_err(|error| error.to_string())?
                        .check()
                        .map_err(|error| format!("X11 VV key ownership unavailable: {error}"))?;
                    lease.grabs.push((code, modifiers));
                }
            }
            // Configure the unmapped surface before GTK can expose it. It does
            // not ask the window manager to activate or focus a toplevel.
            lease
                .connection
                .change_window_attributes(
                    popup,
                    &ChangeWindowAttributesAux::new().override_redirect(1u32),
                )
                .map_err(|error| error.to_string())?
                .check()
                .map_err(|error| error.to_string())?;
            lease
                .connection
                .change_property32(
                    PropMode::REPLACE,
                    popup,
                    AtomEnum::WM_HINTS,
                    AtomEnum::WM_HINTS,
                    &[1, 0, 1, 0, 0, 0, 0, 0, 0],
                )
                .map_err(|error| error.to_string())?
                .check()
                .map_err(|error| error.to_string())?;
            lease
                .connection
                .flush()
                .map_err(|error| error.to_string())?;
            Ok(lease)
        }

        pub(crate) fn cancel(&self) {
            if self.cancelled.replace(true) {
                return;
            }
            self.cycles.borrow_mut().cancel();
            for &(code, modifiers) in &self.grabs {
                let _ = self
                    .connection
                    .ungrab_key(code, self.root, ModMask::from(modifiers));
            }
            let _ = self.connection.flush();
            // Keep this connection alive until already owned key releases have
            // been read, even after the popup is hidden.
            let _ = self
                .connection
                .get_input_focus()
                .ok()
                .and_then(|cookie| cookie.reply().ok());
        }

        pub(crate) fn has_owned_keys(&self) -> bool {
            !self.cycles.borrow().held.is_empty()
        }

        pub(crate) fn focus_is_current(&self) -> bool {
            self.connection
                .get_input_focus()
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .is_some_and(|reply| reply.focus == self.original_focus)
        }

        fn forward_ordinary_key(
            &self,
            mut event: KeyPressEvent,
            release: bool,
        ) -> Result<(), String> {
            event.event = self.original_focus;
            event.child = 0;
            let mask = if release {
                EventMask::KEY_RELEASE
            } else {
                EventMask::KEY_PRESS
            };
            self.connection
                .send_event(false, self.original_focus, mask, event)
                .map_err(|error| error.to_string())?
                .check()
                .map_err(|error| error.to_string())?;
            self.connection.flush().map_err(|error| error.to_string())
        }

        pub(crate) fn poll(&self) -> Result<Vec<X11VvAction>, String> {
            let mut actions = Vec::new();
            let now = self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            while let Some(event) = self
                .connection
                .poll_for_event()
                .map_err(|error| error.to_string())?
            {
                match event {
                    Event::KeyPress(event) => {
                        if let Some(&action) = self.bindings.get(&event.detail) {
                            let held = self.cycles.borrow().held.contains(&event.detail);
                            let modifiers = u16::from(event.state) & 0xff & !self.lock_mask;
                            // A KeyPress can already be queued when cancellation
                            // removes the passive grabs. Retain its full cycle so
                            // closing the connection cannot leak its keyup.
                            if held || modifiers == 0 {
                                let next = self.cycles.borrow_mut().press(
                                    event.detail,
                                    action,
                                    event.time,
                                );
                                if self.cancelled.get() {
                                    self.cycles.borrow_mut().cancel();
                                } else {
                                    actions.extend(next);
                                }
                            }
                        } else {
                            self.forward_ordinary_key(event, false)?;
                            self.cancel();
                            actions.push(X11VvAction::Cancel);
                        }
                    }
                    Event::KeyRelease(event) => {
                        if self.cycles.borrow().held.contains(&event.detail) {
                            self.cycles
                                .borrow_mut()
                                .release(event.detail, event.time, now);
                        } else if !self.bindings.contains_key(&event.detail) {
                            self.forward_ordinary_key(event, true)?;
                        }
                    }
                    _ => {}
                }
            }
            let released = self.cycles.borrow_mut().finish_releases(now);
            if !self.cancelled.get() {
                actions.extend(released);
            }
            Ok(actions)
        }
    }

    impl Drop for X11VvLease {
        fn drop(&mut self) {
            self.cancel();
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) use platform::observe_x11_keys;
#[cfg(target_os = "linux")]
pub(crate) use platform::X11VvLease;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn x11_vv_consumes_repeats_until_the_real_escape_release() {
        let mut keys = OwnedKeyCycles::default();
        assert_eq!(keys.press(9, X11VvAction::Cancel, 1), [X11VvAction::Cancel]);
        keys.release(9, 20, 20);
        assert!(keys.press(9, X11VvAction::Cancel, 20).is_empty());
        assert!(keys.finish_releases(50).is_empty());
        keys.release(9, 60, 60);
        assert!(keys.finish_releases(65).is_empty());
        assert!(keys.finish_releases(72).is_empty());
        assert!(keys.held.is_empty());
        assert!(keys.finish_releases(90).is_empty());
    }
    #[test]
    fn cancelling_x11_vv_suppresses_selection_but_keeps_key_release_ownership() {
        let mut keys = OwnedKeyCycles::default();
        keys.press(10, X11VvAction::Select(0), 1);
        keys.cancel();
        assert!(keys.held.contains(&10));
        keys.release(10, 2, 2);
        assert!(keys.finish_releases(20).is_empty());
        assert!(keys.held.is_empty());
    }
}
