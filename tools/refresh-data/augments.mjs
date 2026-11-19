// Hextech ARAM augment descriptions with their numbers, in every language of the app, from the game's
// own data (CommunityDragon's copies): the mode's augment list and spells (kiwi.bin.json) and the
// string tables. A tooltip's @Name@ is a spell value or a calculation; calculations are written
// the way the game shows them: "100 - 400 (based on level) (+40% AP)". Counters the game fills
// in during a game (@f1@: damage dealt so far...) are left out with their line.

export const KIWI = 'https://raw.communitydragon.org/latest/game/maps/modespecificdata/kiwi.bin.json';

/** Stats a calculation scales with (`mStat`), as each language's client names them. */
const STATS = {
  0: { en: 'AP', zh: '法术强度', 'zh-TW': '魔法攻擊', ja: '魔力', ko: '주문력', es: 'poder de habilidad', fr: 'puissance', ru: 'силы умений' },
  1: { en: 'Armor', zh: '护甲', 'zh-TW': '物理防禦', ja: '物理防御', ko: '방어력', es: 'armadura', fr: 'armure', ru: 'брони' },
  2: { en: 'AD', zh: '攻击力', 'zh-TW': '攻擊力', ja: '攻撃力', ko: '공격력', es: 'daño de ataque', fr: "dégâts d'attaque", ru: 'силы атаки' },
  4: { en: 'Attack Speed', zh: '攻击速度', 'zh-TW': '攻擊速度', ja: '攻撃速度', ko: '공격 속도', es: 'velocidad de ataque', fr: "vitesse d'attaque", ru: 'скорости атаки' },
  6: { en: 'Magic Resist', zh: '魔法抗性', 'zh-TW': '魔法防禦', ja: '魔法防御', ko: '마법 저항력', es: 'resistencia mágica', fr: 'résistance magique', ru: 'сопротивления магии' },
  7: { en: 'Move Speed', zh: '移动速度', 'zh-TW': '移動速度', ja: '移動速度', ko: '이동 속도', es: 'velocidad de movimiento', fr: 'vitesse de déplacement', ru: 'скорости передвижения' },
  8: { en: 'Crit Chance', zh: '暴击几率', 'zh-TW': '暴擊率', ja: 'クリティカル率', ko: '치명타 확률', es: 'probabilidad de crítico', fr: 'chances de coup critique', ru: 'шанса критического удара' },
  9: { en: 'Crit Damage', zh: '暴击伤害', 'zh-TW': '暴擊傷害', ja: 'クリティカルダメージ', ko: '치명타 피해량', es: 'daño crítico', fr: 'dégâts critiques', ru: 'критического урона' },
  11: { en: 'Ability Haste', zh: '技能急速', 'zh-TW': '技能加速', ja: 'スキルヘイスト', ko: '스킬 가속', es: 'velocidad de habilidades', fr: 'accélération de compétence', ru: 'ускорения умений' },
  12: { en: 'max Health', zh: '最大生命值', 'zh-TW': '最大生命', ja: '最大体力', ko: '최대 체력', es: 'vida máxima', fr: 'PV max', ru: 'макс. здоровья' },
  34: { en: 'Heal and Shield Power', zh: '治疗和护盾强度', 'zh-TW': '治療和護盾量', ja: 'ヒールとシールドの効果', ko: '체력 회복 및 보호막 효과', es: 'poder de curaciones y escudos', fr: 'puissance de soins et boucliers', ru: 'силы лечения и щитов' },
};

/** The languages written (ids of `src/lib/locales.ts`). */
export const AUGMENT_LANGS = ['en', 'zh', 'zh-TW', 'ja', 'ko', 'es', 'fr', 'ru'];

