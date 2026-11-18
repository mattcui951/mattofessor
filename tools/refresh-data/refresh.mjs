// Builds the app's `data` branch files:
//   mayhem.json       Hextech ARAM (ARAM: Mayhem) champion modifiers, rules and headliners, and
//                     every augment's description with its numbers (augments.mjs)
//   patch-notes.json  The latest patches' Summoner's Rift and ARAM: Mayhem changes: Riot's
//                     notes in English, Tencent's official Chinese notes in Chinese
//   translations-zh.json  Chinese for the wiki lines (see translate.mjs)
//   meta.json         When it was checked, and whether the wiki's rule text changed
//
// Champion modifiers come from the League of Legends Wiki (CC BY-SA): the ARAM page's list of
// mode overrides, then the ARAM: Mayhem page's own layer on top. The wiki answers scripts with a
// challenge page, so it is read in a headless browser. Patch notes come from Riot's site.
// The rules and headliners are written by hand in two languages (rules.json); the wiki's text of
// those sections is hashed so a change there is noticed and reviewed.
//
// Chinese never mixes in English: names come from the game's own Chinese data (Data Dragon,
// CommunityDragon's string tables), patch notes from Tencent, the rest from translate.mjs.
//
// Usage: node refresh.mjs --out <dir> [--patches 5] [--prev <the data branch's files>]
// Exits non-zero, writing nothing, when anything looks wrong: the app keeps the last good data.

import { chromium } from 'playwright';
import { memory, translateMissing } from './translate.mjs';
import { KIWI, augmentTexts } from './augments.mjs';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > 0 ? process.argv[i + 1] : fallback;
};
const OUT = arg('out', join(here, 'out'));
const PATCHES = Number(arg('patches', 5));
const PREV = arg('prev', '');
const WIKI = 'https://wiki.leagueoflegends.com/en-us/';
const DD = 'https://ddragon.leagueoflegends.com';
const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140 Safari/537.36';

const log = (...a) => console.log(...a);
function fail(msg) {
  console.error(`refresh-data: ${msg}`);
  process.exit(1);
}

async function json(url) {
  const r = await fetch(url, { headers: { 'User-Agent': UA } });
  if (!r.ok) fail(`${url}: ${r.status}`);
  return r.json();
}

// ---------------------------------------------------------------------------------------------
// Data Dragon: names to ids, ability names to keys, Chinese names

const norm = (s) => s.toLowerCase().normalize('NFKD').replace(/[^a-z0-9]/g, '');

async function dataDragon() {
  const version = (await json(`${DD}/api/versions.json`))[0];
  const [en, zh, items, itemsZh] = await Promise.all([
    json(`${DD}/cdn/${version}/data/en_US/championFull.json`),
    json(`${DD}/cdn/${version}/data/zh_CN/championFull.json`),
    json(`${DD}/cdn/${version}/data/en_US/item.json`),
    json(`${DD}/cdn/${version}/data/zh_CN/item.json`),
  ]);
  const champs = new Map(); // normalized English name -> { id, abilities }
  for (const [key, c] of Object.entries(en.data)) {
    const z = zh.data[key];
    const abilities = [
      { key: 'P', en: c.passive.name, zh: z?.passive.name ?? c.passive.name },
      ...c.spells.map((s, i) => ({ key: 'QWER'[i], en: s.name, zh: z?.spells[i]?.name ?? s.name })),
    ];
    // zh_CN puts the epithet in "name" and the champion's name in "title".
    const entry = { id: Number(c.key), name: c.name, zh: z?.title ?? c.name, abilities };
    champs.set(norm(c.name), entry);
    champs.set(norm(key), entry);
  }
  // Names the wiki and patch notes write differently from Data Dragon.
  for (const [alias, real] of [['wukong', 'monkeyking'], ['nunu', 'nunu'], ['renata', 'renataglasc']]) {
    const e = champs.get(real) ?? champs.get(norm(real));
    if (e && !champs.has(alias)) champs.set(alias, e);
  }
  const itemIds = new Map();
  for (const [id, it] of Object.entries(items.data)) {
    const n = Number(id);
    if (n < 10000 || (n >= 120000 && n < 130000)) if (!itemIds.has(norm(it.name)) || n < itemIds.get(norm(it.name))) itemIds.set(norm(it.name), n);
  }
  const itemIdsZh = new Map();
  for (const [id, it] of Object.entries(itemsZh.data)) {
    const n = Number(id);
    if (n < 10000 || (n >= 120000 && n < 130000)) if (!itemIdsZh.has(zhKey(it.name)) || n < itemIdsZh.get(zhKey(it.name))) itemIdsZh.set(zhKey(it.name), n);
  }
  const [major, minor] = version.split('.').map(Number);
  return {
    version,
    patch: `${major + 10}.${minor}`,
    champs,
    itemIds,
    itemIdsZh,
    augmentIds: await augmentIds('default', norm),
    augmentIdsZh: await augmentIds('zh_cn', zhKey),
  };
}

