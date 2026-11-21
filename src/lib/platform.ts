/**
 * The system the app runs on, for what differs on macOS: the window buttons (the system's own,
 * top left), Command instead of Ctrl in shortcuts, and where things are kept.
 */
export const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform || navigator.userAgent);

/** The shortcut modifier: Command on macOS, Ctrl elsewhere (exactly that one held). */
export function modKey(e: Pick<KeyboardEvent | WheelEvent, 'ctrlKey' | 'metaKey'>, mac = isMac): boolean {
  return mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
}
