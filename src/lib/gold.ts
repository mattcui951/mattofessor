/** The team gold strip's numbers: what each team's items are worth, from the game's live data. */
import type { InGame } from './types';

/**
 * An item's full price in gold, by the game's item id. Modes sell some items under ids of their
 * own (22xxxx, 12xxxx, 77xxxx...); those cost what the usual item (the last four digits) costs
 * when Data Dragon does not list them.
 */
export function itemGold(id: number, price: (id: number) => number | undefined): number {
  return price(id) ?? (id >= 100000 ? (price(id % 10000) ?? 0) : 0);
}

export type TeamGold = { order: number; chaos: number };

/** Each side's item value: every player's items as the scoreboard lists them. */
export function teamGold(game: InGame, price: (id: number) => number | undefined): TeamGold {
  const out: TeamGold = { order: 0, chaos: 0 };
  for (const p of game.players) {
    const value = p.items.reduce((sum, id) => sum + itemGold(id, price), 0);
    if (p.team === 'ORDER') out.order += value;
    else if (p.team === 'CHAOS') out.chaos += value;
  }
  return out;
}
