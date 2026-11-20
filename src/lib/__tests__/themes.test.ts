import { describe, expect, it } from 'vitest';
import { ACCENT_KEYS, BUILT_IN, contrast, copyOf, cssVars, ensureContrast, inkOn, mix, parseHex, rarityColors, resolve, sanitize, toHsl } from '../themes';

/** Hue (0-360) and saturation (0-1) of a hex color. */
const hs = (hex: string) => toHsl(hex).slice(0, 2) as [number, number];

const dist = (a: string, b: string) => Math.hypot(...parseHex(a)!.map((v, i) => v - parseHex(b)![i]));

describe('color math', () => {
  it('computes WCAG contrast', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 0);
    expect(contrast('#777777', '#777777')).toBeCloseTo(1, 5);
  });

  it('mixes and nudges toward a target until readable', () => {
    expect(mix('#000000', '#ffffff', 0.5)).toBe('#808080');
    const fixed = ensureContrast('#f6c177', '#faf4ed', 3, '#464261');
    expect(contrast(fixed, '#faf4ed')).toBeGreaterThanOrEqual(3);
    expect(ensureContrast('#ffffff', '#000000', 4.5, '#ff0000')).toBe('#ffffff');
  });

  it('picks button ink by contrast', () => {
    expect(inkOn('#3fe0f0')).toBe('#0b0b12');
    expect(inkOn('#0040c8')).toBe('#ffffff');
  });
});

describe('built-in themes', () => {
  it('have unique ids and every group', () => {
    const ids = BUILT_IN.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const g of ['dark', 'light', 'contrast']) expect(BUILT_IN.some((t) => t.group === g)).toBe(true);
    expect(ids).toContain('rose-pine-moon');
  });

  for (const t of BUILT_IN) {
    it(`${t.name} is readable`, () => {
      const c = resolve(t);
      for (const surface of [c.bg, c.panel]) {
        expect(contrast(c.text, surface), 'text').toBeGreaterThanOrEqual(4.5);
        expect(contrast(c.text2, surface), 'text2').toBeGreaterThanOrEqual(4.5);
        expect(contrast(c.text3, surface), 'text3').toBeGreaterThanOrEqual(4.5);
        for (const k of ACCENT_KEYS) expect(contrast(c[k], surface), k).toBeGreaterThanOrEqual(3);
      }
      // Primary buttons: ink on the accent.
      expect(contrast(inkOn(c.you), c.you)).toBeGreaterThanOrEqual(4.5);
      // Dividers and raised surfaces must be visible against the ground.
      expect(contrast(c.seamStrong, c.bg)).toBeGreaterThan(1.3);
      expect(c.panel3).not.toBe(c.bg);
      const vars = cssVars(t);
      expect(vars['color-scheme']).toBe(t.dark ? 'dark' : 'light');
    });

    it(`${t.name} shows silver, gold and prismatic as themselves`, () => {
      const c = resolve(t);
      const r = rarityColors(t);
      for (const surface of [c.bg, c.panel, c.panel3]) {
        for (const hex of [r.silver, r.gold, ...r.prismaticStops]) expect(contrast(hex, surface)).toBeGreaterThanOrEqual(3);
      }
      expect(hs(r.silver)[1], 'silver is grey').toBeLessThan(0.2);
      const [goldHue, goldSat] = hs(r.gold);
      expect(goldHue, 'gold hue').toBeGreaterThanOrEqual(38);
      expect(goldHue, 'gold hue').toBeLessThanOrEqual(50);
      expect(goldSat, 'gold is saturated').toBeGreaterThan(0.6);
      // The rainbow runs pink, violet, blue.
      const hues = r.prismaticStops.map((x) => hs(x)[0]);
      expect(hues[0]).toBeGreaterThan(300);
      expect(hues[1]).toBeGreaterThan(240);
      expect(hues[2]).toBeLessThan(210);
      // Silver is grey and the other two are vivid, so silver never passes for either.
      for (const x of r.prismaticStops) expect(hs(x)[1], `${x} is vivid`).toBeGreaterThan(0.6);
      for (const x of [r.silver, ...r.prismaticStops]) expect(dist(r.gold, x), `gold vs ${x}`).toBeGreaterThan(90);
    });
  }
});

describe('custom themes', () => {
  it('copies into an editable theme with a new id', () => {
    const moon = BUILT_IN.find((t) => t.id === 'rose-pine-moon')!;
    const c = copyOf(moon, 'Mine', []);
    expect(c.group).toBe('custom');
    expect(c.id).not.toBe(moon.id);
    c.colors.bg = '#000000';
    expect(moon.colors.bg).toBe('#232136');
  });

  it('sanitizes imports', () => {
    expect(sanitize(null)).toBeNull();
    const t = sanitize({ name: '  Neon  ', colors: { bg: '#fff', text: 'not a color', you: '#FF00AA' }, radius: 99, overlayOpacity: 0.1 })!;
    expect(t.name).toBe('Neon');
    expect(t.colors.bg).toBe('#ffffff');
    expect(t.colors.text).toBe(BUILT_IN[0].colors.text);
    expect(t.colors.you).toBe('#ff00aa');
    expect(t.radius).toBe(16);
    expect(t.overlayOpacity).toBe(0.6);
    expect(t.dark).toBe(false);
    expect(t.group).toBe('custom');
  });

  it('keeps fonts free of CSS injection', () => {
    const t = { ...BUILT_IN[0], fontUi: 'Evil"; } body { color: red' };
    expect(cssVars(t)['--font-ui']).not.toMatch(/[;{}]/);
  });
});
