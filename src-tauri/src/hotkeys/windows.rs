//! Windows: in-game hotkeys through a low-level keyboard hook.
//!
//! League's game window tells Windows to skip RegisterHotKey hotkeys while it has focus (raw
//! input with RIDEV_NOHOTKEYS), so ordinary global shortcuts only fire when some other window is
//! in front. A WH_KEYBOARD_LL hook sees every key before that. It is installed only while a match
//! runs, and a matching key press is swallowed so the game does not also act on it.

use super::{Action, Combo};
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
                meta: down(VK_LWIN.0) || down(VK_RWIN.0),
                vk,
            };
            let action = BINDINGS.lock().iter().find(|(c, _)| *c == now).map(|(_, a)| *a);
            if let Some(action) = action {
                *held = Some(vk);
                super::fire(action);
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
