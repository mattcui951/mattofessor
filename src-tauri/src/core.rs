//! App-wide state: settings, windows and hotkeys.

use crate::game::LiveState;
use crate::live::InGame;
use crate::model::Summoner;
use crate::settings::Settings;
use crate::win;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

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
    spells_content: AtomicU32,
    /// Where the spell timers were last placed, to tell a user's drag from our own placement.
    spells_placed: Mutex<Option<(i32, i32)>>,
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
            spells_content: AtomicU32::new(300),
            spells_placed: Mutex::new(None),
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
        let _ = self.app.emit("settings", self.settings());
        Ok(())
    }

    /// "en" or "zh".
    pub fn language(&self) -> String {
        let s = self.settings.read().language.clone();
        if s == "en" || s == "zh" {
            return s;
        }
        match self.client_locale.read().as_deref() {
            Some(l) if l.starts_with("zh") => "zh".into(),
            _ => "en".into(),
        }
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

    /// Show the main window, optionally on a page ("live", "profile", "builds", "settings").
    pub fn show_main(&self, route: Option<&str>) {
        if let Some(w) = self.app.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        } else {
            let hash = route.map(|r| format!("#/{r}")).unwrap_or_default();
            let built = WebviewWindowBuilder::new(&self.app, "main", WebviewUrl::App(format!("index.html{hash}").into()))
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
            let _ = self.app.emit_to("main", "navigate", r);
        }
    }

    fn overlay_window(&self, label: &str) -> Option<WebviewWindow> {
        if let Some(w) = self.app.get_webview_window(label) {
            return Some(w);
        }
        let w = WebviewWindowBuilder::new(&self.app, label, WebviewUrl::App(format!("index.html#{label}").into()))
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
        if let Ok(h) = w.hwnd() {
            win::set_no_activate(h.0 as isize);
        }
        Some(w)
    }

    /// Screen area to cover: the game window, or the main monitor when there is no game.
    fn game_area(&self) -> win::Rect {
        if let Some(r) = win::game_window().and_then(win::window_rect) {
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
        self.app.get_webview_window(label).and_then(|w| w.hwnd().ok()).is_some_and(|h| win::is_visible(h.0 as isize))
    }

    pub fn overlay_visible(&self) -> bool {
        self.visible("overlay")
    }

    /// Centered on the game, as tall as its content (within the game area).
    fn overlay_rect(&self) -> win::Rect {
        let area = self.game_area();
        let scale = self.scale_at(area);
        let width = ((1500.0 * scale * self.ui_scale()) as i32).min(area.w - (40.0 * scale) as i32);
        let content = self.overlay_content.load(Ordering::Relaxed) as f64;
        let height = ((content * scale) as i32).min(area.h - (60.0 * scale) as i32);
        win::Rect { x: area.x + (area.w - width) / 2, y: area.y + (area.h - height) / 2, w: width, h: height }
    }

    pub fn show_overlay(&self) {
        let Some(w) = self.overlay_window("overlay") else { return };
        if let Ok(h) = w.hwnd() {
            win::show_topmost(h.0 as isize, self.overlay_rect());
        }
        let _ = self.app.emit("overlay-shown", true);
    }

    /// The overlay page reports its content height; resize to fit when it changed.
    pub fn fit_overlay(&self, css_height: f64) {
        let h = css_height.clamp(160.0, 1400.0).round() as u32;
        if self.overlay_content.swap(h, Ordering::Relaxed) == h || !self.overlay_visible() {
            return;
        }
        if let Some(Ok(hwnd)) = self.app.get_webview_window("overlay").map(|w| w.hwnd()) {
            win::show_topmost(hwnd.0 as isize, self.overlay_rect());
        }
    }

    pub fn hide_overlay(&self) {
        self.overlay_auto.store(false, Ordering::Relaxed);
        if let Some(Ok(h)) = self.app.get_webview_window("overlay").map(|w| w.hwnd()) {
            win::hide(h.0 as isize);
        }
    }

    pub fn toggle_overlay(&self) {
        if self.overlay_visible() {
            self.hide_overlay();
        } else {
            self.overlay_auto.store(false, Ordering::Relaxed);
            self.show_overlay();
        }
    }

    pub fn spells_visible(&self) -> bool {
        self.visible("spells")
    }

    pub fn show_spells(&self) {
        let Some(w) = self.overlay_window("spells") else { return };
        let area = self.game_area();
        let scale = self.scale_at(area);
        let (width, height) = ((236.0 * scale * self.ui_scale()) as i32, (self.spells_content.load(Ordering::Relaxed) as f64 * scale) as i32);
        let (x, y) = self.settings().spells_pos.unwrap_or((area.x + (12.0 * scale) as i32, area.y + area.h / 2 - height / 2));
        if let Ok(h) = w.hwnd() {
            win::show_topmost(h.0 as isize, win::Rect { x, y, w: width, h: height });
        }
        *self.spells_placed.lock() = Some((x, y));
    }

    pub fn fit_spells(&self, css_height: f64) {
        let h = css_height.clamp(80.0, 900.0).round() as u32;
        if self.spells_content.swap(h, Ordering::Relaxed) == h || !self.spells_visible() {
            return;
        }
        if let Some(w) = self.app.get_webview_window("spells") {
            if let (Ok(hwnd), Ok(p), Ok(s)) = (w.hwnd(), w.outer_position(), w.outer_size()) {
                let scale = w.scale_factor().unwrap_or(1.0);
                win::show_topmost(hwnd.0 as isize, win::Rect { x: p.x, y: p.y, w: s.width as i32, h: (h as f64 * scale) as i32 });
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
            if let Ok(h) = w.hwnd() {
                win::hide(h.0 as isize);
            }
        }
    }

    pub fn toggle_spells(&self) {
        if self.spells_visible() {
            self.hide_spells();
        } else {
            self.show_spells();
        }
    }

    /// In-game keys exist only while a match runs, so they never take over keys in other apps.
    pub fn register_hotkeys(&self, in_game: bool) {
        self.in_game_keys.store(in_game, Ordering::Relaxed);
        let gs = self.app.global_shortcut();
        let _ = gs.unregister_all();
        if !in_game {
            return;
        }
        let s = self.settings();
        for key in [s.hotkey_overlay, s.hotkey_spells] {
            if key.is_empty() {
                continue;
            }
            match key.parse::<Shortcut>() {
                Ok(sc) => {
                    if let Err(e) = gs.register(sc) {
                        log::warn!("hotkey {key}: {e}");
                    }
                }
                Err(e) => log::warn!("hotkey {key}: {e}"),
            }
        }
    }

    pub fn reregister_hotkeys(&self) {
        self.register_hotkeys(self.in_game_keys.load(Ordering::Relaxed));
    }

    pub fn on_shortcut(&self, shortcut: &Shortcut) {
        let s = self.settings();
        let is = |k: &str| !k.is_empty() && k.parse::<Shortcut>().ok().as_ref() == Some(shortcut);
        if is(&s.hotkey_overlay) {
            self.toggle_overlay();
        } else if is(&s.hotkey_spells) {
            self.toggle_spells();
        }
    }
}
