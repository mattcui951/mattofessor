//! In-game hotkeys through a low-level keyboard hook.
//!
//! League's game window tells Windows to skip RegisterHotKey hotkeys while it has focus (raw
//! input with RIDEV_NOHOTKEYS), so ordinary global shortcuts only fire when some other window is
//! in front. A WH_KEYBOARD_LL hook sees every key before that. It is installed only while a match
//! runs, and a matching key press is swallowed so the game does not also act on it.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

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
    pub win: bool,
    pub vk: u16,
}

/// Parses the settings format ("Ctrl+Shift+M", "Alt+F2", "`").
pub fn parse(text: &str) -> Option<Combo> {
    let mut c = Combo { ctrl: false, alt: false, shift: false, win: false, vk: 0 };
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
            "win" | "super" | "meta" | "cmd" | "command" => c.win = true,
            _ => return None,
        }
    }
    c.vk = vk_of(key)?;
    Some(c)
}

/// Windows virtual-key code for a key name as the hotkey recorder writes it (US layout).
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

static BINDINGS: Mutex<Vec<(Combo, Action)>> = Mutex::new(Vec::new());
/// The hotkey key that went down and is still held (its auto-repeat and release are swallowed too).
static HELD: Mutex<Option<u16>> = Mutex::new(None);
/// Thread running the hook's message loop; 0 when no hook is installed.
static THREAD: AtomicU32 = AtomicU32::new(0);

/// Installs the hook with these keys (replacing earlier ones), or removes it when empty.
pub fn enable(bindings: Vec<(Combo, Action)>) {
    if bindings.is_empty() {
        disable();
        return;
    }
    *BINDINGS.lock() = bindings;
    if THREAD.load(Ordering::Acquire) != 0 {
        return;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("hotkeys".into())
        .spawn(move || run(tx))
        .expect("hotkey thread");
    match rx.recv() {
        Ok(Ok(id)) => THREAD.store(id, Ordering::Release),
        Ok(Err(e)) => log::warn!("keyboard hook: {e}"),
        Err(_) => log::warn!("keyboard hook thread ended"),
    }
}

pub fn disable() {
    BINDINGS.lock().clear();
    *HELD.lock() = None;
    let id = THREAD.swap(0, Ordering::AcqRel);
    if id != 0 {
        unsafe {
            let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

fn run(ready: std::sync::mpsc::Sender<Result<u32, String>>) {
    unsafe {
        let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0) {
            Ok(h) => h,
            Err(e) => {
                let _ = ready.send(Err(e.to_string()));
                return;
            }
        };
        let _ = ready.send(Ok(GetCurrentThreadId()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(hook);
    }
}

fn down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// Runs for every key on the system: keep it short (Windows drops slow hooks).
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = info.vkCode as u16;
        let msg = wparam.0 as u32;
        let pressed = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let released = msg == WM_KEYUP || msg == WM_SYSKEYUP;
        let mut held = HELD.lock();
        if *held == Some(vk) {
            if released {
                *held = None;
            }
            return LRESULT(1);
        }
        if pressed {
            let now = Combo {
                ctrl: down(VK_CONTROL.0),
                alt: down(VK_MENU.0),
                shift: down(VK_SHIFT.0),
                win: down(VK_LWIN.0) || down(VK_RWIN.0),
                vk,
            };
            let action = BINDINGS.lock().iter().find(|(c, _)| *c == now).map(|(_, a)| *a);
            if let Some(action) = action {
                *held = Some(vk);
                if let Some(core) = crate::core::get() {
                    let core = core.clone();
                    let _ = core.app.clone().run_on_main_thread(move || core.on_hotkey(action));
                }
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(ctrl: bool, alt: bool, shift: bool, vk: u16) -> Option<Combo> {
        Some(Combo { ctrl, alt, shift, win: false, vk })
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
