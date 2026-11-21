//! macOS: processes (libproc), other apps' windows (Quartz window list), the app's in-game
//! windows as non-activating panels (AppKit) and screen captures (ScreenCaptureKit).
//!
//! Coordinates are points from the top left of the main display, as Quartz gives them. AppKit
//! counts from the bottom left instead; `to_cocoa` turns one into the other.

use super::{Image, ProcInfo, Rect, WindowId};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{define_class, AnyThread, ClassType, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSPanel, NSResponder, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_core_foundation::{CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextCreateImage, CGColorSpace, CGContext, CGDisplayBounds, CGImage, CGImageAlphaInfo,
    CGMainDisplayID, CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess,
    CGWindowListCopyWindowInfo, CGWindowListOption,
};
use objc2_foundation::{NSArray, NSDictionary, NSError, NSNumber, NSObject, NSString};
use std::ffi::c_void;
use std::path::PathBuf;
use std::time::Duration;
use tauri::WebviewWindow;

// ---------------------------------------------------------------------------------------------
// Processes

fn pid_path(pid: i32) -> Option<PathBuf> {
    let mut buf = vec![0u8; 4 * 1024]; // PROC_PIDPATHINFO_MAXSIZE
    let n = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    (n > 0).then(|| PathBuf::from(String::from_utf8_lossy(&buf[..n as usize]).into_owned()))
}

/// Running processes, named after their executable file (as on Windows, where the name is the
/// .exe's: `proc_name` alone cuts long names short).
pub fn processes() -> Vec<ProcInfo> {
    let n = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if n <= 0 {
        return Vec::new();
    }
    let mut pids = vec![0i32; n as usize + 64];
    let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), (pids.len() * 4) as i32) };
    pids.truncate(n.max(0) as usize);
    pids.into_iter()
        .filter(|&pid| pid > 0)
        .filter_map(|pid| {
            let name = match pid_path(pid) {
                Some(p) => p.file_name()?.to_string_lossy().into_owned(),
                None => {
                    let mut buf = [0u8; 64];
                    let n = unsafe { libc::proc_name(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
                    if n <= 0 {
                        return None;
                    }
                    String::from_utf8_lossy(&buf[..n as usize]).into_owned()
                }
            };
            Some(ProcInfo { pid: pid as u32, name })
        })
        .collect()
}

/// Full path of a running process's executable.
pub fn process_path(pid: u32) -> Option<PathBuf> {
    pid_path(pid as i32)
}

// ---------------------------------------------------------------------------------------------
// Other apps' windows

const ON_SCREEN_ONLY: u32 = 1 << 0;
const INCLUDING_WINDOW: u32 = 1 << 3;
const EXCLUDE_DESKTOP: u32 = 1 << 4;

struct WinInfo {
    id: u32,
    pid: u32,
    layer: i64,
    rect: Rect,
}

fn number(dict: &NSDictionary<NSString, AnyObject>, key: &str) -> Option<f64> {
    let v = dict.objectForKey(&NSString::from_str(key))?;
    Some(v.downcast_ref::<NSNumber>()?.as_f64())
}

fn windows_list(option: u32, relative_to: u32) -> Vec<WinInfo> {
    let Some(list) = CGWindowListCopyWindowInfo(CGWindowListOption(option), relative_to) else { return Vec::new() };
    // CFArray of CFDictionary, toll-free bridged to their Foundation types.
    let list: &NSArray<NSDictionary<NSString, AnyObject>> = unsafe { &*(&*list as *const _ as *const NSArray<_>) };
    list.iter()
        .filter_map(|d| {
            let bounds = d.objectForKey(&NSString::from_str("kCGWindowBounds"))?;
            let bounds: &NSDictionary<NSString, AnyObject> = bounds.downcast_ref::<NSDictionary>().map(|b| unsafe { b.cast_unchecked() })?;
            let rect = Rect {
                x: number(bounds, "X")?.round() as i32,
                y: number(bounds, "Y")?.round() as i32,
                w: number(bounds, "Width")?.round() as i32,
                h: number(bounds, "Height")?.round() as i32,
            };
            Some(WinInfo {
                id: number(&d, "kCGWindowNumber")? as u32,
                pid: number(&d, "kCGWindowOwnerPID")? as u32,
                layer: number(&d, "kCGWindowLayer").unwrap_or(0.0) as i64,
                rect,
            })
        })
        .collect()
}

/// Largest on-screen window owned by `pid` (menu bar items and other small helpers left out).
pub fn main_window_of(pid: u32) -> Option<WindowId> {
    windows_list(ON_SCREEN_ONLY | EXCLUDE_DESKTOP, 0)
        .into_iter()
        .filter(|w| w.pid == pid && w.layer >= 0 && w.rect.w > 1 && w.rect.h > 1)
        .max_by_key(|w| w.rect.w as i64 * w.rect.h as i64)
        .map(|w| w.id as WindowId)
}

pub fn window_rect(id: WindowId) -> Option<Rect> {
    windows_list(INCLUDING_WINDOW, id as u32).into_iter().find(|w| w.id == id as u32).map(|w| w.rect).filter(|r| r.w > 0 && r.h > 0)
}

fn displays() -> Vec<(u32, Rect)> {
    let mut ids = [0u32; 16];
    let mut n = 0u32;
    unsafe { objc2_core_graphics::CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut n) };
    ids[..n as usize]
        .iter()
        .map(|&id| {
            let b = CGDisplayBounds(id);
            (id, Rect { x: b.origin.x as i32, y: b.origin.y as i32, w: b.size.width as i32, h: b.size.height as i32 })
        })
        .collect()
}

