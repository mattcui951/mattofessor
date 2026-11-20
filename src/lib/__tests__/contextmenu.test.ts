import { describe, expect, it } from 'vitest';
import { wantsMenu } from '../contextmenu';

describe('right-click menu', () => {
  it('never shows in the in-game windows, not even on a text field', () => {
    expect(wantsMenu(null, true)).toBe(false);
    expect(wantsMenu({ tag: 'INPUT', type: 'text' }, true)).toBe(false);
    expect(wantsMenu({ tag: 'TEXTAREA' }, true)).toBe(false);
  });

  it('shows in the main window only on text fields', () => {
    expect(wantsMenu(null, false)).toBe(false);
    expect(wantsMenu({ tag: 'INPUT', type: 'text' }, false)).toBe(true);
    expect(wantsMenu({ tag: 'INPUT', type: 'password' }, false)).toBe(true);
    expect(wantsMenu({ tag: 'INPUT', type: 'search' }, false)).toBe(true);
    expect(wantsMenu({ tag: 'TEXTAREA' }, false)).toBe(true);
    expect(wantsMenu({ tag: 'DIV', editable: true }, false)).toBe(true);
  });

  it('does not show on inputs that hold no text', () => {
    for (const type of ['checkbox', 'radio', 'range', 'button', 'color']) {
      expect(wantsMenu({ tag: 'INPUT', type }, false)).toBe(false);
    }
    expect(wantsMenu({ tag: 'DIV', editable: false }, false)).toBe(false);
  });
});
