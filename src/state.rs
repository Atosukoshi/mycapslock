use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Mutex;
use std::time::Instant;

use crate::actions::{Action, MappingTable, ModifierFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Idle,
    Pressed,
    Held,
}

pub struct CapsLockState {
    state: State,
    press_time: Option<Instant>,
    other_key_pressed: bool,
    /// CapsLock system state (0 or 1) BEFORE this CapsLock press started.
    /// Used on release to detect if the driver toggled despite our block.
    capslock_pre_press: i32,
}

impl CapsLockState {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            press_time: None,
            other_key_pressed: false,
            capslock_pre_press: 0,
        }
    }

    pub fn on_capslock_down(&mut self, capslock_now: i32) -> HookEvent {
        self.state = State::Pressed;
        self.press_time = Some(Instant::now());
        self.other_key_pressed = false;
        self.capslock_pre_press = capslock_now;
        HookEvent::Block
    }

    pub fn on_capslock_up(
        &mut self,
        hold_threshold_ms: u64,
        tap_to_toggle: bool,
        capslock_now: i32,
    ) -> HookEvent {
        let modifier_used = self.other_key_pressed;
        let duration = self.press_time.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
        let driver_toggled = capslock_now != self.capslock_pre_press;
        let is_tap = !modifier_used && duration < hold_threshold_ms;

        let result = if modifier_used {
            if driver_toggled { HookEvent::ToggleCapsLock } else { HookEvent::Block }
        } else if is_tap && tap_to_toggle {
            if driver_toggled { HookEvent::Block } else { HookEvent::ToggleCapsLock }
        } else if is_tap && !tap_to_toggle {
            if driver_toggled { HookEvent::ToggleCapsLock } else { HookEvent::Block }
        } else {
            // Long hold without modifier
            if driver_toggled { HookEvent::ToggleCapsLock } else { HookEvent::Block }
        };

        self.state = State::Idle;
        self.press_time = None;
        self.other_key_pressed = false;
        result
    }

    pub fn on_other_key_down(&mut self) -> Option<State> {
        match self.state {
            State::Pressed => {
                self.state = State::Held;
                self.other_key_pressed = true;
                Some(State::Held)
            }
            State::Held => {
                self.other_key_pressed = true;
                Some(State::Held)
            }
            State::Idle => None,
        }
    }

}

