/**
 * Hover cards for an item or an augment: name, kind, what it does (the game's text, every stat
 * and effect), plus whatever numbers the page adds.
 */
import { dd, itemIcon, itemName } from './dd.svelte';
import { bi, gameData } from './gamedata.svelte';
import type { HoverInfo } from './hover.svelte';
import { t } from './i18n.svelte';
import { augment } from './meta.svelte';

/** An augment's description with its numbers (the refreshed data), in the UI language. */
export function augmentText(id: number): string {
  return bi(gameData.data?.mayhem.augments?.[String(id)]);
}

/** An item's stats and effects (Data Dragon, in the UI language). */
export function itemText(id: number): string {
  return dd.data?.items[String(id)]?.desc ?? '';
}

export function itemTip(id: number, stats: HoverInfo['stats'] = [], more: Partial<HoverInfo> = {}): HoverInfo {
  const it = dd.data?.items[String(id)];
  return {
    icon: itemIcon(id),
    title: itemName(id),
    sub: it?.gold ? t('tips.gold', it.gold) : t('hex.itemKind'),
    body: itemText(id),
    stats,
    ...more,
  };
}

export function augmentTip(id: number, stats: HoverInfo['stats'] = [], more: Partial<HoverInfo> = {}): HoverInfo {
  const a = augment(id);
  return {
    icon: a?.icon ?? null,
    round: true,
    plate: true,
    title: a?.name ?? `#${id}`,
    sub: a?.rarity ? t(`aug.${a.rarity}`) : '',
    subTone: a?.rarity || '',
    body: augmentText(id),
    stats,
    ...more,
  };
}