/// The display holding most of `r` (the main display when none does).
fn display_at(r: Rect) -> (u32, Rect) {
    let overlap = |d: &Rect| {
        let w = (r.x + r.w).min(d.x + d.w) - r.x.max(d.x);
        let h = (r.y + r.h).min(d.y + d.h) - r.y.max(d.y);
        w.max(0) as i64 * h.max(0) as i64
    };
    displays().into_iter().max_by_key(|(_, d)| overlap(d)).filter(|(_, d)| overlap(d) > 0).unwrap_or_else(|| {
        let id = CGMainDisplayID();
        let b = CGDisplayBounds(id);
        (id, Rect { x: b.origin.x as i32, y: b.origin.y as i32, w: b.size.width as i32, h: b.size.height as i32 })
    })
}

/// Bounds of the display showing a window.
pub fn monitor_rect(id: WindowId) -> Option<Rect> {
    Some(display_at(window_rect(id)?).1)
}

/// A Tauri monitor's bounds in points.
pub fn tauri_monitor_rect(m: &tauri::Monitor) -> Rect {
    let s = m.scale_factor().max(1.0);
    let (p, z) = (m.position(), m.size());
    Rect { x: (p.x as f64 / s).round() as i32, y: (p.y as f64 / s).round() as i32, w: (z.width as f64 / s) as i32, h: (z.height as f64 / s) as i32 }
}

/// Points are CSS pixels.
pub fn screen_scale(_app: &tauri::AppHandle, _r: Rect) -> f64 {
    1.0
}

/// There is nothing to restore: a game in Full screen on macOS keeps its own space.
pub fn restore_if_minimized(_id: WindowId) {}

// ---------------------------------------------------------------------------------------------
// The app's own in-game windows

