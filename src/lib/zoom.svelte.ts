/**
 * Zoom shortcuts for the main window: Ctrl + / Ctrl - / Ctrl 0 and Ctrl + mouse wheel (or a
 * touchpad pinch). They step the saved text size (`uiScale`), so the zoom is remembered and the
 * Appearance slider follows.
 */
import { appearance } from './appearance.svelte';
import { updateSettings } from './state.svelte';

export const MIN_SCALE = 0.8;
export const MAX_SCALE = 1.4;
const STEP = 0.1;

/** The next size one step up (dir 1) or down (dir -1), on the 10% grid, within the range. */
export function stepScale(current: number, dir: 1 | -1): number {
  const tenths = current * 10;
  const next = dir > 0 ? Math.floor(tenths + 1e-6) / 10 + STEP : Math.ceil(tenths - 1e-6) / 10 - STEP;
  return Math.round(Math.max(MIN_SCALE, Math.min(MAX_SCALE, next)) * 100) / 100;
}

/** What a key press asks for: 1 bigger, -1 smaller, 0 reset, null not a zoom key. */
export function zoomKey(e: Pick<KeyboardEvent, 'key' | 'code' | 'ctrlKey' | 'altKey' | 'metaKey'>): 1 | -1 | 0 | null {
  if (!e.ctrlKey || e.altKey || e.metaKey) return null;
  if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd') return 1;
  if (e.key === '-' || e.key === '_' || e.code === 'NumpadSubtract') return -1;
  if (e.key === '0' || e.code === 'Numpad0') return 0;
  return null;
}

/** Shown briefly after each change (the badge in the corner). */
export const zoomBadge = $state({ visible: false, scale: 1 });
let hideTimer: ReturnType<typeof setTimeout> | undefined;
let hovered = false;

export function setBadgeHovered(v: boolean) {
  hovered = v;
  if (!v) scheduleHide();
}

function scheduleHide() {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    if (!hovered) zoomBadge.visible = false;
  }, 1500);
}

export function setScale(scale: number) {
  zoomBadge.scale = scale;
  zoomBadge.visible = true;
  scheduleHide();
  if (scale !== appearance.scale) void updateSettings({ uiScale: scale }, { debounce: true });
}

function zoom(dir: 1 | -1 | 0) {
  setScale(dir === 0 ? 1 : stepScale(appearance.scale, dir));
}

// A mouse wheel notch is about 100; a pinch sends many small deltas. Add them up.
const NOTCH = 100;
let wheelSum = 0;
let wheelTimer: ReturnType<typeof setTimeout> | undefined;

/** Listen on this window. Returns a function that stops listening. */
export function installZoom(target: Window = window): () => void {
  const onKey = (e: KeyboardEvent) => {
    const dir = zoomKey(e);
    if (dir === null || e.defaultPrevented) return;
    e.preventDefault();
    zoom(dir);
  };
  const onWheel = (e: WheelEvent) => {
    if (!e.ctrlKey) return;
    e.preventDefault();
    const delta = e.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? e.deltaY : e.deltaY * NOTCH;
    wheelSum += delta;
    clearTimeout(wheelTimer);
    wheelTimer = setTimeout(() => (wheelSum = 0), 300);
    // Half a notch is enough, so a real notch always counts once even when Windows sends a bit less.
    if (Math.abs(wheelSum) >= NOTCH / 2) {
      zoom(wheelSum < 0 ? 1 : -1);
      wheelSum = 0;
    }
  };
  target.addEventListener('keydown', onKey);
  target.addEventListener('wheel', onWheel, { passive: false });
  return () => {
    target.removeEventListener('keydown', onKey);
    target.removeEventListener('wheel', onWheel);
  };
}
