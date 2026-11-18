//! App-wide state: settings, windows and hotkeys.

use crate::game::LiveState;
use crate::hotkeys::{self, Action};
use crate::live::InGame;
use crate::model::Summoner;
use crate::settings::Settings;
use crate::win;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

static CORE: OnceLock<Arc<Core>> = OnceLock::new();

pub fn get() -> Option<&'static Arc<Core>> {
    CORE.get()
}

pub struct Core {
    pub app: AppHandle,
    settings: RwLock<Settings>,
    pub me: RwLock<Option<Summoner>>,
    /// League client's language ("en_US", "zh_CN"...), for the "auto" language setting.
    pub client_locale: RwLock<Option<String>>,
    /// League client region ("NA", "EUW"...), for the Riot API.
    pub client_region: RwLock<Option<String>>,
    pub live: Mutex<LiveState>,
    pub ingame: Mutex<Option<InGame>>,
    in_game_keys: AtomicBool,
    quitting: AtomicBool,
    /// The overlay was opened automatically for the loading screen (and closes by itself).
    pub overlay_auto: AtomicBool,
    /// Height the overlay's content asked for (CSS pixels).
    overlay_content: AtomicU32,
    /// The zoom last given to the panel's page (f64 bits).
    overlay_zoom: AtomicU64,
    /// The player asked for the panel or spell timers while League ran in Full screen (they
    /// stayed closed): told once the game is over.
    fullscreen_tried: AtomicBool,
    spells_content: AtomicU32,
    /// Where the spell timers were last placed, to tell a user's drag from our own placement.
    spells_placed: Mutex<Option<(i32, i32)>>,
    /// The thread the app's windows live on (`on_main`).
    main_thread: std::thread::ThreadId,
    /// A page the main window was asked to show that its page has not taken yet. A page still
    /// loading misses the "navigate" event, so the page takes the route from here.
    main_route: Mutex<Option<String>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub text: String,
    pub tone: String,
}

impl Core {
    pub fn init(app: AppHandle) -> Arc<Core> {
        let core = Arc::new(Core {
            app,
            settings: RwLock::new(Settings::load()),
            me: RwLock::new(None),
            client_locale: RwLock::new(None),
            client_region: RwLock::new(None),
            live: Mutex::new(LiveState::default()),
            ingame: Mutex::new(None),
            in_game_keys: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
            overlay_auto: AtomicBool::new(false),
            overlay_content: AtomicU32::new(640),
            overlay_zoom: AtomicU64::new(0),
            fullscreen_tried: AtomicBool::new(false),
            spells_content: AtomicU32::new(300),
            spells_placed: Mutex::new(None),
            // `init` runs in the app's setup, on the main thread.
            main_thread: std::thread::current().id(),
            main_route: Mutex::new(None),
        });
        let _ = CORE.set(core.clone());
        core
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn set_settings(&self, s: Settings) -> anyhow::Result<()> {
        s.save()?;
        *self.settings.write() = s;
        self.reregister_hotkeys();
        // A new panel size shows at once.
        if self.overlay_visible() {
            if let Some(w) = self.app.get_webview_window("overlay") {
                self.place_overlay(&w);
            }
        }
        let _ = self.app.emit("settings", self.settings());
        Ok(())
    }

    /// The UI language id (`lang`): the setting, or the League client's language for "auto".
    pub fn language(&self) -> String {
        let s = self.settings.read().language.clone();
        if crate::lang::known(&s) {
            return s;
        }
        self.client_locale.read().as_deref().map(crate::lang::from_client).unwrap_or_else(crate::lang::english).id.to_string()
    }

    /// The UI's text size; the in-game windows grow with it.
    fn ui_scale(&self) -> f64 {
        self.settings.read().ui_scale.clamp(0.8, 1.4)
    }

    pub fn region(&self) -> Option<String> {
        self.client_region.read().clone()
    }

    pub fn riot_status_changed(&self) {
        let st = crate::riot::status();
        let _ = self.app.emit("riot", st);
        if st == crate::riot::KeyStatus::Invalid {
            self.toast("[riot-key-invalid] Riot API key rejected", "error");
        }
    }

    pub fn toast(&self, text: impl Into<String>, tone: &str) {
        let _ = self.app.emit("toast", Toast { text: text.into(), tone: tone.into() });
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::Relaxed)
    }

    pub fn quit(&self) {
        self.quitting.store(true, Ordering::Relaxed);
        self.app.exit(0);
    }