/** Comparable Chinese text: no spaces or punctuation. */
const zhKey = (s) => s.replace(/[\s\p{P}\p{S}]/gu, '');

/** The champion a Chinese heading names ("铸星龙王 奥瑞利安·索尔"): the longest name it contains. */
function champFromZh(dd, text) {
  const t = zhKey(text);
  let best = null;
  for (const c of new Set(dd.champs.values())) if (t.includes(zhKey(c.zh)) && (!best || c.zh.length > best.zh.length)) best = c;
  return best;
}

/**
 * Augment English name to id, as the app knows augments (CommunityDragon's cherry-augments.json,
 * shared with Arena). A name used twice resolves to the id AramHex records for Hextech ARAM.
 */
async function augmentIds(folder, key) {
  const all = await json(`https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/${folder}/v1/cherry-augments.json`);
  const hex = await json('https://raw.githubusercontent.com/czgreat/aramhex-data/main/latest.json');
  const mayhem = new Set(Object.keys(await json(`https://raw.githubusercontent.com/czgreat/aramhex-data/main/${hex.path}/augments.json`)).map(Number));
  const byName = new Map();
  for (const a of all) byName.set(key(a.nameTRA), [...(byName.get(key(a.nameTRA)) ?? []), a.id]);
  const ids = new Map();
  for (const [name, list] of byName) {
    const id = list.find((i) => mayhem.has(i)) ?? (list.length === 1 ? list[0] : undefined);
    if (id !== undefined) ids.set(name, id);
  }
  return ids;
}

// ---------------------------------------------------------------------------------------------
// The game's own text (CommunityDragon's copy of the string tables, English and Chinese)

/**
 * Official Chinese for spell names the wiki uses that are not one of a champion's five (Lee Sin's
 * Resonating Strike, Hwei's subspells), and the headliners' official names and descriptions.
 */
async function stringTables() {
  const base = 'https://raw.communitydragon.org/latest/game';
  const [en, zh] = await Promise.all([json(`${base}/en_us/data/menu/en_us/lol.stringtable.json`), json(`${base}/zh_cn/data/menu/en_us/lol.stringtable.json`)]);
  const E = en.entries;
  const Z = zh.entries;
  const plainText = (v) => (v ?? '').replace(/<[^>]+>/g, '').replace(/%i:[^%]+%/g, '').replace(/\s+/g, ' ').trim();
  const spellZh = new Map();
  for (const [k, v] of Object.entries(E)) {
    if (!/^(game_spell_displayname_|spell_\w+_name$)/.test(k) || !Z[k]) continue;
    const name = plainText(v);
    if (name && !spellZh.has(name)) spellZh.set(name, plainText(Z[k]));
  }
  const goh = (id) => {
    const t = E[`kiwi_goh_${id}_title`];
    const d = E[`kiwi_goh_${id}_long_desc`];
    if (!t || !d || !Z[`kiwi_goh_${id}_title`] || !Z[`kiwi_goh_${id}_long_desc`]) return null;
    return { name: { en: plainText(t), zh: plainText(Z[`kiwi_goh_${id}_title`]) }, effect: { en: plainText(d), zh: plainText(Z[`kiwi_goh_${id}_long_desc`]) } };
  };
  if (spellZh.size < 500) fail(`string tables: only ${spellZh.size} spell names`);
  return { spellZh, goh, en: E, zh: Z };
}

// ---------------------------------------------------------------------------------------------
// The wiki: mode override tables

