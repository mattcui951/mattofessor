//! User settings (%APPDATA%\Mattofessor\settings.json).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "auto" (League client's language), "en" or "zh".
    pub language: String,
    /// Show / hide the in-game scouting panel. Only registered while a game runs.
    pub hotkey_overlay: String,
    /// Show / hide the enemy summoner spell timers.
    pub hotkey_spells: String,
    /// Bring the app forward on the Live page when champ select starts.
    pub open_in_champ_select: bool,
    /// Show the scouting panel by itself while the game loads; it hides when the match starts.
    pub show_during_loading: bool,
    /// Show the spell timers when a match starts (they can still be toggled with the hotkey).
    pub spells_on_start: bool,
    pub auto_import_runes: bool,
    pub auto_import_items: bool,
    pub start_with_windows: bool,
    /// Look for new versions in the background (updates are only offered, never forced).
    pub check_updates: bool,
    /// Last spell timer position (physical pixels), if the user moved it.
    pub spells_pos: Option<(i32, i32)>,
    /// Active theme id (built-in or one of `custom_themes`).
    pub theme: String,
    /// The user's own themes. Their shape belongs to the UI (src/lib/themes.ts).
    pub custom_themes: Vec<serde_json::Value>,
    /// Text and layout size, 0.8-1.4 (also sizes the in-game windows).
    pub ui_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            hotkey_overlay: "Ctrl+Shift+M".into(),
            hotkey_spells: "Ctrl+Shift+N".into(),
            open_in_champ_select: true,
            show_during_loading: true,
            spells_on_start: false,
            auto_import_runes: false,
            auto_import_items: false,
            start_with_windows: true,
            check_updates: true,
            spells_pos: None,
            theme: "arcade".into(),
            custom_themes: Vec::new(),
            ui_scale: 1.0,
        }
    }
}

impl Settings {
    pub fn path() -> std::path::PathBuf {
        crate::paths::config_dir().join("settings.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings = serde_json::from_str(r#"{"language":"zh"}"#).unwrap();
        assert_eq!(s.language, "zh");
        assert_eq!(s.hotkey_overlay, "Ctrl+Shift+M");
        assert!(s.open_in_champ_select);
        assert_eq!(s.theme, "arcade");
        assert_eq!(s.ui_scale, 1.0);
    }
}
