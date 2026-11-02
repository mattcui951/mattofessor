/** The active theme and text size, applied to whichever window this page runs in. */
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { BUILT_IN, cssVars, findTheme, sanitize, type Theme } from './themes';
import type { Settings } from './types';

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

export function applySettings(s: Settings) {
  appearance.custom = (s.customThemes ?? []).map(sanitize).filter((t): t is Theme => t !== null);
  applyTheme(findTheme(s.theme, appearance.custom));
  const scale = Math.max(0.8, Math.min(1.4, s.uiScale || 1));
  appearance.scale = scale;
  if (scale !== lastZoom) {
    lastZoom = scale;
    getCurrentWebview()
      .setZoom(scale)
      .catch(() => {
        /* not in the app (browser preview) */
      });
  }
}