/** In the page: every row of the champion overrides table, cells split into titled groups. */
function readOverrides() {
  const table = [...document.querySelectorAll('table.article-table')].find((t) => /General changes/.test(t.querySelector('tr')?.innerText ?? ''));
  if (!table) return null;
  const groups = (cell) => {
    const out = [];
    let cur = null;
    const start = (title) => {
      cur = { title, lines: [] };
      out.push(cur);
    };
    for (const n of cell?.childNodes ?? []) {
      if (n.nodeType === 3) {
        const t = n.textContent.trim();
        if (t) start(t);
        continue;
      }
      if (n.nodeType !== 1) continue;
      if (n.matches('ul')) {
        if (!cur) start('');
        for (const li of n.querySelectorAll(':scope > li')) cur.lines.push(li.innerText.replace(/\s+/g, ' ').trim());
        continue;
      }
      const icon = n.matches('.ability-icon') ? n : n.querySelector('.ability-icon');
      const t = (icon ?? n).innerText.replace(/\s+/g, ' ').trim();
      if (t) start(t);
    }
    return out.filter((g) => g.lines.length);
  };
  return [...table.querySelectorAll('tbody tr')]
    .map((tr) => tr.querySelectorAll('td'))
    .filter((td) => td.length >= 3)
    .map((td) => ({ name: td[0].innerText.trim(), general: groups(td[1]), abilities: groups(td[2]) }));
}

/** In the page: the plain text of the rule sections, for change detection. */
function readRuleText(ids) {
  return ids.map((id) => {
    const hd = document.getElementById(id);
    if (!hd) return `${id}: missing`;
    let blk = hd;
    while (blk.parentElement && !blk.parentElement.classList.contains('mw-parser-output')) blk = blk.parentElement;
    let el = blk.nextElementSibling;
    let txt = '';
    while (el && !el.classList.contains('mw-heading')) {
      txt += el.innerText + '\n';
      el = el.nextElementSibling;
    }
    return `${id}:\n${txt.replace(/\s+/g, ' ').trim()}`;
  });
}

const RULE_SECTIONS = ['Global_stat_changes', 'Combo_Breaker', 'Miscellaneous_changes', 'Headliners', 'Selection_screens_&_rerolling'];

async function openWiki(browser, page) {
  const p = await browser.newPage({ userAgent: UA });
  await p.goto(WIKI + page, { waitUntil: 'domcontentloaded', timeout: 90000 });
  // The challenge page reloads into the article; wait for the article's tables.
  await p.waitForSelector('table.article-table', { timeout: 90000 });
  return p;
}

// Special modifier names on the wiki, to our keys. Percentages become fractions; ability haste
// stays a number of points.
const STATS = {
  'damage dealt': 'dealt',
  'damage received': 'received',
  'healing done': 'healing',
  'shielding done': 'shielding',
  tenacity: 'tenacity',
  'ability haste': 'abilityHaste',
  'energy regeneration': 'energyRegen',
  'total attack speed': 'attackSpeed',
  'total attack speed increase': 'attackSpeed',
};

function statValue(num, pct) {
  const v = Number(num);
  return pct ? Math.round(v * 100) / 10000 : v;
}

/** "Damage dealt reduced by 10%." (ARAM) or "Damage dealt changed to -5% from -7%." (Mayhem). */
function parseStat(line, champ) {
  let m = /^(.+?) (increased|reduced) by ([\d.]+)(%?)\.?$/i.exec(line);
  if (m) {
    const key = STATS[m[1].toLowerCase()];
    if (!key) fail(`${champ}: unknown modifier "${line}"`);
    const v = statValue(m[3], m[4]);
    return { key, value: m[2].toLowerCase() === 'reduced' ? -v : v };
  }
  m = /^(.+?) changed to ([+-]?[\d.]+)(%?) from ([+-]?[\d.]+)(%?)\.?$/i.exec(line);
  if (m) {
    const key = STATS[m[1].toLowerCase()];
    if (!key) fail(`${champ}: unknown modifier "${line}"`);
    return { key, value: statValue(m[2], m[3]), from: statValue(m[4], m[5]) };
  }
  fail(`${champ}: unreadable modifier "${line}"`);
}

/** What a change line is about ("Cooldown" in "Cooldown changed to 18 seconds."). */
const subject = (line) => line.replace(/\s+(changed|increased|reduced|decreased)\s+(to|by)\b.*$/i, '').toLowerCase();

