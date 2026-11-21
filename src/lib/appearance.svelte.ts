/** The active theme and text size, applied to whichever window this page runs in. */
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { BUILT_IN, cssVars, findTheme, sanitize, type Theme } from './themes';
import type { OverlaySize, Settings } from './types';

/** The in-game panel's sizes, smallest first (Settings and the panel's own buttons). */
export const OVERLAY_SIZES: OverlaySize[] = ['small', 'medium', 'regular', 'large', 'xlarge'];

export const appearance = $state({
  theme: BUILT_IN[0] as Theme,
  custom: [] as Theme[],
  scale: 1,
});

let lastZoom = 0;

/** Apply a theme's tokens to this document (also used for live editing before saving). */
export function applyTheme(t: Theme) {
  appearance.theme = t;
  const root = document.documentElement;
  for (const [k, v] of Object.entries(cssVars(t))) {
    if (k === 'color-scheme') root.style.colorScheme = v;
    else root.style.setProperty(k, v);
  }
}

/** The lowest opacity the in-game windows go to: below it their text is lost over the game. */
export const MIN_OVERLAY_OPACITY = 0.3;

/** The opacity setting as a fraction, or null for the theme's own (0, missing or not a number). */
export function overlayOpacity(v: number | undefined): number | null {
  if (!v || !Number.isFinite(v)) return null;
  return Math.max(MIN_OVERLAY_OPACITY, Math.min(1, v));
}

export function applySettings(s: Settings) {
  appearance.custom = (s.customThemes ?? []).map(sanitize).filter((t): t is Theme => t !== null);
  applyTheme(findTheme(s.theme, appearance.custom));
  // The in-game windows' opacity setting goes over the theme's own (0: the theme's).
  const opacity = overlayOpacity(s.overlayOpacity);
  if (opacity !== null) document.documentElement.style.setProperty('--overlay-alpha', `${Math.round(opacity * 100)}%`);
  const scale = Math.max(0.8, Math.min(1.4, s.uiScale || 1));
  appearance.scale = scale;
  // The in-game panel has its own size setting; the app zooms that window itself.
  if (location.hash === '#overlay') return;
  if (scale !== lastZoom) {
    lastZoom = scale;
    getCurrentWebview()
      .setZoom(scale)
      .catch(() => {
        /* not in the app (browser preview) */
      });
  }
}
