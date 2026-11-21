//! In-game hotkeys. Ordinary global shortcuts do not reach the app while League's game has
//! focus on Windows, so each system has its own way in: a low-level keyboard hook on Windows
//! (`windows.rs`), system hotkeys on macOS (`macos.rs`). Both are on only while a match runs, and
//! a hotkey press is kept from the game.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{disable, enable};
#[cfg(windows)]
pub use windows::{disable, enable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Overlay,
    Spells,
}

/// A key with the exact modifiers that must be held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combo {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// The Windows key, or Command on macOS.
    pub meta: bool,
    pub vk: u16,
}

/// Parses the settings format ("Ctrl+Shift+M", "Alt+F2", "`").
pub fn parse(text: &str) -> Option<Combo> {
    let mut c = Combo { ctrl: false, alt: false, shift: false, meta: false, vk: 0 };
    let parts: Vec<&str> = text.split('+').map(str::trim).collect();
    // "Ctrl++" (the plus key) splits into trailing empty parts.
    let (key, mods) = match parts.as_slice() {
        [mods @ .., "", ""] => ("+", mods),
        [mods @ .., key] => (*key, mods),
        [] => return None,
    };
    for m in mods {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => c.ctrl = true,
            "alt" | "option" => c.alt = true,
            "shift" => c.shift = true,
            "win" | "super" | "meta" | "cmd" | "command" => c.meta = true,
            _ => return None,
        }
    }
    c.vk = vk_of(key)?;
    Some(c)
}

/// Windows virtual-key code for a key name (the key's id on every system) as the hotkey recorder writes it (US layout).
fn vk_of(key: &str) -> Option<u16> {
    let mut chars = key.chars();
    if let (Some(ch), None) = (chars.next(), chars.clone().next()) {
        let ch = ch.to_ascii_uppercase();
        if ch.is_ascii_alphanumeric() {
            return Some(ch as u16);
        }
        // Symbols, also as typed with Shift held ("Shift+!" is the 1 key).
        let base = "`1234567890-=[]\\;',./";
        let shifted = "~!@#$%^&*()_+{}|:\"<>?";
        let ch = shifted.find(ch).and_then(|i| base.chars().nth(i)).unwrap_or(ch);
        return Some(match ch {
            '`' => 0xC0,
            '-' => 0xBD,
            '=' => 0xBB,
            '[' => 0xDB,
            ']' => 0xDD,
            '\\' => 0xDC,
            ';' => 0xBA,
            '\'' => 0xDE,
            ',' => 0xBC,
            '.' => 0xBE,
            '/' => 0xBF,
            c if c.is_ascii_digit() => c as u16,
            _ => return None,
        });
    }
    let lower = key.to_ascii_lowercase();
    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
        return (1..=24).contains(&n).then_some(0x70 + n - 1);
    }
    Some(match lower.as_str() {
        "space" => 0x20,
        "enter" | "return" => 0x0D,
        "tab" => 0x09,
        "esc" | "escape" => 0x1B,
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "insert" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "up" | "arrowup" => 0x26,
        "down" | "arrowdown" => 0x28,
        "left" | "arrowleft" => 0x25,
        "right" | "arrowright" => 0x27,
        _ => return None,
    })
}

/// Tells the app a hotkey was pressed (on the main thread).
fn fire(action: Action) {
    if let Some(core) = crate::core::get() {
        let core = core.clone();
        let _ = core.app.clone().run_on_main_thread(move || core.on_hotkey(action));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(ctrl: bool, alt: bool, shift: bool, vk: u16) -> Option<Combo> {
        Some(Combo { ctrl, alt, shift, meta: false, vk })
    }

    #[test]
    fn parses_recorded_hotkeys() {
        assert_eq!(parse("Ctrl+X"), c(true, false, false, b'X' as u16));
        assert_eq!(parse("Ctrl+Shift+M"), c(true, false, true, b'M' as u16));
        assert_eq!(parse("Alt+F2"), c(false, true, false, 0x71));
        assert_eq!(parse("`"), c(false, false, false, 0xC0));
        assert_eq!(parse("Shift+!"), c(false, false, true, b'1' as u16));
        assert_eq!(parse("Ctrl++"), c(true, false, false, 0xBB));
        assert_eq!(parse("Ctrl+Space"), c(true, false, false, 0x20));
        assert_eq!(parse("Ctrl+x"), c(true, false, false, b'X' as u16));
    }

    #[test]
    fn rejects_unknown() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("Ctrl+Hyper"), None);
        assert_eq!(parse("Foo+X"), None);
        assert_eq!(parse("F25"), None);
    }
}
