//! macOS: in-game hotkeys as system hotkeys (Carbon's RegisterEventHotKey, through Tauri's
//! global-shortcut plugin). They reach the app whichever app is in front, the game included, the
//! key press goes to the app only, and no permission is needed (a keyboard tap would need
//! Input Monitoring).

use super::{Action, Combo};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// The plugin's key for a Windows virtual-key code (`super::parse` gives those on every system).
fn code(vk: u16) -> Option<Code> {
    const LETTERS: [Code; 26] = [
        Code::KeyA, Code::KeyB, Code::KeyC, Code::KeyD, Code::KeyE, Code::KeyF, Code::KeyG, Code::KeyH, Code::KeyI,
        Code::KeyJ, Code::KeyK, Code::KeyL, Code::KeyM, Code::KeyN, Code::KeyO, Code::KeyP, Code::KeyQ, Code::KeyR,
        Code::KeyS, Code::KeyT, Code::KeyU, Code::KeyV, Code::KeyW, Code::KeyX, Code::KeyY, Code::KeyZ,
    ];
    const DIGITS: [Code; 10] =
        [Code::Digit0, Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4, Code::Digit5, Code::Digit6, Code::Digit7, Code::Digit8, Code::Digit9];
    const FUNCTION: [Code; 24] = [
        Code::F1, Code::F2, Code::F3, Code::F4, Code::F5, Code::F6, Code::F7, Code::F8, Code::F9, Code::F10, Code::F11, Code::F12,
        Code::F13, Code::F14, Code::F15, Code::F16, Code::F17, Code::F18, Code::F19, Code::F20, Code::F21, Code::F22, Code::F23,
        Code::F24,
    ];
    Some(match vk {
        0x41..=0x5A => LETTERS[(vk - 0x41) as usize],
        0x30..=0x39 => DIGITS[(vk - 0x30) as usize],
        0x70..=0x87 => FUNCTION[(vk - 0x70) as usize],
        0xC0 => Code::Backquote,
        0xBD => Code::Minus,
        0xBB => Code::Equal,
        0xDB => Code::BracketLeft,
        0xDD => Code::BracketRight,
        0xDC => Code::Backslash,
        0xBA => Code::Semicolon,
        0xDE => Code::Quote,
        0xBC => Code::Comma,
        0xBE => Code::Period,
        0xBF => Code::Slash,
        0x20 => Code::Space,
        0x0D => Code::Enter,
        0x09 => Code::Tab,
        0x1B => Code::Escape,
        0x08 => Code::Backspace,
        0x2E => Code::Delete,
        0x2D => Code::Insert,
        0x24 => Code::Home,
        0x23 => Code::End,
        0x21 => Code::PageUp,
        0x22 => Code::PageDown,
        0x26 => Code::ArrowUp,
        0x28 => Code::ArrowDown,
        0x25 => Code::ArrowLeft,
        0x27 => Code::ArrowRight,
        _ => return None,
    })
}

fn shortcut(c: &Combo) -> Option<Shortcut> {
    let mut m = Modifiers::empty();
    m.set(Modifiers::CONTROL, c.ctrl);
    m.set(Modifiers::ALT, c.alt);
    m.set(Modifiers::SHIFT, c.shift);
    m.set(Modifiers::SUPER, c.meta);
    Some(Shortcut::new((!m.is_empty()).then_some(m), code(c.vk)?))
}

/// Registers these keys (replacing earlier ones), or removes them all when empty.
pub fn enable(bindings: Vec<(Combo, Action)>) {
    let Some(core) = crate::core::get() else { return };
    let gs = core.app.global_shortcut();
    let _ = gs.unregister_all();
    for (combo, action) in bindings {
        let Some(sc) = shortcut(&combo) else {
            log::warn!("hotkey {combo:?} has no macOS key");
            continue;
        };
        let registered = gs.on_shortcut(sc, move |_app, _sc, event| {
            if event.state == ShortcutState::Pressed {
                super::fire(action);
            }
        });
        if let Err(e) = registered {
            // Another app holds the same keys.
            log::warn!("hotkey {sc}: {e}");
        }
    }
}

pub fn disable() {
    if let Some(core) = crate::core::get() {
        let _ = core.app.global_shortcut().unregister_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_recorded_hotkeys_to_mac_keys() {
        let sc = |t: &str| shortcut(&super::super::parse(t).unwrap()).unwrap();
        assert_eq!(sc("Ctrl+Shift+M"), Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyM));
        assert_eq!(sc("Cmd+F2"), Shortcut::new(Some(Modifiers::SUPER), Code::F2));
        assert_eq!(sc("`"), Shortcut::new(None, Code::Backquote));
        assert_eq!(sc("Alt+0"), Shortcut::new(Some(Modifiers::ALT), Code::Digit0));
    }
}
