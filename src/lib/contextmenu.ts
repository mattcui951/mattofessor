/**
 * The web view's own right-click menu (Back, Refresh, Save as, Print...) means nothing in this
 * app, and over the game it gets in the way: a right-click meant to move the champion that lands
 * on an in-game window opened it, and picking anything but Refresh did something unwanted.
 *
 * In-game windows never show it. The main window shows it only on text fields, for cut, copy and
 * paste (a Riot ID, the API key). The app's own right-click handlers (the spell timers' reset)
 * still run: this only stops the default menu.
 */

/** The text field a right-click landed in, if any: its tag and, for an input, its type. */
export type Field = { tag: string; type?: string; editable?: boolean } | null;

const NOT_TEXT = new Set(['button', 'checkbox', 'radio', 'range', 'color', 'file', 'submit', 'reset', 'image', 'hidden']);

export function wantsMenu(field: Field, overlay: boolean): boolean {
  if (overlay || !field) return false;
  const tag = field.tag.toLowerCase();
  if (tag === 'input') return !NOT_TEXT.has((field.type ?? 'text').toLowerCase());
  return tag === 'textarea' || field.editable === true;
}

function fieldOf(target: EventTarget | null): Field {
  if (!(target instanceof Element)) return null;
  const el = target.closest('input, textarea, [contenteditable]');
  if (!el) return null;
  return {
    tag: el.tagName,
    type: el instanceof HTMLInputElement ? el.type : undefined,
    editable: el instanceof HTMLElement && el.isContentEditable,
  };
}

/** Stops the default menu everywhere it is not wanted (`wantsMenu`). */
export function blockContextMenu(overlay: boolean) {
  window.addEventListener(
    'contextmenu',
    (e) => {
      if (!wantsMenu(fieldOf(e.target), overlay)) e.preventDefault();
    },
    { capture: true },
  );
}