#[derive(Debug, Clone, PartialEq)]
pub enum HookEvent {
    Action(Action),
    ToggleCapsLock,
    PassThrough,
    Block,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, MappingTable};
    use std::collections::HashMap;

    // Helper: caps=0 means CapsLock OFF in system, caps=1 means ON
    const OFF: i32 = 0;
    const ON: i32 = 1;

    #[test]
    fn test_capslock_tap_to_toggle_enabled_driver_no_toggle() {
        // LRESULT(1) successfully prevented driver toggle
        let mut st = CapsLockState::new();
        assert_eq!(st.on_capslock_down(OFF), HookEvent::Block);
        std::thread::sleep(std::time::Duration::from_millis(50));
        // CapsLock still OFF → driver didn't toggle → we must toggle
        assert_eq!(st.on_capslock_up(200, true, OFF), HookEvent::ToggleCapsLock);
    }

    #[test]
    fn test_capslock_tap_to_toggle_enabled_driver_did_toggle() {
        // Driver toggled despite LRESULT(1) — don't double-toggle
        let mut st = CapsLockState::new();
        assert_eq!(st.on_capslock_down(OFF), HookEvent::Block);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(st.on_capslock_up(200, true, ON), HookEvent::Block);
    }

    #[test]
    fn test_capslock_tap_to_toggle_disabled() {
        let mut st = CapsLockState::new();
        assert_eq!(st.on_capslock_down(OFF), HookEvent::Block);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(st.on_capslock_up(200, false, OFF), HookEvent::Block);
    }

    #[test]
    fn test_capslock_tap_to_toggle_disabled_driver_toggled() {
        let mut st = CapsLockState::new();
        assert_eq!(st.on_capslock_down(OFF), HookEvent::Block);
        std::thread::sleep(std::time::Duration::from_millis(50));
        // tap_to_toggle=false, driver toggled → correct back
        assert_eq!(st.on_capslock_up(200, false, ON), HookEvent::ToggleCapsLock);
    }

    #[test]
    fn test_capslock_long_hold_no_toggle() {
        let mut st = CapsLockState::new();
        assert_eq!(st.on_capslock_down(OFF), HookEvent::Block);
        std::thread::sleep(std::time::Duration::from_millis(250));
        // Long hold (>200ms) without other key → no toggle
        assert_eq!(st.on_capslock_up(200, true, OFF), HookEvent::Block);
    }

    #[test]
    fn test_capslock_long_hold_driver_toggled() {
        let mut st = CapsLockState::new();
        assert_eq!(st.on_capslock_down(OFF), HookEvent::Block);
        std::thread::sleep(std::time::Duration::from_millis(250));
        // Long hold, driver toggled → correct back
        assert_eq!(st.on_capslock_up(200, true, ON), HookEvent::ToggleCapsLock);
    }

    #[test]
    fn test_capslock_held_with_other_key_mapped() {
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let mut cursor = HashMap::new();
        cursor.insert("e".to_string(), "Up".to_string());
        let mapping = MappingTable::from_cursor_mappings(&cursor);
        let ctx = HookContext::new(tx, mapping, 200, true);

        // CapsLock down → blocked (driver didn't toggle)
        assert_eq!(ctx.process_key(0x14, true, OFF, 0), HookEvent::Block);
        // Press E → remapped
        assert_eq!(
            ctx.process_key(0x45, true, OFF, 0),
            HookEvent::Action(Action::KeyPress(enigo::Key::UpArrow))
        );
        // CapsLock up → modifier used, driver didn't toggle → no toggle
        assert_eq!(ctx.process_key(0x14, false, OFF, 0), HookEvent::Block);
    }

    #[test]
    fn test_capslock_modifier_driver_toggled() {
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let mut cursor = HashMap::new();
        cursor.insert("e".to_string(), "Up".to_string());
        let mapping = MappingTable::from_cursor_mappings(&cursor);
        let ctx = HookContext::new(tx, mapping, 200, true);

        ctx.process_key(0x14, true, OFF, 0);  // down
        ctx.process_key(0x45, true, ON, 0);   // E, driver toggled in between
        // CapsLock up → modifier used + driver toggled → correct
        assert_eq!(ctx.process_key(0x14, false, ON, 0), HookEvent::ToggleCapsLock);
    }

    #[test]
    fn test_capslock_held_with_unmapped_key() {
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let mapping = MappingTable::from_cursor_mappings(&HashMap::new());
        let ctx = HookContext::new(tx, mapping, 200, true);

        ctx.process_key(0x14, true, OFF, 0);
        let event = ctx.process_key(0x5A, true, OFF, 0);
        assert_eq!(event, HookEvent::Block);
    }

    #[test]
    fn test_normal_key_passes_through() {
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let mapping = MappingTable::from_cursor_mappings(&HashMap::new());
        let ctx = HookContext::new(tx, mapping, 200, true);

        assert_eq!(ctx.process_key(0x41, true, OFF, 0), HookEvent::PassThrough);
        assert_eq!(ctx.process_key(0x41, false, OFF, 0), HookEvent::PassThrough);
    }

    #[test]
    fn test_state_transitions() {
        let mut st = CapsLockState::new();

        st.on_capslock_down(OFF);
        assert_eq!(st.on_other_key_down(), Some(State::Held));
        assert_eq!(st.on_other_key_down(), Some(State::Held));
        // modifier + no driver toggle → Block
        assert_eq!(st.on_capslock_up(200, true, OFF), HookEvent::Block);
    }

    #[test]
    fn test_normal_key_when_idle_does_nothing() {
        let mut st = CapsLockState::new();
        assert_eq!(st.on_other_key_down(), None);
    }

    #[test]
    fn test_backspace_as_trigger_key() {
        let mut cursor = HashMap::new();
        cursor.insert("backspace".into(), "Home, Shift+End, Delete".into());
        let mapping = MappingTable::from_cursor_mappings(&cursor);
        assert!(mapping.lookup(0x08, 0).is_some(), "VK_BACK (0x08) mapping should exist");
    }
}

