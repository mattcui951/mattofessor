import { describe, expect, it } from 'vitest';
import { LOCALES, pick } from '../locales';
import rust from '../../../src-tauri/src/lang.rs?raw';

describe('locales', () => {
  it('match the Rust list (src-tauri/src/lang.rs)', () => {
    const ids = [...rust.matchAll(/Lang \{ id: "([^"]+)"/g)].map((m) => m[1]);
    expect(ids).toEqual(LOCALES.map((l) => l.id));
  });

  it('pick a language, English or nothing', () => {
    const t = { en: 'Shield', zh: '护盾' };
    expect(pick(t, 'zh')).toBe('护盾');
    expect(pick(t, 'ja')).toBe('Shield');
    expect(pick(t, 'ja', true)).toBe('');
    expect(pick(t, 'en', true)).toBe('Shield');
    expect(pick(undefined, 'fr')).toBe('');
  });
});