define_class!(
    /// An in-game window: a panel that never becomes the key window and never activates the app,
    /// so a click on it leaves the game in front with the keyboard (WS_EX_NOACTIVATE on Windows).
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MattofessorOverlayPanel"]
    struct OverlayPanel;

    impl OverlayPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            false
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// Runs `f` with the window's NSWindow on the main thread (AppKit only works there).
fn with_ns_window(w: &WebviewWindow, f: impl FnOnce(&NSWindow) + Send + 'static) {
    let Ok(ptr) = w.ns_window() else { return };
    let ptr = ptr as usize;
    let run = move || {
        // SAFETY: Tauri's own NSWindow, alive while its window is, used on the main thread.
        let window = unsafe { &*(ptr as *const NSWindow) };
        f(window);
    };
    if MainThreadMarker::new().is_some() {
        run();
    } else {
        let _ = w.run_on_main_thread(run);
    }
}

fn main_display_height() -> f64 {
    CGDisplayBounds(CGMainDisplayID()).size.height
}

/// A rect in Quartz points (top left) to an AppKit frame (bottom left of the main display).
fn to_cocoa(r: Rect) -> CGRect {
    CGRect::new(CGPoint::new(r.x as f64, main_display_height() - (r.y + r.h) as f64), CGSize::new(r.w as f64, r.h as f64))
}

/// Where one of the app's own windows is (points).
pub fn own_rect(w: &WebviewWindow) -> Option<Rect> {
    let s = w.scale_factor().ok()?.max(1.0);
    let (p, z) = (w.outer_position().ok()?, w.outer_size().ok()?);
    Some(Rect { x: (p.x as f64 / s).round() as i32, y: (p.y as f64 / s).round() as i32, w: (z.width as f64 / s).round() as i32, h: (z.height as f64 / s).round() as i32 })
}

/// Make an in-game window a non-activating panel that shows on every space, over a game in
/// Full screen too.
pub fn prepare_overlay(w: &WebviewWindow) {
    with_ns_window(w, |window| unsafe {
        // Tauri made a plain window; giving it the panel class keeps everything else it set up.
        AnyObject::set_class(window, OverlayPanel::class());
        window.setStyleMask(window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        window.setHidesOnDeactivate(false);
        let panel: &NSPanel = &*(window as *const NSWindow as *const NSPanel);
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(true);
    });
}

/// Over everything but the menu bar's menus (kCGPopUpMenuWindowLevel): above a game's window.
const OVERLAY_LEVEL: isize = 101;

/// Place an in-game window and show it without activating the app, so the game keeps focus.
pub fn show_topmost(w: &WebviewWindow, r: Rect) {
    let frame = to_cocoa(r);
    with_ns_window(w, move |window| {
        window.setLevel(OVERLAY_LEVEL);
        window.setFrame_display(frame, true);
        window.orderFrontRegardless();
    });
}

pub fn hide(w: &WebviewWindow) {
    with_ns_window(w, |window| window.orderOut(None));
}

pub fn is_visible(w: &WebviewWindow) -> bool {
    w.is_visible().unwrap_or(false)
}

// ---------------------------------------------------------------------------------------------
// Screen capture

/// Whether the user let Mattofessor record the screen (System Settings > Privacy & Security >
/// Screen & System Audio Recording). The card reader needs it.
pub fn screen_capture_allowed() -> bool {
    CGPreflightScreenCaptureAccess()
}

/// Ask for screen recording (macOS shows its prompt once; later it only lists the app in
/// System Settings).
pub fn request_screen_capture() {
    if !CGPreflightScreenCaptureAccess() {
        CGRequestScreenCaptureAccess();
    }
}

/// What the screen shows inside `r` (points), at the display's own resolution, without
/// Mattofessor's own windows. Blocks up to a second; call it from a blocking task.
pub fn capture(r: Rect) -> Option<Image> {
    if r.w <= 0 || r.h <= 0 || !CGPreflightScreenCaptureAccess() {
        return None;
    }
    let image = screenshot(r)?;
    rgba(&image)
}

/// Waits for an Objective-C completion handler's result.
fn wait<T: Send + 'static>(start: impl FnOnce(std::sync::mpsc::SyncSender<T>)) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    start(tx);
    rx.recv_timeout(Duration::from_secs(1)).ok()
}

/// A `Retained` that crosses to the waiting thread. Objective-C objects are reference counted
/// atomically; the receiver is the only user once it arrives.
struct Sendable<T>(T);
unsafe impl<T> Send for Sendable<T> {}

fn screenshot(r: Rect) -> Option<CFRetained<CGImage>> {
    use objc2_screen_capture_kit::{SCContentFilter, SCDisplay, SCRunningApplication, SCScreenshotManager, SCShareableContent, SCStreamConfiguration};
    // macOS 13 has no screenshots in ScreenCaptureKit (the card reader needs 14).
    AnyClass::get(c"SCScreenshotManager")?;
    let content = wait(|tx| {
        let block = block2::RcBlock::new(move |content: *mut SCShareableContent, _err: *mut NSError| {
            let content = unsafe { Retained::retain(content) };
            let _ = tx.try_send(Sendable(content));
        });
        unsafe { SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(false, true, &block) };
    })?
    .0?;
    let (display_id, bounds) = display_at(r);
    let displays = unsafe { content.displays() };
    let display: Retained<SCDisplay> = displays.iter().find(|d| unsafe { d.displayID() } == display_id)?;
    let me = std::process::id() as i32;
    let mine: Vec<Retained<SCRunningApplication>> = unsafe { content.applications() }.iter().filter(|a| unsafe { a.processID() } == me).collect();
    let filter = unsafe {
        SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
            SCContentFilter::alloc(),
            &display,
            &NSArray::from_retained_slice(&mine),
            &NSArray::new(),
        )
    };
    let scale = unsafe { filter.pointPixelScale() }.max(1.0) as f64;
    let config = unsafe { SCStreamConfiguration::new() };
    // Relative to the display, in points; the image in pixels.
    let source = CGRect::new(CGPoint::new((r.x - bounds.x) as f64, (r.y - bounds.y) as f64), CGSize::new(r.w as f64, r.h as f64));
    unsafe {
        config.setSourceRect(source);
        config.setWidth((r.w as f64 * scale).round() as usize);
        config.setHeight((r.h as f64 * scale).round() as usize);
        config.setShowsCursor(false);
        config.setScalesToFit(false);
    }
    wait(|tx| {
        let block = block2::RcBlock::new(move |image: *mut CGImage, _err: *mut NSError| {
            let image = std::ptr::NonNull::new(image).map(|i| unsafe { CFRetained::retain(i) });
            let _ = tx.try_send(Sendable(image));
        });
        unsafe { SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(&filter, &config, Some(&block)) };
    })?
    .0
}