    /// Runs `f` on the main thread and waits for it. Windows are looked up and created there only,
    /// one request at a time, so two threads cannot both create the same window (that once gave
    /// two stacked main windows). None when the app is closing.
    fn on_main<R: Send + 'static>(&self, f: impl FnOnce(&Core) -> R + Send + 'static) -> Option<R> {
        if std::thread::current().id() == self.main_thread {
            return Some(f(self));
        }
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        self.app
            .run_on_main_thread(move || {
                if let Some(core) = get() {
                    let _ = tx.send(f(core));
                }
            })
            .ok()?;
        rx.recv().ok()
    }

    /// Show the main window, optionally on a page ("live", "profile", "builds", "settings").
    pub fn show_main(&self, route: Option<&str>) {
        let route = route.map(str::to_string);
        self.on_main(move |core| core.show_main_here(route.as_deref()));
    }

    fn show_main_here(&self, route: Option<&str>) {
        if let Some(w) = self.app.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        } else {
            // A new window starts on the page it was opened for.
            *self.main_route.lock() = None;
            let hash = route.map(|r| format!("#/{r}")).unwrap_or_default();
            let built = WebviewWindowBuilder::new(&self.app, "main", WebviewUrl::App(format!("index.html{hash}").into()));
            let built = dev_data(built)
                .title("Mattofessor")
                .inner_size(1320.0, 860.0)
                .min_inner_size(1040.0, 680.0)
                .decorations(false)
                .center()
                .build();
            if let Err(e) = built {
                log::error!("main window: {e}");
            }
            return;
        }
        if let Some(r) = route {
            *self.main_route.lock() = Some(r.to_string());
            let _ = self.app.emit_to("main", "navigate", r);
        }
    }

    /// The page the main window was last asked to show, once (`show_main`).
    pub fn take_main_route(&self) -> Option<String> {
        self.main_route.lock().take()
    }

    /// One of the in-game windows, created (hidden) the first time.
    fn overlay_window(&self, label: &'static str) -> Option<WebviewWindow> {
        self.on_main(move |core| core.overlay_window_here(label)).flatten()
    }

    fn overlay_window_here(&self, label: &str) -> Option<WebviewWindow> {
        if let Some(w) = self.app.get_webview_window(label) {
            return Some(w);
        }
        let w = WebviewWindowBuilder::new(&self.app, label, WebviewUrl::App(format!("index.html#{label}").into()));
        let w = dev_data(w)
            .title("Mattofessor")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .inner_size(400.0, 300.0)
            .build()
            .map_err(|e| log::error!("{label} window: {e}"))
            .ok()?;
        if let Some(h) = hwnd(&w) {
            win::set_no_activate(h);
        }
        Some(w)
    }

    /// Screen area to cover: the game window, or the main monitor when there is no game.
    fn game_area(&self) -> win::Rect {
        if let Some(r) = win::game_window().and_then(win::window_rect) {
            return r;
        }
        // Champ select: the screen the League client is on.
        if let Some(r) = win::client_window().and_then(win::monitor_rect) {
            return r;
        }
        if let Some(m) = self.app.primary_monitor().ok().flatten() {
            let (p, s) = (m.position(), m.size());
            return win::Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 };
        }
        win::Rect { x: 0, y: 0, w: 1920, h: 1080 }
    }

    fn scale_at(&self, r: win::Rect) -> f64 {
        self.app
            .monitor_from_point((r.x + r.w / 2) as f64, (r.y + r.h / 2) as f64)
            .ok()
            .flatten()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0)
    }

    fn visible(&self, label: &str) -> bool {
        self.app.get_webview_window(label).as_ref().and_then(hwnd).is_some_and(win::is_visible)
    }

    fn hide_window(&self, label: &str) {
        if let Some(h) = self.app.get_webview_window(label).as_ref().and_then(hwnd) {
            win::hide(h);
        }
    }

    pub fn overlay_visible(&self) -> bool {
        self.visible("overlay")
    }

    /// Centered on the game, as tall as its content (within the game area), and the zoom its
    /// page is drawn at.
    fn overlay_layout(&self) -> (win::Rect, f64) {
        let area = self.game_area();
        let dpi = self.scale_at(area);
        let zoom = overlay_zoom(&self.settings.read().overlay_size, area.w as f64 / dpi);
        let width = ((OVERLAY_WIDTH * zoom * dpi) as i32).min(area.w);
        let content = self.overlay_content.load(Ordering::Relaxed) as f64;
        let height = ((content * zoom * dpi) as i32).min(area.h - (40.0 * dpi) as i32);
        (win::Rect { x: area.x + (area.w - width) / 2, y: area.y + (area.h - height) / 2, w: width, h: height }, zoom)
    }

    /// Size, place and zoom the panel (shown).
    fn place_overlay(&self, w: &WebviewWindow) {
        let (rect, zoom) = self.overlay_layout();
        if (f64::from_bits(self.overlay_zoom.load(Ordering::Relaxed)) - zoom).abs() > 1e-6 {
            self.overlay_zoom.store(zoom.to_bits(), Ordering::Relaxed);
            let _ = w.set_zoom(zoom);
        }
        if let Some(h) = hwnd(w) {
            win::show_topmost(h, rect);
        }
    }

    /// In-game windows stay closed over a Full screen game (it would minimize).
    fn held_back(&self) -> bool {
        crate::display::game_fullscreen()
    }

    /// The player asked for an in-game window during a Full screen game.
    fn tried_in_fullscreen(&self) {
        if !self.fullscreen_tried.swap(true, Ordering::Relaxed) {
            log::info!("in-game panel asked for while League runs in Full screen: kept closed");
            let _ = self.app.emit("fullscreen-tried", true);
        }
    }

    pub fn fullscreen_tried(&self) -> bool {
        self.fullscreen_tried.load(Ordering::Relaxed)
    }

    pub fn clear_fullscreen_tried(&self) {
        if self.fullscreen_tried.swap(false, Ordering::Relaxed) {
            let _ = self.app.emit("fullscreen-tried", false);
        }
    }

    pub fn show_overlay(&self) {
        if self.held_back() {
            return;
        }
        let Some(w) = self.overlay_window("overlay") else { return };
        self.place_overlay(&w);
        let _ = self.app.emit("overlay-shown", true);
    }

    /// The overlay page reports its content height (CSS pixels, before its zoom); resize to fit
    /// when it changed.
    pub fn fit_overlay(&self, css_height: f64) {
        let h = css_height.clamp(120.0, 2400.0).round() as u32;
        if self.overlay_content.swap(h, Ordering::Relaxed) == h || !self.overlay_visible() {
            return;
        }
        if let Some(w) = self.app.get_webview_window("overlay") {
            self.place_overlay(&w);
        }
    }

    pub fn hide_overlay(&self) {
        self.overlay_auto.store(false, Ordering::Relaxed);
        self.hide_window("overlay");
    }

    /// The player closed the panel (hotkey or its X): also bring back a Full screen game that
    /// stepped aside (minimized) when the panel showed.
    pub fn close_overlay(&self) {
        self.hide_overlay();
        if let Some(g) = win::game_window() {
            win::restore_if_minimized(g);
        }
    }

    pub fn toggle_overlay(&self) {
        if self.overlay_visible() {
            self.close_overlay();
        } else if self.held_back() {
            self.tried_in_fullscreen();
        } else {
            self.overlay_auto.store(false, Ordering::Relaxed);
            self.show_overlay();
        }
    }

    /// Show the augment card win rates over `r` (physical pixels), click-through.
    pub fn show_cards(&self, r: win::Rect, offer: &crate::cards::Offer) {
        if self.held_back() {
            return;
        }
        let Some(w) = self.overlay_window("cards") else { return };
        let _ = w.set_ignore_cursor_events(true);
        let _ = self.app.emit_to("cards", "offer", offer);
        if let Some(h) = hwnd(&w) {
            win::show_topmost(h, r);
        }
    }

    /// The card win-rate window, made ahead and hidden (`cards::prepare`).
    pub fn prepare_cards(&self) {
        if let Some(w) = self.overlay_window("cards") {
            let _ = w.set_ignore_cursor_events(true);
        }
    }

    pub fn hide_cards(&self) {
        self.hide_window("cards");
    }

    pub fn spells_visible(&self) -> bool {
        self.visible("spells")
    }

    pub fn show_spells(&self) {
        if self.held_back() {
            return;
        }
        let Some(w) = self.overlay_window("spells") else { return };
        let area = self.game_area();
        let scale = self.scale_at(area);
        let (width, height) = ((236.0 * scale * self.ui_scale()) as i32, (self.spells_content.load(Ordering::Relaxed) as f64 * scale) as i32);
        let (x, y) = self.settings().spells_pos.unwrap_or((area.x + (12.0 * scale) as i32, area.y + area.h / 2 - height / 2));
        if let Some(h) = hwnd(&w) {
            win::show_topmost(h, win::Rect { x, y, w: width, h: height });
        }
        *self.spells_placed.lock() = Some((x, y));
    }

    pub fn fit_spells(&self, css_height: f64) {
        let h = css_height.clamp(80.0, 900.0).round() as u32;
        if self.spells_content.swap(h, Ordering::Relaxed) == h || !self.spells_visible() {
            return;
        }
        if let Some(w) = self.app.get_webview_window("spells") {
            if let (Some(hw), Ok(p), Ok(s)) = (hwnd(&w), w.outer_position(), w.outer_size()) {
                let scale = w.scale_factor().unwrap_or(1.0);
                win::show_topmost(hw, win::Rect { x: p.x, y: p.y, w: s.width as i32, h: (h as f64 * scale) as i32 });
            }
        }
    }

    pub fn hide_spells(&self) {
        if let Some(w) = self.app.get_webview_window("spells") {
            if let Ok(p) = w.outer_position() {
                // Remember the position only when the user dragged the window somewhere.
                let mut s = self.settings();
                let moved = *self.spells_placed.lock() != Some((p.x, p.y));
                if moved && s.spells_pos != Some((p.x, p.y)) && self.spells_visible() {
                    s.spells_pos = Some((p.x, p.y));
                    let _ = s.save();
                    *self.settings.write() = s;
                }
            }
        }
        self.hide_window("spells");
    }

    pub fn toggle_spells(&self) {
        if self.spells_visible() {
            self.hide_spells();
        } else if self.held_back() {
            self.tried_in_fullscreen();
        } else {
            self.show_spells();
        }
    }

    /// The hotkeys exist only from champ select to the end of a game (`game::keys_phase`), so
    /// they never take over keys in other apps.
    pub fn register_hotkeys(&self, in_game: bool) {
        self.in_game_keys.store(in_game, Ordering::Relaxed);
        if !in_game {
            hotkeys::disable();
            return;
        }
        let s = self.settings();
        let mut bindings = Vec::new();
        for (key, action) in [(s.hotkey_overlay, Action::Overlay), (s.hotkey_spells, Action::Spells)] {
            if key.is_empty() {
                continue;
            }
            match hotkeys::parse(&key) {
                Some(combo) => bindings.push((combo, action)),
                None => log::warn!("hotkey {key}: not a key this app understands"),
            }
        }
        hotkeys::enable(bindings);
    }

    pub fn reregister_hotkeys(&self) {
        self.register_hotkeys(self.in_game_keys.load(Ordering::Relaxed));
    }

    pub fn on_hotkey(&self, action: Action) {
        match action {
            Action::Overlay => self.toggle_overlay(),
            // The spell timers are for a running game (champ select has the panel only).
            Action::Spells if crate::game::in_game_phase(&self.live.lock().phase) => self.toggle_spells(),
            Action::Spells => {}
        }
    }
}