function abilityOf(champ, title, st) {
  if (/^general$/i.test(title)) return { key: '', en: 'General', zh: '通用' };
  const found = champ.abilities.find((a) => norm(a.en) === norm(title));
  if (found) return found;
  // One half of a two-form ability ("Safeguard / Iron Will"): the same half of its Chinese name.
  for (const a of champ.abilities) {
    const ens = a.en.split(/\s*\/\s*/);
    const zhs = a.zh.split(/\s*\/\s*/);
    const i = ens.findIndex((x) => norm(x) === norm(title));
    if (i >= 0 && ens.length > 1) return { key: a.key, en: title, zh: zhs.length === ens.length ? zhs[i] : a.zh };
  }
  const zh = st.spellZh.get(title);
  if (!zh) fail(`${champ.name}: no Chinese name for the ability "${title}"`);
  return { key: null, en: title, zh };
}

function buildChampions(aramRows, mayhemRows, dd, st) {
  const champions = {};
  const lookup = (name) => dd.champs.get(norm(name)) ?? fail(`unknown champion "${name}"`);
  const entry = (c) =>
    (champions[c.id] ??= { name: c.name, zh: c.zh, stats: {}, aram: {}, abilities: [] });
  const ability = (e, a) => {
    let found = e.abilities.find((x) => x.name.en === a.en);
    if (!found) e.abilities.push((found = { key: a.key, name: { en: a.en, zh: a.zh }, lines: [] }));
    return found;
  };
  for (const [rows, mode] of [
    [aramRows, 'aram'],
    [mayhemRows, 'mayhem'],
  ]) {
    for (const row of rows) {
      const c = lookup(row.name);
      const e = entry(c);
      for (const g of row.general) {
        if (!/special modifiers/i.test(g.title)) fail(`${row.name}: unknown group "${g.title}"`);
        for (const line of g.lines) {
          const s = parseStat(line, row.name);
          if (mode === 'mayhem') {
            e.aram[s.key] = e.stats[s.key] ?? 0;
            if (Math.abs(s.from - e.aram[s.key]) > 1e-9) log(`note: ${row.name} ${s.key}: Mayhem says from ${s.from}, ARAM list has ${e.aram[s.key]}`);
          }
          e.stats[s.key] = s.value;
        }
      }
      for (const g of row.abilities) {
        const a = ability(e, abilityOf(c, g.title, st));
        if (mode === 'mayhem') {
          // A Mayhem line replaces the ARAM line about the same thing.
          const replaced = new Set(g.lines.map(subject));
          a.lines = a.lines.filter((l) => !replaced.has(subject(l.text)));
        }
        for (const text of g.lines) a.lines.push({ text, mode });
      }
    }
  }
  for (const e of Object.values(champions)) {
    for (const k of Object.keys(e.stats)) if (e.stats[k] === 0 && !(k in e.aram)) delete e.stats[k];
    const order = { P: 0, Q: 1, W: 2, E: 3, R: 4, '': 5 };
    e.abilities.sort((a, b) => (order[a.key] ?? 6) - (order[b.key] ?? 6));
  }
  return champions;
}

/**
 * Champion id to "melee" or "ranged", as the game counts it for the mode's bonus stats
 * (CommunityDragon's per-champion data; Data Dragon's attack range gets Gnar wrong).
 */
async function attackTypes(dd) {
  const ids = [...new Set([...dd.champs.values()].map((c) => c.id))];
  const out = {};
  for (let i = 0; i < ids.length; i += 16) {
    await Promise.all(
      ids.slice(i, i + 16).map(async (id) => {
        const c = await json(`https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champions/${id}.json`);
        const t = c.tacticalInfo?.attackType;
        if (t !== 'melee' && t !== 'ranged') fail(`${c.name}: attack type "${t}"`);
        out[id] = t;
      }),
    );
  }
  return out;
}

/** Champion id to its abilities' names in both languages (for patch note headings). */
function abilityNames(dd) {
  const out = {};
  for (const c of dd.champs.values()) out[c.id] = Object.fromEntries(c.abilities.map((a) => [a.key, { en: a.en, zh: a.zh }]));
  return out;
}

// ---------------------------------------------------------------------------------------------
// Riot patch notes

const patchUrls = (p) => {
  const s = p.replace('.', '-');
  return [`https://www.leagueoflegends.com/en-us/news/game-updates/league-of-legends-patch-${s}-notes/`, `https://www.leagueoflegends.com/en-us/news/game-updates/patch-${s}-notes/`];
};

