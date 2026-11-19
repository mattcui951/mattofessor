import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api', () => ({ api: {} }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import { _dicts, i18n } from '../i18n.svelte';
import { LOCALES } from '../locales';
import { ago, compact, dec, gamesText, kdaRatio, num, ordinal, positionRole, queueName, roleName, tagText } from '../format';
import { tierScore, tierText } from '../dd.svelte';
import { errorText } from '../state.svelte';
import { cooldownFor } from '../../overlay/spells';

beforeEach(() => {
  i18n.lang = 'en';
});

describe('i18n', () => {
  it('has a dictionary for every language', () => {
    expect(Object.keys(_dicts).sort()).toEqual(LOCALES.map((l) => l.id).sort());
  });

  it('has every key in every language with the same placeholders', () => {
    const en = _dicts.en!;
    for (const l of LOCALES) {
      const d = _dicts[l.id] as Record<string, string>;
      for (const [key, text] of Object.entries(en)) {
        expect(d[key], `${l.id} ${key}`).toBeTruthy();
        expect((d[key].match(/\{\d\}/g) ?? []).sort(), `${l.id} ${key}`).toEqual((text.match(/\{\d\}/g) ?? []).sort());
      }
      expect(Object.keys(d).filter((k) => !(k in en)), `${l.id}: keys English does not have`).toEqual([]);
    }
  });
});

describe('errorText', () => {
  it('translates coded backend errors', () => {
    expect(errorText('[no-build] No high-elo games on this champion yet')).toBe('No high-elo games on this champion yet');
    i18n.lang = 'zh';
    expect(errorText('[no-player:Foo#NA1] No player named Foo#NA1')).toBe('找不到玩家 Foo#NA1');
    expect(errorText('[unknown-code] Something')).toBe('Something');
    expect(errorText('plain message')).toBe('plain message');
  });
});

describe('format', () => {
  it('formats elapsed days with each language’s plural rules', () => {
    const day = 86_400_000;
    expect(ago(0, day - 1)).toBe('23 h ago');
    expect(ago(0, day)).toBe('1 day ago');
    expect(ago(0, 2 * day)).toBe('2 days ago');
    i18n.lang = 'fr';
    expect(ago(0, day)).toBe('il y a 1 jour');
    i18n.lang = 'es';
    expect(ago(0, day)).toBe('hace 1 día');
    i18n.lang = 'ru';
    expect(ago(0, day)).toBe('1 день назад');
    expect(ago(0, 2 * day)).toBe('2 дня назад');
    expect(ago(0, 5 * day)).toBe('5 дней назад');
  });
  it('names role groups', () => {
    expect(roleName('mid+top')).toBe('Mid / Top');
    expect(roleName('')).toBe('');
    i18n.lang = 'zh';
    expect(roleName('adc')).toBe('下路');
  });

  it('names custom games', () => {
    expect(queueName(3100)).toBe('Custom');
    expect(queueName(3140)).toBe('Practice Tool');
    expect(queueName(450)).toBe('ARAM');
  });

  it('maps client positions', () => {
    expect(positionRole('utility')).toBe('support');
    expect(positionRole('MIDDLE')).toBe('mid');
    expect(positionRole('')).toBe('');
  });

  it('writes places', () => {
    expect([1, 2, 3, 4, 10, 11, 12, 13, 21, 22].map(ordinal)).toEqual(['1st', '2nd', '3rd', '4th', '10th', '11th', '12th', '13th', '21st', '22nd']);
    i18n.lang = 'zh';
    expect(ordinal(3)).toBe('第3');
  });

  it('pluralizes games and KDA', () => {
    expect(gamesText(1)).toBe('1 game');
    expect(gamesText(3)).toBe('3 games');
    expect(kdaRatio(5, 0, 3)).toBe('Perfect');
    expect(kdaRatio(5, 2, 3)).toBe('4.00');
  });

  it('writes the play-style and win-rate tags', () => {
    expect(tagText({ key: 'style', tone: 'info', arg: 'poke' })).toBe('Mostly poke champions');
    expect(tagText({ key: 'highWr', tone: 'pos', arg: 64 })).toBe('64% win rate');
    expect(tagText({ key: 'strongChamp', tone: 'pos', arg: [5, 86] })).toMatch(/86% wins$/);
    i18n.lang = 'zh';
    expect(tagText({ key: 'style', tone: 'info', arg: 'tank' })).toBe('常玩坦克');
    expect(tagText({ key: 'style', tone: 'info', arg: 'poke' })).toBe('常玩丢丢怪');
    expect(tagText({ key: 'lowWr', tone: 'neg', arg: 38 })).toBe('胜率38%');
  });

  it('writes tiers', () => {
    expect(tierText({ tier: 'GOLD', division: 'I' }, true)).toBe('Gold 1');
    expect(tierText({ tier: 'MASTER', division: 'I' })).toBe('Master');
    expect(tierText(null)).toBe('Unranked');
    expect(tierScore({ tier: 'EMERALD', division: 'IV' })).toBe(20);
    i18n.lang = 'zh';
    expect(tierText({ tier: 'PLATINUM', division: 'II' }, true)).toBe('铂金 2');
  });
});

describe('spell timers', () => {
  it('applies Ionian Boots summoner haste', () => {
    expect(cooldownFor(300, [], 'CLASSIC')).toBe(300);
    expect(cooldownFor(300, [3158], 'CLASSIC')).toBeCloseTo(272.7, 1);
  });
});

describe('numbers in the UI language', () => {
  afterEach(() => {
    i18n.lang = 'en';
  });
  it('writes short numbers its way, without a decimal past three digits', () => {
    i18n.lang = 'en';
    expect(compact(57_823_093)).toBe('57.8M');
    expect(compact(23_456)).toBe('23.5K');
    expect(compact(999)).toBe('999');
    i18n.lang = 'zh';
    expect(compact(57_823_093)).toBe('5782万');
    i18n.lang = 'ko';
    expect(compact(57_823_093)).toBe('5782만');
  });
  it('groups digits its way', () => {
    i18n.lang = 'en';
    expect(num(57_823)).toBe('57,823');
    i18n.lang = 'es';
    expect(num(57_823)).toBe('57.823');
  });
  it('writes decimals its way', () => {
    i18n.lang = 'fr';
    expect(kdaRatio(5, 2, 3)).toBe('4,00');
    expect(dec(54.66)).toBe('54,7');
  });
});