/** The game's hash of a name it stores hashed (`{0123abcd}`): 32-bit FNV-1a of the lower case. */
export function hashName(name) {
  let h = 0x811c9dc5;
  for (const c of name.toLowerCase()) {
    h ^= c.charCodeAt(0);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return `{${h.toString(16).padStart(8, '0')}}`;
}

/** A spell's value or calculation by name, stored plain or hashed. */
const lookup = (map, name) => map[name] ?? map[hashName(name)];
/**
 * Each language's wording: `scale(pct, stat, kind)` writes one scaling ("40% bonus AD",
 * "增加攻撃力の40%", "40% de daño de ataque adicional"), `kind` being "bonus", "base" or "".
 */
const WORDS = {
  en: { level: ' (based on level)', bonus: 'bonus ', base: 'base ', scale: (p, n, k) => `${p} ${k}${n}`, stack: (v) => ` (+${v} per stack)`, mana: 'max Mana', ranged: (p) => ` (${p} for ranged)`, spell: 'chosen ability', open: ' (+', close: ')' },
  zh: { level: '（基于等级）', bonus: '额外', base: '基础', scale: (p, n, k) => `${p}${k}${n}`, stack: (v) => `（每层+${v}）`, mana: '最大法力值', ranged: (p) => `（远程英雄为${p}）`, spell: '指定', open: '（+', close: '）' },
  'zh-TW': { level: '（依等級）', bonus: '額外', base: '基礎', scale: (p, n, k) => `${p}${k}${n}`, stack: (v) => `（每層+${v}）`, mana: '最大魔力', ranged: (p) => `（遠程英雄為${p}）`, spell: '指定', open: '（+', close: '）' },
  ja: { level: '（レベルに応じて）', bonus: '増加', base: '基本', scale: (p, n, k) => `${k}${n}の${p}`, stack: (v) => `（1スタックごとに+${v}）`, mana: '最大マナ', ranged: (p) => `（遠隔攻撃チャンピオンは${p}）`, spell: '指定スキル', open: '（+', close: '）' },
  ko: { level: ' (레벨에 따라)', bonus: '추가 ', base: '기본 ', scale: (p, n, k) => `${k}${n}의 ${p}`, stack: (v) => ` (중첩당 +${v})`, mana: '최대 마나', ranged: (p) => ` (원거리 챔피언은 ${p})`, spell: '지정한 스킬', open: ' (+', close: ')' },
  es: { level: ' (según el nivel)', bonus: ' adicional', base: ' base', scale: (p, n, k) => `${p} de ${n}${k}`, stack: (v) => ` (+${v} por acumulación)`, mana: 'maná máximo', ranged: (p) => ` (${p} a distancia)`, spell: 'habilidad elegida', open: ' (+', close: ')' },
  fr: { level: ' (selon le niveau)', bonus: ' bonus', base: ' de base', scale: (p, n, k) => `${p} de ${n}${k}`, stack: (v) => ` (+${v} par cumul)`, mana: 'mana max', ranged: (p) => ` (${p} à distance)`, spell: 'compétence choisie', open: ' (+', close: ')' },
  ru: { level: ' (зависит от уровня)', bonus: 'доп. ', base: 'базов. ', scale: (p, n, k) => `${p} ${k}${n}`, stack: (v) => ` (+${v} за заряд)`, mana: 'макс. маны', ranged: (p) => ` (${p} для дальнего боя)`, spell: 'выбранное умение', open: ' (+', close: ')' },
};

const num = (v, precision = 2) => {
  const p = 10 ** precision;
  return String(Math.round(v * p) / p);
};

/**
 * One calculation as a range plus scalings: { lo, hi, scal: [{ coef, stat, formula }], stack, ranged, percent, precision }.
 * `missing` collects what could not be read (logged by the caller).
 */
function evaluate(calcs, values, key, missing, depth = 0) {
  const calc = lookup(calcs, key);
  if (!calc || depth > 6) {
    missing.add(`calc ${key}`);
    return null;
  }
  const value = (name) => {
    const v = lookup(values, name);
    if (v !== undefined) return v;
    missing.add(`value ${name}`);
    return 0;
  };
  const part = (p) => {
    const t = p.__type;
    if (t === 'NamedDataValueCalculationPart') return { lo: value(p.mDataValue), hi: value(p.mDataValue), scal: [] };
    if (t === '{2b25a73a}') return { lo: value(p.DataValue), hi: value(p.DataValue), scal: [] };
    if (t === 'NumberCalculationPart') return { lo: p.mNumber ?? 0, hi: p.mNumber ?? 0, scal: [] };
    if (t === 'ByCharLevelInterpolationCalculationPart') return { lo: p.mStartValue ?? 0, hi: p.mEndValue ?? 0, scal: [] };
    if (t === '{ee18a47b}') return { lo: value(p.StartDataValue), hi: value(p.EndDataValue), scal: [] };
    if (t === 'ByCharLevelBreakpointsCalculationPart') {
      const lo = p.mLevel1Value ?? 0;
      return { lo, hi: lo + (p.mBreakpoints ?? []).reduce((s, b) => s + (b.mAdditionalBonusAtThisLevel ?? 0), 0), scal: [] };
    }
    if (t === 'StatByCoefficientCalculationPart') return { lo: 0, hi: 0, scal: [{ coef: p.mCoefficient ?? 0, stat: p.mStat ?? 0, formula: p.mStatFormula ?? 0 }] };
    if (t === 'StatByNamedDataValueCalculationPart') return { lo: 0, hi: 0, scal: [{ coef: value(p.mDataValue), stat: p.mStat ?? 0, formula: p.mStatFormula ?? 0 }] };
    if (t === 'AbilityResourceByCoefficientCalculationPart') return { lo: 0, hi: 0, scal: [{ coef: p.mCoefficient ?? 0, stat: 'mana', formula: 0 }] };
    if (t === 'BuffCounterByCoefficientCalculationPart') return { lo: 0, hi: 0, scal: [], stack: p.mCoefficient ?? 0 };
    if (t === 'BuffCounterByNamedDataValueCalculationPart') return { lo: 0, hi: 0, scal: [], stack: value(p.mDataValue) };
    if (t === '{f3cbe7b2}') return evaluate(calcs, values, p.mSpellCalculationKey, missing, depth + 1);
    if (t === 'SumOfSubPartsCalculationPart') return sum((p.mSubparts ?? []).map(part));
    if (t === 'ProductOfSubPartsCalculationPart') {
      const a = part(p.mPart1);
      const b = part(p.mPart2);
      if (!a || !b) return null;
      // A constant times anything: scale it (both constant over levels).
      const k = a.scal.length || a.lo !== a.hi ? b : a;
      const o = k === a ? b : a;
      return { lo: o.lo * k.lo, hi: o.hi * k.lo, scal: o.scal.map((s) => ({ ...s, coef: s.coef * k.lo })), stack: o.stack ? o.stack * k.lo : undefined };
    }
    missing.add(`part ${t}`);
    return null;
  };
  const sum = (parts) => {
    if (parts.some((p) => !p)) return null;
    return parts.reduce((a, p) => ({ lo: a.lo + p.lo, hi: a.hi + p.hi, scal: [...a.scal, ...p.scal], stack: (a.stack ?? 0) + (p.stack ?? 0) || undefined }), { lo: 0, hi: 0, scal: [] });
  };
  const times = (r, m) => (r ? { ...r, lo: r.lo * m, hi: r.hi * m, scal: r.scal.map((s) => ({ ...s, coef: s.coef * m })), stack: r.stack ? r.stack * m : undefined } : r);
  const t = calc.__type;
  let r;
  if (t === 'GameCalculationModified') r = evaluate(calcs, values, calc.mModifiedGameCalculation, missing, depth + 1);
  else if (t === 'GameCalculationConditional') r = evaluate(calcs, values, calc.mDefaultGameCalculation, missing, depth + 1);
  else r = sum((calc.mFormulaParts ?? []).map(part));
  if (!r) return null;
  if (calc.mMultiplier) {
    const m = part(calc.mMultiplier);
    if (m) r = times(r, m.lo);
  }
  const ranged = calc.mRangedMultiplier ? part(calc.mRangedMultiplier)?.lo : undefined;
  return { ...r, ranged, percent: calc.mDisplayAsPercent ?? r.percent ?? false, precision: calc.mPrecision ?? r.precision };
}

/** A calculation as text in one language; unknown stats are dropped (and noted). */
function show(r, lang, missing) {
  const w = WORDS[lang];
  const v = (x) => (r.percent ? `${num(x * 100, r.precision ?? 1)}%` : num(x, r.precision ?? 2));
  const scal = r.scal
    .filter((s) => {
      if (s.stat === 'mana' || STATS[s.stat]) return true;
      missing.add(`stat ${s.stat}`);
      return false;
    })
    .map((s) => {
      const name = s.stat === 'mana' ? w.mana : STATS[s.stat][lang];
      const kind = s.formula === 2 ? w.bonus : s.formula === 1 ? w.base : '';
      return w.scale(`${num(s.coef * 100, 1)}%`, name, kind);
    });
  const base = r.lo === r.hi ? v(r.lo) : `${v(r.lo)} - ${v(r.hi)}${w.level}`;
  let text;
  if (r.lo === 0 && r.hi === 0 && scal.length) text = scal.join(' + ');
  else text = base + scal.map((s) => `${w.open}${s}${w.close}`).join('');
  if (r.stack) text += w.stack(v(r.stack));
  if (r.ranged !== undefined && r.ranged !== 1) text += w.ranged(`${num(r.ranged * 100, 1)}%`);
  return text;
}

/** A spell value as the tooltip asks for it ("@Name*100@"). */
function scaled(v, mult) {
  return num(v * (mult === undefined ? 1 : Number(mult)), 2);
}

/** Resolves one tooltip: references, values, calculations; drops counters and icons. */
export function resolve(text, table, calcs, values, lang, missing) {
  let s = text ?? '';
  for (let i = 0; i < 5 && /\{\{\s*\w+\s*\}\}/.test(s); i++) {
    // The ability an augment picks for each player shows as "chosen ability".
    s = s.replace(/\{\{\s*(\w+)\s*\}\}/g, (all, k) => (k === 'SpellName' ? WORDS[lang].spell : (table[k.toLowerCase()] ?? (missing.add(`ref ${k}`), ''))));
  }
  // Lines the game fills in while playing ("Damage dealt: @f1@").
  s = s
    .split(/<br\s*\/?>/i)
    .filter((line) => !/@f\d[^@]*@/i.test(line))
    .join('<br>');
  s = s.replace(/%i:[^%]+%\s?/g, '');
  s = s.replace(/@SpellName@/g, WORDS[lang].spell);
  s = s.replace(/@([A-Za-z0-9_{}.]+?)(?:\*(-?[\d.]+))?@/g, (all, name, mult) => {
    if (lookup(calcs, name)) {
      const r = evaluate(calcs, values, name, missing);
      return r ? show(mult ? { ...r, lo: r.lo * mult, hi: r.hi * mult, scal: r.scal.map((x) => ({ ...x, coef: x.coef * mult })) } : r, lang, missing) : '?';
    }
    if (lookup(values, name) !== undefined) return scaled(lookup(values, name), mult);
    missing.add(`@${name}@`);
    return '?';
  });
  // Tags left empty by the removed lines, and stray breaks.
  s = s.replace(/<(\w+)>\s*<\/\1>/g, '').replace(/^(\s*<br>)+|(<br>\s*)+$/g, '').replace(/(<br>\s*){3,}/g, '<br><br>');
  return s.trim();
}

/**
 * Every augment of the mode: { [id]: { en, zh, ja... } } (markup kept), and what could not be read.
 * `tables`: each language's string table entries, by language id.
 */
export function augmentTexts(kiwi, tables) {
  const out = {};
  const problems = [];
  for (const a of Object.values(kiwi)) {
    if (a?.__type !== 'AugmentData' || !a.AugmentPlatformId) continue;
    const spell = kiwi[a.RootSpell]?.mSpell ?? {};
    const values = Object.fromEntries((spell.DataValues ?? []).map((d) => [d.name, d.values?.[1] ?? d.values?.[0] ?? 0]));
    const calcs = spell.mSpellCalculations ?? {};
    const keys = [a.AugmentTooltipTra, a.DescriptionTra].filter(Boolean).map((k) => k.toLowerCase());
    const pick = (table) => keys.map((k) => table[k]).find((t) => t && !/^\s*$/.test(t));
    const missing = new Set();
    const text = {};
    for (const lang of AUGMENT_LANGS) {
      const table = tables[lang];
      if (!table) continue;
      const t = resolve(pick(table), table, calcs, values, lang, lang === 'en' ? missing : new Set());
      if (t) text[lang] = t;
    }
    if (!text.en && !text.zh) continue;
    out[a.AugmentPlatformId] = text;
    if (missing.size) problems.push(`${a.AugmentNameId}: ${[...missing].join(', ')}`);
  }
  return { texts: out, problems };
}
