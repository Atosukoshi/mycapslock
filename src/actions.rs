use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::collections::HashMap;

// ---- Modifier flags for composite key lookup ----
pub type ModifierFlags = u32;
pub const MOD_NONE: ModifierFlags = 0;
pub const MOD_SHIFT: ModifierFlags = 1;
pub const MOD_CTRL: ModifierFlags = 2;
pub const MOD_ALT: ModifierFlags = 4;
pub const MOD_META: ModifierFlags = 8;

// ---- Action types ----

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    KeyPress(Key),
    KeyChord(Vec<Key>),
    Sequence(Vec<Action>),
}

// ---- Key name resolution ----

pub fn str_to_key(s: &str) -> Option<Key> {
    match s.to_lowercase().as_str() {
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "delete" => Some(Key::Delete),
        "escape" | "esc" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "tab" => Some(Key::Tab),
        "enter" | "return" => Some(Key::Return),
        "space" => Some(Key::Space),
        "control" | "ctrl" => Some(Key::Control),
        "alt" => Some(Key::Alt),
        "shift" => Some(Key::Shift),
        "meta" | "win" | "windows" => Some(Key::Meta),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() {
                Some(Key::Unicode(c))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_single_action(value: &str) -> Option<Action> {
    let value = value.trim();
    if value.contains('+') {
        let parts: Vec<&str> = value.split('+').map(|s| s.trim()).collect();
        let keys: Vec<Key> = parts.iter().filter_map(|s| str_to_key(s)).collect();
        if keys.len() == parts.len() && keys.len() >= 2 {
            Some(Action::KeyChord(keys))
        } else {
            None
        }
    } else {
        str_to_key(value).map(Action::KeyPress)
    }
}

pub fn parse_action(value: &str) -> Option<Action> {
    let value = value.trim();
    // Comma-separated = Sequence
    if value.contains(',') {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        let actions: Vec<Action> = parts.iter().filter_map(|s| parse_single_action(s)).collect();
        if actions.len() == parts.len() && actions.len() >= 2 {
            Some(Action::Sequence(actions))
        } else {
            None
        }
    } else {
        parse_single_action(value)
    }
}

// ---- ActionExecutor ----

pub struct ActionExecutor {
    enigo: Enigo,
}

impl ActionExecutor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo })
    }

    pub fn execute(&mut self, action: &Action) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::KeyPress(key) => {
                self.enigo.key(*key, Direction::Click)?;
            }
            Action::KeyChord(keys) => {
                let modifiers = &keys[..keys.len() - 1];
                let main_key = keys[keys.len() - 1];
                for m in modifiers {
                    self.enigo.key(*m, Direction::Press)?;
                }
                self.enigo.key(main_key, Direction::Click)?;
                for m in modifiers.iter().rev() {
                    self.enigo.key(*m, Direction::Release)?;
                }
            }
            Action::Sequence(actions) => {
                for a in actions {
                    self.execute(a)?;
                }
            }
        }
        Ok(())
    }
}

// ---- MappingTable with modifier support ----

pub struct MappingTable {
    table: HashMap<(u32, ModifierFlags), Action>,
}

impl MappingTable {
    pub fn from_cursor_mappings(cursor_map: &HashMap<String, String>) -> Self {
        let mut table = HashMap::new();
        for (key_name, action_str) in cursor_map {
            if let Some((vk, mod_flags)) = char_to_vk(key_name) {
                if let Some(action) = parse_action(action_str) {
                    table.insert((vk, mod_flags), action);
                } else {
                }
            } else {
            }
        }
        Self { table }
    }

    pub fn lookup(&self, vk: u32, mod_flags: ModifierFlags) -> Option<Action> {
        // Exact match first
        if let Some(action) = self.table.get(&(vk, mod_flags)) {
            return Some(action.clone());
        }
        // Fallback: try without modifiers
        if mod_flags != MOD_NONE {
            self.table.get(&(vk, MOD_NONE)).cloned()
        } else {
            None
        }
    }
}

// ---- Key name to VK + modifier flags ----

fn char_to_vk(s: &str) -> Option<(u32, ModifierFlags)> {
    let s = s.trim();
    // Parse modifier prefix: "shift+e", "ctrl+a", "alt+f", "meta+x"
    let (mod_flags, key_part) = parse_modifier_prefix(s);
    let vk = vk_from_name(key_part)?;
    Some((vk, mod_flags))
}

fn parse_modifier_prefix(s: &str) -> (ModifierFlags, &str) {
    let lower = s.to_lowercase();
    for (prefix, flag) in &[
        ("shift+", MOD_SHIFT),
        ("ctrl+", MOD_CTRL),
        ("alt+", MOD_ALT),
        ("meta+", MOD_META),
        ("win+", MOD_META),
    ] {
        if lower.starts_with(prefix) {
            return (*flag, &s[prefix.len()..]);
        }
    }
    (MOD_NONE, s)
}

fn vk_from_name(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.len() == 1 {
        let c = s.chars().next().unwrap();
        if c.is_ascii_uppercase() || c.is_ascii_lowercase() {
            return Some(c.to_ascii_uppercase() as u32);
        }
        if c.is_ascii_digit() {
            return Some(c as u32);
        }
    }
    match s.to_lowercase().as_str() {
        "comma" => Some(0xBC),
        "period" => Some(0xBE),
        "semicolon" => Some(0xBA),
        "slash" => Some(0xBF),
        "backslash" => Some(0xDC),
        "minus" => Some(0xBD),
        "equals" => Some(0xBB),
        "quote" => Some(0xDE),
        "bracketleft" => Some(0xDB),
        "bracketright" => Some(0xDD),
        // Named keys usable as triggers
        "backspace" => Some(0x08),
        "tab" => Some(0x09),
        "enter" | "return" => Some(0x0D),
        "space" => Some(0x20),
        "delete" => Some(0x2E),
        "insert" => Some(0x2D),
        "home" => Some(0x24),
        "end" => Some(0x23),
        "pageup" => Some(0x21),
        "pagedown" => Some(0x22),
        "up" => Some(0x26),
        "down" => Some(0x28),
        "left" => Some(0x25),
        "right" => Some(0x27),
        "escape" | "esc" => Some(0x1B),
        "f1" => Some(0x70), "f2" => Some(0x71), "f3" => Some(0x72),
        "f4" => Some(0x73), "f5" => Some(0x74), "f6" => Some(0x75),
        "f7" => Some(0x76), "f8" => Some(0x77), "f9" => Some(0x78),
        "f10" => Some(0x79), "f11" => Some(0x7A), "f12" => Some(0x7B),
        _ => None,
    }
}
