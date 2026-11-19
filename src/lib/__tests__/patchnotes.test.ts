import { describe, expect, it } from 'vitest';
import { noteBetter } from '../gamedata.svelte';

const line = (attr: string, before: string, after: string) => ({ text: '', attr, before, after });

describe('noteBetter', () => {
  it('Hextech ARAM modifiers: less damage taken helps', () => {
    expect(noteBetter(line('Damage Taken', '110%', '105%'))).toBe(true);
    expect(noteBetter(line('Damage Dealt', '100%', '95%'))).toBe(false);
  });

  it('stat lines: higher numbers are a buff', () => {
    expect(noteBetter(line('伤害', '80 / 120 / 160 / 200 / 240 (+80%法术强度)', '80 / 125 / 170 / 215 / 260 (+100%法术强度)'))).toBe(true);
    expect(noteBetter(line('Attack Range', '525', '550'))).toBe(true);
    expect(noteBetter(line('Base Armor', '38', '35'))).toBe(false);
  });

  it('cooldowns and costs: lower numbers are a buff, in any language', () => {
    expect(noteBetter(line('Cooldown', '14 / 13 / 12', '12 / 11 / 10'))).toBe(true);
    expect(noteBetter(line('冷却时间', '10', '12'))).toBe(false);
    expect(noteBetter(line('Délai de récupération', '1,5', '1,25'))).toBe(true);
  });

  it('says nothing when the numbers do not pair up or move both ways', () => {
    expect(noteBetter(line('提伯斯生命值', '1150 - 3500 (+50%法术强度)', '1275 - 4000 (+50%法术强度) (+150%额外生命值)'))).toBeUndefined();
    expect(noteBetter(line('Damage', '50 / 100', '60 / 90'))).toBeUndefined();
    expect(noteBetter({ text: 'Removed' })).toBeUndefined();
  });
});
