/**
 * Themes: every color, font and corner radius in the UI comes from these tokens.
 *
 * Built-in palettes follow each theme's published colors. Each theme maps them onto the same
 * roles (the Arcade theme's meanings): `you` = primary / you / kills, `death` = deaths and
 * danger, `objective` = gold / info tags, `win` = wins and positive, `assist` = assists and
 * premades, `warn` = warnings. With "readable" on (default), secondary text and accents are
 * nudged toward the text color where a palette is too faint on its background
 * (WCAG 4.5:1 for small text, 3:1 for accents), so light themes stay legible.
 */

export type ThemeColors = {
  bg: string;
  bgSunk: string;
  panel: string;
  panel2: string;
  panel3: string;
  seam: string;
  seamStrong: string;
  text: string;
  text2: string;
  text3: string;
  you: string;
  death: string;
  objective: string;
  win: string;
  assist: string;
  warn: string;
};

export type ThemeGroup = 'dark' | 'light' | 'contrast' | 'custom';

export type Theme = {
  id: string;
  name: string;
  group: ThemeGroup;
  dark: boolean;
  colors: ThemeColors;
  /** Body text font family (a name; fallbacks are added). */
  fontUi: string;
  /** Numbers, headings and labels. */
  fontScore: string;
  /** Largest corner radius in px (panels); buttons and chips scale from it. */
  radius: number;
  /** In-game windows' background opacity, 0.6-1. */
  overlayOpacity: number;
  /** Nudge faint text and accents to readable contrast. */
  readable: boolean;
};

export const COLOR_KEYS: (keyof ThemeColors)[] = [
  'bg',
  'bgSunk',
  'panel',
  'panel2',
  'panel3',
  'seam',
  'seamStrong',
  'text',
  'text2',
  'text3',
  'you',
  'death',
  'objective',
  'win',
  'assist',
  'warn',
];

export const ACCENT_KEYS: (keyof ThemeColors)[] = ['you', 'death', 'objective', 'win', 'assist', 'warn'];

/** Fonts offered in the picker. `bundled` ones ship with the app; the rest come with Windows. */
export const UI_FONTS = [
  { id: 'Segoe UI Variable Text', label: 'Segoe UI', bundled: false },
  { id: 'Inter', label: 'Inter', bundled: true },
  { id: 'Microsoft YaHei UI', label: 'Microsoft YaHei UI (微软雅黑)', bundled: false },
  { id: 'Bahnschrift', label: 'Bahnschrift', bundled: false },
  { id: 'Verdana', label: 'Verdana', bundled: false },
  { id: 'JetBrains Mono', label: 'JetBrains Mono', bundled: true },
  { id: 'Consolas', label: 'Consolas', bundled: false },
];

export const SCORE_FONTS = [
  { id: 'Chakra Petch', label: 'Chakra Petch', bundled: true },
  { id: 'Inter', label: 'Inter', bundled: true },
  { id: 'Bahnschrift', label: 'Bahnschrift', bundled: false },
  { id: 'Segoe UI Variable Display', label: 'Segoe UI', bundled: false },
  { id: 'JetBrains Mono', label: 'JetBrains Mono', bundled: true },
  { id: 'Consolas', label: 'Consolas', bundled: false },
];

const DEFAULT_UI = 'Segoe UI Variable Text';
const DEFAULT_SCORE = 'Chakra Petch';

type Def = Omit<Theme, 'fontUi' | 'fontScore' | 'radius' | 'overlayOpacity' | 'readable'> & Partial<Theme>;

function theme(d: Def): Theme {
  return { fontUi: DEFAULT_UI, fontScore: DEFAULT_SCORE, radius: 6, overlayOpacity: 0.97, readable: true, ...d };
}

const c = (
  bg: string,
  bgSunk: string,
  panel: string,
  panel2: string,
  panel3: string,
  seam: string,
  seamStrong: string,
  text: string,
  text2: string,
  text3: string,
  you: string,
  death: string,
  objective: string,
  win: string,
  assist: string,
  warn: string,
): ThemeColors => ({ bg, bgSunk, panel, panel2, panel3, seam, seamStrong, text, text2, text3, you, death, objective, win, assist, warn });

