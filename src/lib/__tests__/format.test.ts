import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api', () => ({ api: {} }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import { _dicts, i18n } from '../i18n.svelte';
import { gamesText, kdaRatio, ordinal, positionRole, roleName, tagText } from '../format';
import { tierScore, tierText } from '../dd.svelte';
import { errorText } from '../state.svelte';
import { cooldownFor } from '../../overlay/spells';

beforeEach(() => {
  i18n.lang = 'en';
});

describe('i18n', () => {
  it('has every key in both languages with the same placeholders', () => {
    for (const [key, en] of Object.entries(_dicts.en)) {
      const zh = (_dicts.zh as Record<string, string>)[key];
      expect(zh, key).toBeTruthy();
      expect((zh.match(/\{\d\}/g) ?? []).sort(), key).toEqual((en.match(/\{\d\}/g) ?? []).sort());
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
  it('names role groups', () => {
    expect(roleName('mid+top')).toBe('Mid / Top');
    expect(roleName('')).toBe('');
    i18n.lang = 'zh';
    expect(roleName('adc')).toBe('下路');
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