/** In the page: the article as sections of entries, in reading order. */
function readPatchNotes() {
  const root = document.querySelector('#patch-notes-container') ?? document.querySelector('article') ?? document.body;
  const clean = (s) => s.replace(/ /g, ' ').replace(/\s+/g, ' ').trim();
  const sections = [];
  let sec = null;
  let group = '';
  let entry = null;
  let part = null;
  const line = (li) => {
    const text = clean(li.innerText);
    const m = /^(.+?):\s*(.+?)\s*⇒\s*(.+)$/.exec(text);
    return m ? { text, attr: m[1], before: m[2], after: m[3] } : { text };
  };
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, {
    acceptNode: (n) => (/^(H2|H3|H4|P|UL|BLOCKQUOTE)$/.test(n.tagName) && !n.parentElement.closest('ul, blockquote') ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP),
  });
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    // Headings are styled in capitals; their text is not.
    const text = clean(/^H[234]$/.test(n.tagName) ? n.textContent : n.innerText);
    if (n.tagName === 'H2') {
      sec = { title: text, intro: '', entries: [], lines: [] };
      sections.push(sec);
      group = '';
      entry = part = null;
      continue;
    }
    if (!sec || !text) continue;
    if (n.tagName === 'H3') {
      entry = { group, name: text, isNew: false, summary: '', parts: [], fromH3: true };
      sec.entries.push(entry);
      part = null;
    } else if (n.tagName === 'H4') {
      if (entry?.fromH3) {
        part = { title: text, lines: [] };
        entry.parts.push(part);
      } else {
        group = text;
        entry = part = null;
      }
    } else if (n.tagName === 'P') {
      const strong = n.querySelector('strong');
      if (strong && clean(n.innerText.replace(/\bNEW\b/, '')) === clean(strong.innerText.replace(/\bNEW\b/, '')) || (strong && /\bNEW\b/.test(text) && n.querySelectorAll('strong').length <= 2 && text.length < 80)) {
        const name = clean(text.replace(/^NEW\s*/, ''));
        entry = { group, name, isNew: /^NEW\b/.test(text), summary: '', parts: [], fromH3: false };
        sec.entries.push(entry);
        part = null;
      } else if (entry) entry.summary = clean(`${entry.summary} ${text}`);
      else sec.intro = clean(`${sec.intro} ${text}`);
    } else if (n.tagName === 'BLOCKQUOTE') {
      if (entry) entry.summary = clean(`${entry.summary} ${text}`);
      else sec.intro = clean(`${sec.intro} ${text}`);
    } else if (n.tagName === 'UL') {
      const lines = [...n.querySelectorAll(':scope > li')].map(line);
      if (part) part.lines.push(...lines);
      else {
        // Bullets under a group heading with no named entry (bugfixes, systems) get an unnamed one.
        if (!entry && group) {
          entry = { group, name: '', isNew: false, summary: '', parts: [], fromH3: false };
          sec.entries.push(entry);
        }
        if (entry) {
          if (!entry.parts.length) entry.parts.push({ title: '', lines: [] });
          entry.parts[entry.parts.length - 1].lines.push(...lines);
        } else sec.lines.push(...lines);
      }
    }
  }
  const date = document.querySelector('time')?.getAttribute('datetime') ?? '';
  return { date, sections };
}

const RIFT = /^(Champions|Items|Runes|Summoner Spells|Systems)$/i;
const MAYHEM = /^ARAM:?\s*Mayhem/i;

function shapePatch(raw, dd) {
  const keep = (secs) =>
    secs.map((s) => ({
      title: s.title,
      intro: s.intro,
      lines: s.lines,
      entries: s.entries.map(({ fromH3, ...e }) => {
        const champ = dd.champs.get(norm(e.name));
        const item = dd.itemIds.get(norm(e.name));
        const augment = dd.augmentIds.get(norm(e.name));
        return {
          ...e,
          ...(champ && /champion/i.test(`${s.title} ${e.group}`) ? { championId: champ.id } : {}),
          ...(item && /item/i.test(`${s.title} ${e.group}`) ? { itemId: item } : {}),
          ...(augment && /augment/i.test(e.group) ? { augmentId: augment } : {}),
        };
      }),
    }));
  return {
    rift: keep(raw.sections.filter((s) => RIFT.test(s.title))),
    mayhem: keep(raw.sections.filter((s) => MAYHEM.test(s.title))),
  };
}