//                         bg         sunk       panel      panel2     panel3     seam       seamStrong text       text2      text3      you        death      objective  win        assist     warn
export const BUILT_IN: Theme[] = [
  theme({ id: 'arcade', name: 'Arcade', group: 'dark', dark: true,
    colors: c('#100d26', '#0b0920', '#161238', '#1d1847', '#262058', '#2a2461', '#3b3485', '#eeebff', '#bdb6e6', '#908ac2', '#3fe0f0', '#ff4f8b', '#ffc83d', '#52f0a8', '#b197ff', '#ffa53d') }),
  theme({ id: 'rose-pine', name: 'Rosé Pine', group: 'dark', dark: true,
    colors: c('#191724', '#13111d', '#1f1d2e', '#26233a', '#2f2c44', '#403d52', '#524f67', '#e0def4', '#b8b5cf', '#908caa', '#c4a7e7', '#eb6f92', '#f6c177', '#9ccfd8', '#ebbcba', '#f6c177') }),
  theme({ id: 'rose-pine-moon', name: 'Rosé Pine Moon', group: 'dark', dark: true,
    colors: c('#232136', '#1c1a2d', '#2a273f', '#393552', '#44415a', '#44415a', '#56526e', '#e0def4', '#b8b5cf', '#908caa', '#c4a7e7', '#eb6f92', '#f6c177', '#9ccfd8', '#ea9a97', '#f6c177') }),
  theme({ id: 'catppuccin-mocha', name: 'Catppuccin Mocha', group: 'dark', dark: true,
    colors: c('#1e1e2e', '#181825', '#262637', '#313244', '#45475a', '#45475a', '#585b70', '#cdd6f4', '#bac2de', '#a6adc8', '#89b4fa', '#f38ba8', '#f9e2af', '#a6e3a1', '#cba6f7', '#fab387') }),
  theme({ id: 'catppuccin-macchiato', name: 'Catppuccin Macchiato', group: 'dark', dark: true,
    colors: c('#24273a', '#1e2030', '#2b2f45', '#363a4f', '#494d64', '#494d64', '#5b6078', '#cad3f5', '#b8c0e0', '#a5adcb', '#8aadf4', '#ed8796', '#eed49f', '#a6da95', '#c6a0f6', '#f5a97f') }),
  theme({ id: 'catppuccin-frappe', name: 'Catppuccin Frappé', group: 'dark', dark: true,
    colors: c('#303446', '#292c3c', '#383c50', '#414559', '#51576d', '#51576d', '#626880', '#c6d0f5', '#b5bfe2', '#a5adce', '#8caaee', '#e78284', '#e5c890', '#a6d189', '#ca9ee6', '#ef9f76') }),
  theme({ id: 'tokyo-night', name: 'Tokyo Night', group: 'dark', dark: true,
    colors: c('#1a1b26', '#16161e', '#1f2335', '#292e42', '#343a55', '#292e42', '#414868', '#c0caf5', '#a9b1d6', '#737aa2', '#7aa2f7', '#f7768e', '#e0af68', '#9ece6a', '#bb9af7', '#ff9e64') }),
  theme({ id: 'tokyo-night-storm', name: 'Tokyo Night Storm', group: 'dark', dark: true,
    colors: c('#24283b', '#1f2335', '#292e42', '#2f3549', '#3b4261', '#3b4261', '#545c7e', '#c0caf5', '#a9b1d6', '#737aa2', '#7aa2f7', '#f7768e', '#e0af68', '#9ece6a', '#bb9af7', '#ff9e64') }),
  theme({ id: 'dracula', name: 'Dracula', group: 'dark', dark: true,
    colors: c('#282a36', '#21222c', '#2e303e', '#373a4b', '#44475a', '#44475a', '#6272a4', '#f8f8f2', '#d6d6d0', '#6272a4', '#8be9fd', '#ff5555', '#f1fa8c', '#50fa7b', '#bd93f9', '#ffb86c') }),
  theme({ id: 'nord', name: 'Nord', group: 'dark', dark: true,
    colors: c('#2e3440', '#292e39', '#3b4252', '#434c5e', '#4c566a', '#434c5e', '#4c566a', '#eceff4', '#d8dee9', '#a7b1c2', '#88c0d0', '#bf616a', '#ebcb8b', '#a3be8c', '#b48ead', '#d08770') }),
  theme({ id: 'gruvbox-dark', name: 'Gruvbox Dark', group: 'dark', dark: true,
    colors: c('#282828', '#1d2021', '#32302f', '#3c3836', '#504945', '#3c3836', '#665c54', '#ebdbb2', '#d5c4a1', '#a89984', '#83a598', '#fb4934', '#fabd2f', '#b8bb26', '#d3869b', '#fe8019') }),
  theme({ id: 'one-dark', name: 'One Dark', group: 'dark', dark: true,
    colors: c('#282c34', '#21252b', '#2c313a', '#333842', '#3e4451', '#3e4451', '#4b5263', '#dcdfe4', '#abb2bf', '#5c6370', '#61afef', '#e06c75', '#e5c07b', '#98c379', '#c678dd', '#d19a66') }),
  theme({ id: 'kanagawa', name: 'Kanagawa', group: 'dark', dark: true,
    colors: c('#1f1f28', '#16161d', '#2a2a37', '#363646', '#43435a', '#363646', '#54546d', '#dcd7ba', '#c8c093', '#727169', '#7e9cd8', '#e46876', '#e6c384', '#98bb6c', '#957fb8', '#ffa066') }),
  theme({ id: 'everforest-dark', name: 'Everforest Dark', group: 'dark', dark: true,
    colors: c('#2d353b', '#232a2e', '#343f44', '#3d484d', '#475258', '#475258', '#56635f', '#d3c6aa', '#c5b99d', '#9da9a0', '#7fbbb3', '#e67e80', '#dbbc7f', '#a7c080', '#d699b6', '#e69875') }),
  theme({ id: 'solarized-dark', name: 'Solarized Dark', group: 'dark', dark: true,
    colors: c('#002b36', '#00212b', '#073642', '#0a3f4c', '#0e4a59', '#0e4a59', '#586e75', '#eee8d5', '#93a1a1', '#839496', '#268bd2', '#dc322f', '#b58900', '#859900', '#6c71c4', '#cb4b16') }),
  theme({ id: 'github-dark', name: 'GitHub Dark', group: 'dark', dark: true,
    colors: c('#0d1117', '#010409', '#161b22', '#1c2128', '#262c36', '#30363d', '#484f58', '#e6edf3', '#c9d1d9', '#8d96a0', '#58a6ff', '#f85149', '#d29922', '#3fb950', '#a371f7', '#db6d28') }),

  theme({ id: 'rose-pine-dawn', name: 'Rosé Pine Dawn', group: 'light', dark: false,
    colors: c('#faf4ed', '#f2e9e1', '#fffaf3', '#f4ede8', '#dfdad9', '#dfdad9', '#cecacd', '#464261', '#575279', '#797593', '#907aa9', '#b4637a', '#ea9d34', '#286983', '#d7827e', '#ea9d34') }),
  theme({ id: 'catppuccin-latte', name: 'Catppuccin Latte', group: 'light', dark: false,
    colors: c('#eff1f5', '#e6e9ef', '#f7f8fa', '#e6e9ef', '#dce0e8', '#ccd0da', '#bcc0cc', '#4c4f69', '#5c5f77', '#6c6f85', '#1e66f5', '#d20f39', '#df8e1d', '#40a02b', '#8839ef', '#fe640b') }),
  theme({ id: 'tokyo-night-day', name: 'Tokyo Night Day', group: 'light', dark: false,
    colors: c('#e1e2e7', '#d0d5e3', '#e9e9ed', '#d6d8df', '#c4c8da', '#c4c8da', '#a8aecb', '#3760bf', '#4a5a8c', '#6172b0', '#2e7de9', '#f52a65', '#8c6c3e', '#587539', '#9854f1', '#b15c00') }),
  theme({ id: 'gruvbox-light', name: 'Gruvbox Light', group: 'light', dark: false,
    colors: c('#fbf1c7', '#f2e5bc', '#f9f5d7', '#ebdbb2', '#d5c4a1', '#d5c4a1', '#bdae93', '#3c3836', '#504945', '#7c6f64', '#076678', '#9d0006', '#b57614', '#79740e', '#8f3f71', '#af3a03') }),
  theme({ id: 'everforest-light', name: 'Everforest Light', group: 'light', dark: false,
    colors: c('#fdf6e3', '#efebd4', '#f4f0d9', '#efebd4', '#e6e2cc', '#e0dcc7', '#bdc3af', '#5c6a72', '#6b7a82', '#829181', '#3a94c5', '#f85552', '#dfa000', '#8da101', '#df69ba', '#f57d26') }),
  theme({ id: 'solarized-light', name: 'Solarized Light', group: 'light', dark: false,
    colors: c('#fdf6e3', '#eee8d5', '#fffbf0', '#eee8d5', '#e4ddc8', '#e4ddc8', '#cfc8b2', '#073642', '#586e75', '#657b83', '#268bd2', '#dc322f', '#b58900', '#859900', '#6c71c4', '#cb4b16') }),
  theme({ id: 'github-light', name: 'GitHub Light', group: 'light', dark: false,
    colors: c('#ffffff', '#f6f8fa', '#f6f8fa', '#eaeef2', '#dde3ea', '#d0d7de', '#afb8c1', '#1f2328', '#424a53', '#59636e', '#0969da', '#cf222e', '#9a6700', '#1a7f37', '#8250df', '#bc4c00') }),

  theme({ id: 'high-contrast-dark', name: 'High Contrast Dark', group: 'contrast', dark: true, radius: 3, overlayOpacity: 1,
    colors: c('#000000', '#000000', '#0b0b0b', '#1c1c1c', '#2b2b2b', '#9a9a9a', '#ffffff', '#ffffff', '#ffffff', '#e6e6e6', '#00e5ff', '#ff6b6b', '#ffff00', '#3dff7a', '#e2b0ff', '#ffab40') }),
  theme({ id: 'high-contrast-light', name: 'High Contrast Light', group: 'contrast', dark: false, radius: 3, overlayOpacity: 1,
    colors: c('#ffffff', '#f0f0f0', '#ffffff', '#ececec', '#d9d9d9', '#595959', '#000000', '#000000', '#111111', '#2e2e2e', '#0040c8', '#b00020', '#6e5100', '#006b2b', '#6a1b9a', '#9c4100') }),
];

