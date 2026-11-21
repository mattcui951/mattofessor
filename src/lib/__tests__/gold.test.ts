import { describe, expect, it } from 'vitest';
import { itemGold, teamGold } from '../gold';
import type { InGame, LivePlayer } from '../types';

const PRICES: Record<number, number> = { 3031: 3450, 3047: 1200, 2003: 50, 3340: 0, 1001: 300 };
const price = (id: number) => PRICES[id];

const player = (team: string, items: number[]): LivePlayer => ({ riotId: '', champion: '', team, level: 1, dead: false, respawn: 0, spells: [], items, me: false });

describe('team gold', () => {
  it('prices a mode-only item like the usual one', () => {
    expect(itemGold(3031, price)).toBe(3450);
    expect(itemGold(223031, price)).toBe(3450);
    expect(itemGold(999999, price), 'unknown everywhere').toBe(0);
    expect(itemGold(3340, price), 'a free trinket').toBe(0);
  });

  it('adds up each side', () => {
    const game: InGame = {
      gameTime: 600,
      mode: 'KIWI',
      myTeam: 'CHAOS',
      players: [player('ORDER', [3031, 3047]), player('ORDER', [2003]), player('CHAOS', [223031, 1001, 3340]), player('NEUTRAL', [3031])],
    };
    expect(teamGold(game, price)).toEqual({ order: 4700, chaos: 3750 });
  });
});