/// Debug builds keep their web view data (what the windows remember) apart from the installed
/// app's. Every window must use the same folder: they share one WebView2 environment.
/// `MATTOFESSOR_DEVTOOLS_PORT=<port>` opens the windows to DevTools on that port, for tests that
/// drive the real web view (every window passes the same arguments, as one environment needs).
fn dev_data<'a>(b: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    if !cfg!(debug_assertions) {
        return b;
    }
    let b = b.data_directory(crate::paths::config_dir().join("webview"));
    match std::env::var("MATTOFESSOR_DEVTOOLS_PORT").ok().and_then(|p| p.parse::<u16>().ok()) {
        // wry's own defaults, plus the port.
        Some(port) => b.additional_browser_args(&format!(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required --remote-debugging-port={port}"
        )),
        None => b,
    }
}

/// A window's handle, for the `win` calls.
fn hwnd(w: &WebviewWindow) -> Option<isize> {
    w.hwnd().ok().map(|h| h.0 as isize)
}

/// Width the in-game panel is laid out at (CSS pixels): both teams side by side with their last
/// games. The size setting zooms it as a whole, so nothing stretches or leaves gaps.
const OVERLAY_WIDTH: f64 = 1520.0;

/// The panel's zoom for a size setting, for a game `game_width` CSS pixels wide. A game too
/// narrow for the size gets a smaller zoom instead of a squeezed layout; "xlarge" fills the width.
pub fn overlay_zoom(size: &str, game_width: f64) -> f64 {
    let fits = (game_width - 24.0) / OVERLAY_WIDTH;
    let wanted = match size {
        "small" => 0.75,
        "medium" => 0.875,
        "large" => 1.15,
        "xlarge" => fits,
        _ => 1.0,
    };
    wanted.min(fits).clamp(0.5, 3.0)
}

#[cfg(test)]
mod tests {
    use super::overlay_zoom;

    #[test]
    fn panel_zoom_follows_the_size_and_fits_the_game() {
        // A 2560-wide game has room for every size.
        assert_eq!(overlay_zoom("small", 2560.0), 0.75);
        assert_eq!(overlay_zoom("regular", 2560.0), 1.0);
        assert_eq!(overlay_zoom("large", 2560.0), 1.15);
        assert!((overlay_zoom("xlarge", 2560.0) * 1520.0 - (2560.0 - 24.0)).abs() < 1e-6);
        // Unknown sizes are regular.
        assert_eq!(overlay_zoom("huge", 2560.0), 1.0);
        // A 1366-wide laptop: even "regular" shrinks to fit rather than squeezing the layout.
        assert!(overlay_zoom("regular", 1366.0) < 0.95);
        assert!(overlay_zoom("large", 1366.0) <= overlay_zoom("xlarge", 1366.0) + 1e-9);
    }
}