async function patchNotes(browser, dd) {
  const [major, minor] = dd.patch.split('.').map(Number);
  const wanted = [];
  for (let m = minor; m >= 1 && wanted.length < PATCHES; m--) wanted.push(`${major}.${m}`);
  const page = await browser.newPage({ userAgent: UA });
  const patches = [];
  for (const patch of wanted) {
    let raw = null;
    let url = null;
    for (const u of patchUrls(patch)) {
      const r = await page.goto(u, { waitUntil: 'domcontentloaded', timeout: 60000 });
      if (r && r.ok()) {
        url = u;
        raw = await page.evaluate(readPatchNotes);
        break;
      }
    }
    if (!raw) {
      log(`patch ${patch}: no notes yet`);
      continue;
    }
    const shaped = shapePatch(raw, dd);
    log(`patch ${patch}: rift ${shaped.rift.length} sections, mayhem ${shaped.mayhem.length} sections`);
    patches.push({ patch, url, date: raw.date, ...shaped });
  }
  await page.close();
  return patches;
}

// ---------------------------------------------------------------------------------------------
// Tencent's official Chinese patch notes (lol.qq.com)

const QQ_LIST = 'https://apps.game.qq.com/cmc/zmMcnTargetContentList?r0=json&page=1&num=60&target=24&source=web_pc';

/** In the page: the article as sections of entries, like readPatchNotes. */
function readTencentNotes() {
  const root = document.querySelector('#newsContent') ?? document.body;
  const clean = (s) => s.replace(/[\u00a0\u2003]/g, ' ').replace(/\s+/g, ' ').trim();
  const line = (text) => {
    const m = /^(.+?)[：:]\s*(.+?)\s*⇒\s*(.+)$/.exec(text);
    return m ? { text, attr: m[1].trim(), before: m[2].trim(), after: m[3].trim() } : { text };
  };
  // In these sections a heading under the section is a group (英雄, 强化符文 ...), elsewhere an entry.
  const GROUPED = /海克斯大乱斗|经典模式|斗魂竞技场|极地大乱斗|无限火力/;
  const sections = [];
  let sec = null;
  let group = '';
  let entry = null;
  let part = null;
  const newEntry = (name) => {
    entry = { group, name, isNew: /^(全新|新增)/.test(name), summary: '', parts: [] };
    sec.entries.push(entry);
    part = null;
  };
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, {
    acceptNode: (n) => (/^(H3|H4|P|UL|BLOCKQUOTE)$/.test(n.tagName) && !n.parentElement.closest('ul, blockquote') ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP),
  });
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const text = clean(n.innerText);
    if (n.tagName === 'H3') {
      sec = { title: text, intro: '', entries: [], lines: [] };
      sections.push(sec);
      group = '';
      entry = part = null;
      continue;
    }
    if (!sec || !text) continue;
    if (n.tagName === 'H4') {
      if (GROUPED.test(sec.title)) {
        group = text;
        entry = part = null;
      } else newEntry(text);
    } else if (n.tagName === 'BLOCKQUOTE') {
      if (entry) entry.summary = clean(`${entry.summary} ${text}`);
      else sec.intro = clean(`${sec.intro} ${text}`);
    } else if (n.tagName === 'UL') {
      const lines = [...n.querySelectorAll(':scope > li')].map((li) => line(clean(li.innerText)));
      if (!entry && group) newEntry('');
      if (!entry) sec.lines.push(...lines);
      else {
        if (!part) entry.parts.push((part = { title: '', lines: [] }));
        part.lines.push(...lines);
      }
    } else if (n.tagName === 'P') {
      const strong = n.querySelector('strong');
      if (GROUPED.test(sec.title) && strong && clean(strong.innerText) && text.replace(clean(strong.innerText), '').replace(/^(全新|新增)/, '').trim() === '') {
        newEntry(text);
        continue;
      }
      const pieces = n.innerText.split('\n').map(clean).filter(Boolean);
      if (!entry) {
        sec.intro = clean(`${sec.intro} ${text}`);
        continue;
      }
      const changes = pieces.filter((p) => p.includes('⇒'));
      if (!changes.length) {
        // An ability heading on its own line ("E - 星芒凝汇"), or a note.
        if (/^([PQWER]|被动)\s*[-–]/.test(pieces[0]) || n.querySelector('img')) entry.parts.push((part = { title: pieces[0], lines: [] }));
        else entry.summary = clean(`${entry.summary} ${text}`);
        continue;
      }
      const head = pieces[0].includes('⇒') ? '' : pieces[0];
      if (head || !part) entry.parts.push((part = { title: head, lines: [] }));
      part.lines.push(...pieces.filter((p) => p !== head).map(line));
    }
  }
  return { sections };
}

