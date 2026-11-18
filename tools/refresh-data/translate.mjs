// Chinese for the wiki's ability change lines, which have no official translation.
//
// Every line is looked up in, in order: zh-lines.json (written and reviewed by hand, in this
// repository), then the translations earlier runs made (translations-zh.json on the data
// branch). Lines still missing are translated by Claude when ANTHROPIC_API_KEY is set, with the
// game's own Chinese names for the champion, its abilities and common terms as a glossary, so the
// result reads like Tencent's patch notes. Without a key they stay missing: the app then leaves
// them out in Chinese, and the run reports how many wait for a translation.

import Anthropic from '@anthropic-ai/sdk';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const MODEL = 'claude-opus-5-5';

/** Official Chinese for words these lines use (from the game's string tables and Tencent's notes). */
const GLOSSARY = {
  AP: '法术强度',
  AD: '攻击力',
  'bonus AD': '额外攻击力',
  'maximum health': '最大生命值',
  'bonus health': '额外生命值',
  'missing health': '已损失生命值',
  cooldown: '冷却时间',
  'ability haste': '技能急速',
  tenacity: '韧性',
  omnivamp: '全能吸血',
  'health relic': '治疗圣物',
  'Stat Anvil': '属性锻造器',
  augment: '强化符文',
  takedown: '参与击杀',
  minion: '小兵',
  turret: '防御塔',
  'Howling Abyss': '嚎哭深渊',
  'NEW EFFECT': '新增效果',
  REMOVED: '移除',
};

/** The translations known so far: the reviewed file wins over earlier runs. */
export function memory(here, prevDir) {
  const read = (p) => (existsSync(p) ? JSON.parse(readFileSync(p, 'utf8')) : {});
  return { ...(prevDir ? read(join(prevDir, 'translations-zh.json')) : {}), ...read(join(here, 'zh-lines.json')) };
}

/**
 * Translates the missing lines with Claude; returns English to Chinese for those it could.
 * `items`: { text, champion: { en, zh }, ability: { en, zh } }.
 */
export async function translateMissing(items, log) {
  if (!items.length) return {};
  if (!process.env.ANTHROPIC_API_KEY) {
    log(`translations: ${items.length} lines have no Chinese yet (no ANTHROPIC_API_KEY)`);
    return {};
  }
  const client = new Anthropic();
  const out = {};
  for (let i = 0; i < items.length; i += 40) {
    const batch = items.slice(i, i + 40);
    const names = [...new Map(batch.flatMap((b) => [[b.champion.en, b.champion.zh], [b.ability.en, b.ability.zh]])).entries()].filter(([en, zh]) => en && zh && en !== zh);
    const glossary = [...Object.entries(GLOSSARY), ...names].map(([en, zh]) => `${en} = ${zh}`).join('\n');
    const response = await client.beta.messages.create({
      model: MODEL,
      max_tokens: 16000,
      betas: ['server-side-fallback-2026-07-01'],
      fallbacks: 'default',
      output_config: {
        effort: 'low',
        format: {
          type: 'json_schema',
          schema: {
            type: 'object',
            properties: { translations: { type: 'array', items: { type: 'object', properties: { en: { type: 'string' }, zh: { type: 'string' } }, required: ['en', 'zh'], additionalProperties: false } } },
            required: ['translations'],
            additionalProperties: false,
          },
        },
      },
      system:
        'You translate League of Legends ability change notes into Simplified Chinese as used on the Chinese servers (Tencent patch notes style). ' +
        'Use the glossary terms exactly; keep every number, slash list and percent sign as written; write ratios like "40%法术强度"; ' +
        '"changed to X" is "改为X", "increased to X from Y" is "提升至X（原Y）", "reduced to X from Y" is "降低至X（原Y）". Return one translation per line, with its English unchanged.',
      messages: [
        {
          role: 'user',
          content: `Glossary:\n${glossary}\n\nLines (champion | ability | text):\n${batch.map((b) => `${b.champion.en} | ${b.ability.en} | ${b.text}`).join('\n')}`,
        },
      ],
    });
    if (response.stop_reason === 'refusal') {
      log('translations: declined');
      continue;
    }
    const text = response.content.find((b) => b.type === 'text')?.text ?? '';
    for (const t of JSON.parse(text).translations ?? []) {
      if (batch.some((b) => b.text === t.en) && /[一-鿿]/.test(t.zh)) out[t.en] = t.zh;
    }
  }
  log(`translations: Claude translated ${Object.keys(out).length} of ${items.length} new lines`);
  return out;
}