pub struct HookContext {
    pub state: Mutex<CapsLockState>,
    pub mapping: Mutex<MappingTable>,
    pub sender: Mutex<Option<SyncSender<HookEvent>>>,
    pub hold_threshold_ms: AtomicU64,
    pub tap_to_toggle: AtomicBool,
    pub modifiers: AtomicU32,
}

impl HookContext {
    fn track_modifier(&self, vk_code: u32, is_key_down: bool) {
        use crate::actions::{MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT};
        let flag = match vk_code {
            0x10 => Some(MOD_SHIFT),                    // VK_SHIFT
            0x11 => Some(MOD_CTRL),                     // VK_CONTROL
            0x12 | 0xA4 | 0xA5 => Some(MOD_ALT),       // VK_MENU / VK_LMENU / VK_RMENU
            0x5B | 0x5C => Some(MOD_META),              // VK_LWIN / VK_RWIN
            _ => None,
        };
        if let Some(f) = flag {
            if is_key_down {
                self.modifiers.fetch_or(f, Ordering::Relaxed);
            } else {
                self.modifiers.fetch_and(!f, Ordering::Relaxed);
            }
        }
    }

    pub fn update_settings(&self, hold_threshold_ms: u64, tap_to_toggle: bool, mapping: MappingTable) {
        self.hold_threshold_ms.store(hold_threshold_ms, Ordering::Relaxed);
        self.tap_to_toggle.store(tap_to_toggle, Ordering::Relaxed);
        if let Ok(mut m) = self.mapping.lock() {
            *m = mapping;
        }
    }
}

impl HookContext {
    pub fn new(
        sender: SyncSender<HookEvent>,
        mapping: MappingTable,
        hold_threshold_ms: u64,
        tap_to_toggle: bool,
    ) -> Self {
        Self {
            state: Mutex::new(CapsLockState::new()),
            mapping: Mutex::new(mapping),
            sender: Mutex::new(Some(sender)),
            hold_threshold_ms: AtomicU64::new(hold_threshold_ms),
            tap_to_toggle: AtomicBool::new(tap_to_toggle),
            modifiers: AtomicU32::new(0),
        }
    }

    pub fn process_key(&self, vk_code: u32, is_key_down: bool, capslock_now: i32, _mod_flags: ModifierFlags) -> HookEvent {
        const VK_CAPITAL: u32 = 0x14;

        // Track physical modifier key state (more reliable than GetAsyncKeyState in hook callback)
        self.track_modifier(vk_code, is_key_down);

        if vk_code == VK_CAPITAL {
            let mut st = self.state.lock().unwrap();
            if is_key_down {
                st.on_capslock_down(capslock_now)
            } else {
                st.on_capslock_up(
                    self.hold_threshold_ms.load(Ordering::Relaxed),
                    self.tap_to_toggle.load(Ordering::Relaxed),
                    capslock_now,
                )
            }
        } else if is_key_down {
            let mut st = self.state.lock().unwrap();
            if st.on_other_key_down().is_some() {
                let mapping = self.mapping.lock().unwrap();
                let tracked_mods = self.modifiers.load(Ordering::Relaxed);
                if let Some(action) = mapping.lookup(vk_code, tracked_mods) {
                    return HookEvent::Action(action);
                }
                return HookEvent::Block;
            }
            HookEvent::PassThrough
        } else {
            HookEvent::PassThrough
        }
    }

    pub fn send_event(&self, event: HookEvent) {
        match &event {
            HookEvent::Action(_) | HookEvent::ToggleCapsLock => {
                if let Ok(sender) = self.sender.lock() {
                    if let Some(ref tx) = *sender {
                        let _ = tx.send(event);
                    }
                }
            }
            HookEvent::Block | HookEvent::PassThrough => {}
        }
    }
}