const RIFT_ZH = /^(英雄|装备|符文|召唤师技能|系统)$/;
const MAYHEM_ZH = /海克斯大乱斗/;

function shapeZh(raw, dd) {
  const keep = (secs) =>
    secs.map((s) => ({
      title: s.title,
      intro: s.intro,
      lines: s.lines,
      entries: s.entries.map((e) => {
        const where = `${s.title} ${e.group}`;
        const champ = /英雄/.test(where) && e.name ? champFromZh(dd, e.name) : null;
        const item = /装备/.test(where) ? dd.itemIdsZh.get(zhKey(e.name)) : undefined;
        const augment = /强化符文/.test(e.group) ? dd.augmentIdsZh.get(zhKey(e.name)) : undefined;
        return { ...e, ...(champ ? { championId: champ.id } : {}), ...(item ? { itemId: item } : {}), ...(augment ? { augmentId: augment } : {}) };
      }),
    }));
  return { rift: keep(raw.sections.filter((s) => RIFT_ZH.test(s.title))), mayhem: keep(raw.sections.filter((s) => MAYHEM_ZH.test(s.title))) };
}

async function patchNotesZh(browser, dd) {
  const list = await json(QQ_LIST);
  const articles = (list.data?.result ?? [])
    .map((r) => ({ patch: /^(\d+\.\d+)版本更新公告/.exec(r.sTitle ?? '')?.[1], url: r.sRedirectURL, date: r.sIdxTime }))
    .filter((a) => a.patch && a.url)
    .slice(0, PATCHES);
  const page = await browser.newPage({ userAgent: UA });
  const patches = [];
  for (const a of articles) {
    let raw = null;
    for (let attempt = 1; attempt <= 3 && !raw; attempt++) {
      try {
        await page.goto(a.url, { waitUntil: 'domcontentloaded', timeout: 60000 });
        raw = await page.evaluate(readTencentNotes);
      } catch (e) {
        log(`patch ${a.patch} (Tencent): attempt ${attempt} failed: ${String(e).split('\n')[0]}`);
      }
    }
    if (!raw) continue;
    const shaped = shapeZh(raw, dd);
    log(`patch ${a.patch} (Tencent): rift ${shaped.rift.length} sections, mayhem ${shaped.mayhem.length} sections`);
    if (!shaped.rift.length && !shaped.mayhem.length) fail(`Tencent ${a.patch}: nothing read from ${a.url}`);
    patches.push({ patch: a.patch, url: a.url.replace(/^http:/, 'https:'), date: a.date.replace(' ', 'T') + '+08:00', ...shaped });
  }
  await page.close();
  return patches;
}

// ---------------------------------------------------------------------------------------------

/** Puts the Chinese on every wiki line; returns the translations used and how many are missing. */
async function translateLines(champions) {
  const known = memory(here, PREV);
  const missing = [];
  for (const e of Object.values(champions))
    for (const a of e.abilities)
      for (const l of a.lines)
        if (!known[l.text] && !missing.some((m) => m.text === l.text))
          missing.push({ text: l.text, champion: { en: e.name, zh: e.zh }, ability: { en: a.name.en, zh: a.name.zh } });
  Object.assign(known, await translateMissing(missing, log));
  let untranslated = 0;
  for (const e of Object.values(champions))
    for (const a of e.abilities)
      for (const l of a.lines) {
        if (known[l.text]) l.zh = known[l.text];
        else untranslated++;
      }
  return { known, untranslated };
}

function stable(o) {
  return JSON.stringify(o, null, 1) + '\n';
}

/** The data branch's Chinese patch notes, or none. */
function previousZh() {
  if (!PREV) return [];
  try {
    return JSON.parse(readFileSync(join(PREV, 'patch-notes.json'), 'utf8')).zh?.patches ?? [];
  } catch {
    return [];
  }
}

