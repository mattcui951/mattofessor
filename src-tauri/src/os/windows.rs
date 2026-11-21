//! Windows: thin wrappers over the Win32 calls the app needs.

use super::{Image, ProcInfo, Rect, WindowId};
use tauri::WebviewWindow;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
    SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
};

/// Snapshot of running processes (cheap: one ToolHelp snapshot).
pub fn processes() -> Vec<ProcInfo> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                out.push(ProcInfo { pid: entry.th32ProcessID, name: String::from_utf16_lossy(&entry.szExeFile[..len]) });
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

/// Full path of a running process's executable.
pub fn process_path(pid: u32) -> Option<std::path::PathBuf> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        ok.then(|| String::from_utf16_lossy(&buf[..len as usize]).into())
    }
}

struct FindCtx {
    pid: u32,
    best: Option<(isize, i64)>,
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut FindCtx);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid != ctx.pid || !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let mut r = RECT::default();
    let _ = GetWindowRect(hwnd, &mut r);
    let area = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
    if ctx.best.is_none_or(|(_, a)| area > a) {
        ctx.best = Some((hwnd.0 as isize, area));
    }
    BOOL(1)
}

/// Largest visible top-level window owned by `pid`.
pub fn main_window_of(pid: u32) -> Option<WindowId> {
    let mut ctx = FindCtx { pid, best: None };
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.best.map(|(h, _)| h)
}

pub fn window_rect(hwnd: WindowId) -> Option<Rect> {
    let mut r = RECT::default();
    unsafe { GetWindowRect(HWND(hwnd as *mut _), &mut r).ok()? };
    let rect = Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top };
    (rect.w > 0 && rect.h > 0).then_some(rect)
}

/// Bounds of the monitor showing `hwnd` (physical pixels).
pub fn monitor_rect(hwnd: WindowId) -> Option<Rect> {
    unsafe {
        let mon = MonitorFromWindow(HWND(hwnd as *mut _), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(mon, &mut info).as_bool() {
            return None;
        }
        let r = info.rcMonitor;
        Some(Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top })
    }
}

/// A Tauri monitor's bounds in screen coordinates (physical pixels).
pub fn tauri_monitor_rect(m: &tauri::Monitor) -> Rect {
    let (p, s) = (m.position(), m.size());
    Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 }
}

/// Physical pixels per CSS pixel on the monitor showing the middle of `r`.
pub fn screen_scale(app: &tauri::AppHandle, r: Rect) -> f64 {
    app.monitor_from_point((r.x + r.w / 2) as f64, (r.y + r.h / 2) as f64)
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0)
}

fn hwnd_of(w: &WebviewWindow) -> Option<HWND> {
    w.hwnd().ok().map(|h| HWND(h.0 as *mut _))
}

/// Where one of the app's own windows is (physical pixels).
pub fn own_rect(w: &WebviewWindow) -> Option<Rect> {
    let (p, s) = (w.outer_position().ok()?, w.outer_size().ok()?);
    Some(Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 })
}

/// Make an in-game window clickable without ever taking focus from the game.
pub fn prepare_overlay(w: &WebviewWindow) {
    let Some(h) = hwnd_of(w) else { return };
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        SetWindowLongPtrW(h, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
    }
}

/// Place a window above everything (including a borderless game) and show it, without
/// activating it, so the game keeps keyboard focus.
pub fn show_topmost(w: &WebviewWindow, r: Rect) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW};
    let Some(h) = hwnd_of(w) else { return };
    unsafe {
        let _ = SetWindowPos(h, Some(HWND_TOPMOST), r.x, r.y, r.w, r.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

/// Hide a window shown with `show_topmost` (Tauri's hide() is skipped when its own flag says
/// the window is already hidden).
pub fn hide(w: &WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    let Some(h) = hwnd_of(w) else { return };
    unsafe {
        let _ = ShowWindow(h, SW_HIDE);
    }
}

/// Real visibility (Tauri's own flag misses windows shown with `show_topmost`).
pub fn is_visible(w: &WebviewWindow) -> bool {
    hwnd_of(w).is_some_and(|h| unsafe { IsWindowVisible(h).as_bool() })
}

/// Bring back a window that was minimized (a Full screen game steps aside when another window
/// shows over it). Plain window calls only: no synthetic keys while the game runs.
pub fn restore_if_minimized(hwnd: WindowId) {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, ShowWindow, SW_RESTORE};
    unsafe {
        let h = HWND(hwnd as *mut _);
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
            let _ = SetForegroundWindow(h);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Keyboard and mouse, read by asking (no hook): Tab for the team gold strip, the left button and
// the pointer for which augment card was taken. Nothing is kept from the game.

fn key_down(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// The Tab key is held.
pub fn tab_down() -> bool {
    key_down(windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB.0)
}

/// The left mouse button is held.
pub fn left_button_down() -> bool {
    key_down(windows::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON.0)
}

/// Where the pointer is (physical pixels).
pub fn cursor_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x, p.y))
}

/// The window is the one the keyboard goes to.
pub fn has_focus(hwnd: WindowId) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow().0 as isize == hwnd }
}

/// Windows lets every program read the screen.
pub fn screen_capture_allowed() -> bool {
    true
}

pub fn request_screen_capture() {}

/// What the screen shows inside `r` (physical pixels), one pixel per unit.
pub fn capture(r: Rect) -> Option<Image> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    };
    if r.w <= 0 || r.h <= 0 {
        return None;
    }
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, r.w, r.h);
        let old = SelectObject(mem, bmp.into());
        let copied = BitBlt(mem, 0, 0, r.w, r.h, Some(screen), r.x, r.y, SRCCOPY).is_ok();
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: r.w,
                biHeight: -r.h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut px = vec![0u8; (r.w * r.h * 4) as usize];
        let rows = GetDIBits(mem, bmp, 0, r.h as u32, Some(px.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        if !copied || rows != r.h {
            return None;
        }
        // BGRA to RGBA, opaque.
        for p in px.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
            p[3] = 255;
        }
        Some(Image { w: r.w as u32, h: r.h as u32, rgba: px })
    }
}