/// A CGImage's pixels as RGBA rows.
fn rgba(image: &CGImage) -> Option<Image> {
    let (w, h) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    if w == 0 || h == 0 {
        return None;
    }
    let mut px = vec![0u8; w * h * 4];
    let space = CGColorSpace::new_device_rgb()?;
    let ctx = unsafe {
        CGBitmapContextCreate(px.as_mut_ptr().cast::<c_void>(), w, h, 8, w * 4, Some(&space), CGImageAlphaInfo::NoneSkipLast.0)
    }?;
    CGContext::draw_image(Some(&ctx), CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(w as f64, h as f64)), Some(image));
    drop(ctx);
    for p in px.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    Some(Image { w: w as u32, h: h as u32, rgba: px })
}

/// An RGBA buffer as a CGImage (for text recognition).
pub(crate) fn cg_image(rgba: &[u8], w: u32, h: u32) -> Option<CFRetained<CGImage>> {
    let mut px = rgba.to_vec();
    let space = CGColorSpace::new_device_rgb()?;
    let ctx = unsafe {
        CGBitmapContextCreate(px.as_mut_ptr().cast::<c_void>(), w as usize, h as usize, 8, w as usize * 4, Some(&space), CGImageAlphaInfo::NoneSkipLast.0)
    }?;
    // Copies the pixels: the buffer may go after this.
    let image = CGBitmapContextCreateImage(Some(&ctx));
    drop(ctx);
    drop(px);
    image
}
