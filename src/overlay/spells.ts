/** Ionian Boots of Lucidity give summoner spell haste (the only source visible on enemies). */
const SUMMONER_HASTE_ITEMS: Record<number, number> = { 3158: 10 };

/** Cooldown in seconds after a spell is used, from its base cooldown and the owner's items. */
export function cooldownFor(base: number, items: number[], _mode: string): number {
  const haste = items.reduce((h, id) => h + (SUMMONER_HASTE_ITEMS[id] ?? 0), 0);
  return base * (100 / (100 + haste));
}