async function main() {
  const dd = await dataDragon();
  log(`Data Dragon ${dd.version} (patch ${dd.patch})`);
  const rules = JSON.parse(readFileSync(join(here, 'rules.json'), 'utf8'));
  const st = await stringTables();
  // Headliners: the game's own name and description where it has them.
  rules.headliners = rules.headliners.map(({ goh, ...h }) => {
    const official = goh ? st.goh(goh) : null;
    if (goh && !official) log(`headliner ${h.id}: no official text for "${goh}", keeping rules.json`);
    return official ? { ...h, ...official, official: true } : h;
  });

  const browser = await chromium.launch();
  try {
    const aramPage = await openWiki(browser, 'ARAM');
    const aram = await aramPage.evaluate(readOverrides);
    const mayhemPage = await openWiki(browser, 'ARAM:_Mayhem');
    const mayhem = await mayhemPage.evaluate(readOverrides);
    const ruleText = (await mayhemPage.evaluate(readRuleText, RULE_SECTIONS)).join('\n\n');
    if (!aram || aram.length < 120) fail(`ARAM overrides: ${aram?.length ?? 'no'} rows`);
    if (!mayhem || mayhem.length < 30) fail(`Mayhem overrides: ${mayhem?.length ?? 'no'} rows`);
    log(`wiki: ${aram.length} ARAM rows, ${mayhem.length} Mayhem rows`);

    const champions = buildChampions(aram, mayhem, dd, st);
    if (Object.keys(champions).length < 120) fail(`only ${Object.keys(champions).length} champions`);
    const rulesHash = createHash('sha256').update(ruleText).digest('hex').slice(0, 16);
    const rulesChanged = rulesHash !== rules.sourceHash;
    if (rulesChanged) log(`rules: the wiki's rule text changed (${rules.sourceHash} -> ${rulesHash}); review rules.json`);

    const attack = await attackTypes(dd);
    const patches = await patchNotes(browser, dd);
    if (!patches.length) fail('no patch notes found');
    let patchesZh = await patchNotesZh(browser, dd);
    if (!patchesZh.length) {
      // lol.qq.com is sometimes out of reach from GitHub's runners: keep the last good Chinese
      // notes rather than holding back everything else.
      patchesZh = previousZh();
      if (!patchesZh.length) fail('no Tencent patch notes found');
      log(`Tencent not reachable: keeping the last ${patchesZh.length} Chinese patch notes`);
    }
    const { known, untranslated } = await translateLines(champions);
    // Augment descriptions with their numbers (the game's text and the mode's spell data).
    const { texts: augments, problems } = augmentTexts(await json(KIWI), st.en, st.zh);
    if (Object.keys(augments).length < 150) fail(`only ${Object.keys(augments).length} augment descriptions`);
    for (const p of problems) log(`augment text: ${p}`);
    log(`augment descriptions: ${Object.keys(augments).length}`);

    mkdirSync(OUT, { recursive: true });
    const { sourceHash, ...shownRules } = rules;
    writeFileSync(
      join(OUT, 'mayhem.json'),
      stable({
        patch: dd.patch,
        source: { name: 'League of Legends Wiki', url: `${WIKI}ARAM:_Mayhem`, license: 'CC BY-SA 3.0' },
        ...shownRules,
        attackTypes: attack,
        abilityNames: abilityNames(dd),
        champions,
        augments,
      }),
    );
    writeFileSync(
      join(OUT, 'patch-notes.json'),
      stable({
        source: { name: 'Riot Games', url: 'https://www.leagueoflegends.com/en-us/news/tags/patch-notes/' },
        patches,
        zh: { source: { name: '英雄联盟官方网站', url: 'https://lol.qq.com/' }, patches: patchesZh },
      }),
    );
    writeFileSync(join(OUT, 'translations-zh.json'), stable(Object.fromEntries(Object.entries(known).sort())));
    writeFileSync(join(OUT, 'meta.json'), stable({ checkedAt: new Date().toISOString(), ddVersion: dd.version, rulesHash, rulesChanged, untranslated }));
    writeFileSync(join(OUT, 'rule-text.txt'), ruleText);
    log(`wrote ${OUT}`);
  } finally {
    await browser.close();
  }
}

main().catch((e) => fail(e.stack ?? String(e)));
