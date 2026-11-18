/**
 * The game's tooltip markup (Data Dragon item descriptions, augment tooltips): `<stats>`,
 * `<passive>Name</passive>`, `<magicDamage>`, `<br>`... turned into lines of plain text pieces,
 * each with a tone for its color. Nothing is ever put in the page as HTML.
 */
export type Tone = '' | 'title' | 'stat' | 'magic' | 'physical' | 'true' | 'heal' | 'defense' | 'speed' | 'crit' | 'gold' | 'key' | 'ap' | 'ad' | 'mana';
export type Seg = { text: string; tone: Tone; dim: boolean };
/** A line of pieces; an empty line is a paragraph break. */
export type RichLine = Seg[];

const TONES: Record<string, Tone> = {
  passive: 'title',
  active: 'title',
  unique: 'title',
  rarityMythic: 'title',
  rarityLegendary: 'title',
  spellName: 'title',
  titleLeft: 'title',
  stats: 'stat',
  magicDamage: 'magic',
  physicalDamage: 'physical',
  trueDamage: 'true',
  healing: 'heal',
  lifeSteal: 'heal',
  scaleHealth: 'heal',
  health: 'heal',
  shield: 'heal',
  scaleHealShield: 'heal',
  scaleArmor: 'defense',
  scaleMR: 'defense',
  speed: 'speed',
  attackSpeed: 'speed',
  scaleAS: 'speed',
  crit: 'crit',
  scaleCrit: 'crit',
  gold: 'gold',
  scaleAP: 'ap',
  scaleAD: 'ad',
  scaleAF: 'ad',
  scaleMana: 'mana',
  scaleLevel: 'key',
  attention: 'key',
  keyword: 'key',
  keywordMajor: 'key',
  keywordStealth: 'key',
  status: 'key',
  OnHit: 'key',
  font: 'key',
  buffedStat: 'key',
};
/** Small print: rules, flavor text, footnotes. */
const DIM = new Set(['rules', 'flavorText', 'postScriptLeft', 'postScriptRight', 'postScript']);

const ENTITIES: Record<string, string> = { '&nbsp;': ' ', '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&#39;': "'" };

export function parseRich(markup: string): RichLine[] {
  const lines: RichLine[] = [[]];
  const stack: string[] = [];
  const push = (text: string) => {
    const t = text.replace(/&[a-z#0-9]+;/gi, (e) => ENTITIES[e.toLowerCase()] ?? '');
    if (!t) return;
    const tone = [...stack].reverse().map((s) => TONES[s]).find(Boolean) ?? '';
    const dim = stack.some((s) => DIM.has(s));
    const line = lines[lines.length - 1];
    const last = line[line.length - 1];
    if (last && last.tone === tone && last.dim === dim) last.text += t;
    else line.push({ text: t, tone, dim });
  };
  const re = /<(\/?)([a-zA-Z]+)[^>]*?(\/?)>/g;
  let at = 0;
  for (const m of markup.matchAll(re)) {
    push(markup.slice(at, m.index));
    at = m.index! + m[0].length;
    const [, close, name, self] = m;
    if (/^(br|li)$/i.test(name)) {
      if (!close) lines.push([]);
      continue;
    }
    if (self) continue;
    if (close) {
      const i = stack.lastIndexOf(name);
      if (i >= 0) stack.length = i;
      // A title ends its line in Data Dragon's markup (<passive>Name</passive><br>text).
    } else stack.push(name);
  }
  push(markup.slice(at));
  // Trim each line, drop leading and trailing blank lines, keep one blank line between paragraphs.
  const out: RichLine[] = [];
  for (const l of lines) {
    if (l.length) {
      l[0].text = l[0].text.replace(/^\s+/, '');
      l[l.length - 1].text = l[l.length - 1].text.replace(/\s+$/, '');
    }
    const kept = l.filter((s) => s.text);
    if (!kept.length && (!out.length || !out[out.length - 1].length)) continue;
    out.push(kept);
  }
  while (out.length && !out[out.length - 1].length) out.pop();
  return out;
}