export const DEFAULT_THEME = 'arcade';

// ---------- color math ----------

export function parseHex(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6}|[0-9a-f]{3})$/i.exec(hex.trim());
  if (!m) return null;
  const h = m[1].length === 3 ? [...m[1]].map((ch) => ch + ch).join('') : m[1];
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

export function toHex([r, g, b]: [number, number, number]): string {
  return '#' + [r, g, b].map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0')).join('');
}

/** Linear blend: t=0 gives a, t=1 gives b. */
export function mix(a: string, b: string, t: number): string {
  const x = parseHex(a) ?? [0, 0, 0];
  const y = parseHex(b) ?? [0, 0, 0];
  return toHex([x[0] + (y[0] - x[0]) * t, x[1] + (y[1] - x[1]) * t, x[2] + (y[2] - x[2]) * t]);
}

function luminance(hex: string): number {
  const rgb = parseHex(hex) ?? [0, 0, 0];
  const [r, g, b] = rgb.map((v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio, 1-21. */
export function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** Move `fg` toward `toward` just enough to reach `min` contrast against `bg`. */
export function ensureContrast(fg: string, bg: string, min: number, toward: string): string {
  if (contrast(fg, bg) >= min) return fg;
  for (let t = 0.05; t <= 1.0001; t += 0.05) {
    const c2 = mix(fg, toward, t);
    if (contrast(c2, bg) >= min) return c2;
  }
  return toward;
}

/** Colors as shown: the theme's own, made readable when `readable` is on. */
export function resolve(t: Theme): ThemeColors {
  const out = { ...t.colors };
  if (!t.readable) return out;
  // Check against the lighter (dark themes) or darker (light themes) of bg and panel: the harder case.
  const surfaces = [out.bg, out.panel];
  const worst = (fg: string) => surfaces.reduce((w, s) => (contrast(fg, s) < contrast(fg, w) ? s : w), surfaces[0]);
  out.text2 = ensureContrast(out.text2, worst(out.text2), 4.5, out.text);
  out.text3 = ensureContrast(out.text3, worst(out.text3), 4.5, out.text);
  for (const k of ACCENT_KEYS) out[k] = ensureContrast(out[k], worst(out[k]), 3, out.text);
  return out;
}

/** Ink for text on a primary (`you`) button: whichever of near-black / near-white reads better. */
export function inkOn(hex: string): string {
  return contrast(hex, '#0b0b12') >= contrast(hex, '#ffffff') ? '#0b0b12' : '#ffffff';
}

function stack(font: string, fallback: string): string {
  const name = font.trim().replace(/["';{}]/g, '');
  return `${name ? `"${name}", ` : ''}${fallback}`;
}

const CSS_VARS: Record<keyof ThemeColors, string> = {
  bg: '--bg',
  bgSunk: '--bg-sunk',
  panel: '--panel',
  panel2: '--panel-2',
  panel3: '--panel-3',
  seam: '--seam',
  seamStrong: '--seam-strong',
  text: '--text',
  text2: '--text-2',
  text3: '--text-3',
  you: '--you',
  death: '--death',
  objective: '--objective',
  win: '--win',
  assist: '--assist',
  warn: '--warn',
};

/** CSS custom properties for a theme. */
export function cssVars(t: Theme): Record<string, string> {
  const col = resolve(t);
  const vars: Record<string, string> = {};
  for (const k of COLOR_KEYS) vars[CSS_VARS[k]] = col[k];
  vars['--you-ink'] = inkOn(col.you);
  vars['--mark'] = col.text;
  vars['--font-ui'] = stack(t.fontUi, "'Segoe UI', 'Microsoft YaHei UI', system-ui, sans-serif");
  vars['--font-score'] = stack(t.fontScore, "'Segoe UI', 'Microsoft YaHei UI', sans-serif");
  const r = Math.max(0, Math.min(16, t.radius));
  vars['--r-lg'] = `${r}px`;
  vars['--r-md'] = `${Math.round((r * 2) / 3)}px`;
  vars['--r-sm'] = `${Math.round(r / 2)}px`;
  vars['--overlay-alpha'] = `${Math.round(Math.max(0.6, Math.min(1, t.overlayOpacity)) * 100)}%`;
  vars['--shadow-pop'] = t.dark
    ? '0 10px 28px -6px rgb(0 0 0 / 0.6), 0 2px 6px rgb(0 0 0 / 0.45)'
    : '0 10px 28px -8px rgb(20 20 40 / 0.22), 0 2px 6px rgb(20 20 40 / 0.12)';
  vars['color-scheme'] = t.dark ? 'dark' : 'light';
  return vars;
}

/** Theme by id among built-ins and the user's custom themes; Arcade when unknown. */
export function findTheme(id: string, custom: Theme[]): Theme {
  return custom.find((t) => t.id === id) ?? BUILT_IN.find((t) => t.id === id) ?? BUILT_IN[0];
}

/** A custom copy of a theme, ready to edit. */
export function copyOf(t: Theme, name: string, taken: string[]): Theme {
  let id = `custom-${Date.now().toString(36)}`;
  while (taken.includes(id)) id += 'x';
  // JSON copy: the theme may be a reactive proxy, which structuredClone cannot copy.
  return { ...(JSON.parse(JSON.stringify(t)) as Theme), id, name, group: 'custom' };
}

/** Validate an imported / stored theme; fills anything missing from Arcade. */
export function sanitize(v: unknown): Theme | null {
  if (!v || typeof v !== 'object') return null;
  const o = v as Partial<Theme> & { colors?: Partial<ThemeColors> };
  const base = BUILT_IN[0];
  const colors = { ...base.colors };
  for (const k of COLOR_KEYS) {
    const val = o.colors?.[k];
    if (typeof val === 'string' && parseHex(val)) colors[k] = toHex(parseHex(val)!);
  }
  const num = (x: unknown, d: number) => (typeof x === 'number' && Number.isFinite(x) ? x : d);
  return {
    id: typeof o.id === 'string' && o.id ? o.id : `custom-${Date.now().toString(36)}`,
    name: typeof o.name === 'string' && o.name.trim() ? o.name.trim().slice(0, 40) : 'Imported theme',
    group: 'custom',
    dark: typeof o.dark === 'boolean' ? o.dark : luminance(colors.bg) < 0.4,
    colors,
    fontUi: typeof o.fontUi === 'string' ? o.fontUi.slice(0, 60) : base.fontUi,
    fontScore: typeof o.fontScore === 'string' ? o.fontScore.slice(0, 60) : base.fontScore,
    radius: Math.max(0, Math.min(16, num(o.radius, base.radius))),
    overlayOpacity: Math.max(0.6, Math.min(1, num(o.overlayOpacity, base.overlayOpacity))),
    readable: typeof o.readable === 'boolean' ? o.readable : true,
  };
}

/** Tier colors are tuned for dark grounds; light themes darken them toward the text color. */
export function readableOn(hex: string, t: Theme): string {
  const col = resolve(t);
  return ensureContrast(hex, col.bg, 3, col.text);
}
