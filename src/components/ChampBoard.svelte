<script lang="ts">
  /**
   * Every champion on one board: a column per class (Data Dragon's first class), a row per win
   * rate tier (`TIER_LINES`). ▲▼ marks a move of a point or more since the patch before. "By
   * best build" puts each champion's best three-item build's first item on its icon.
   */
  import Img from './Img.svelte';
  import { num } from '../lib/format';
  import { champIcon, champName, dd, itemIcon, itemName } from '../lib/dd.svelte';
  import { tip, type HoverInfo } from '../lib/hover.svelte';
  import { t } from '../lib/i18n.svelte';
  import { TIER_LINES, delta, pct1, share } from '../lib/meta.svelte';
  import { go } from '../lib/state.svelte';
  import type { Board, BoardChamp, ChampClass } from '../lib/types';

  let { board, builds = false, matches = null }: { board: Board; builds?: boolean; matches?: Set<number> | null } = $props();

  const CLASSES: ChampClass[] = ['Tank', 'Fighter', 'Assassin', 'Marksman', 'Mage', 'Support'];
  /** A move this big since the patch before gets a mark. */
  const MOVED = 0.01;

  const cls = (id: number): ChampClass => (dd.data?.champions[String(id)]?.tags[0] as ChampClass) ?? 'Fighter';
  const rows = $derived(
    TIER_LINES.map((l, i) => {
      const max = i ? TIER_LINES[i - 1].min : Infinity;
      const inRow = board.champions.filter((c) => c.winRate >= l.min && c.winRate < max);
      return { ...l, max, cells: CLASSES.map((k) => inRow.filter((c) => cls(c.championId) === k)) };
    }),
  );
  const range = (min: number, max: number) => (max === Infinity ? `≥${pct1(min)}%` : min === 0 ? `<${pct1(max)}%` : `${pct1(min)}-${pct1(max)}%`);
  const move = (c: BoardChamp) => (c.previous === null ? 0 : c.winRate - c.previous >= MOVED ? 1 : c.previous - c.winRate >= MOVED ? -1 : 0);

  function hoverOf(c: BoardChamp): HoverInfo {
    const stats: HoverInfo['stats'] = [
      { label: t('hex.win'), value: `${pct1(c.winRate)}%` },
      { label: t('hex.rank'), value: `${c.rank} / ${board.champions.length}` },
      { label: t('hex.games'), value: num(c.games) },
      { label: t('hex.playRate'), value: `${share(c.pickRate)}%` },
    ];
    if (c.previous !== null && board.previousPatch) {
      const d = c.winRate - c.previous;
      stats.push({ label: t('hex.since', board.previousPatch), value: delta(d), tone: d >= MOVED ? 'up' : d <= -MOVED ? 'down' : '' });
    }
    if (builds && c.best) {
      stats.push({ label: t('hex.bestBuild'), value: `${pct1(c.best.winRate)}%` });
    }
    return {
      icon: champIcon(c.championId),
      title: champName(c.championId),
      sub: t(`class.${cls(c.championId)}`),
      stats,
      note: builds && c.best ? c.best.items.map((id) => itemName(id)).join(' › ') : undefined,
    };
  }
</script>

<div class="board" role="table" aria-label={t('hex.board')}>
  <div class="hd corner" role="columnheader">{t('hex.win')}</div>
  {#each CLASSES as k (k)}<div class="hd" role="columnheader">{t(`class.${k}`)}</div>{/each}
  {#each rows as r (r.tier)}
    <div class="band t{r.tier}" role="rowheader">
      <b>{t(`tiers.${r.tier}`)}</b>
      <small class="num">{range(r.min, r.max)}</small>
    </div>
    {#each r.cells as cell, ci (ci)}
      <div class="cell" role="cell">
        {#each cell as c (c.championId)}
          {@const m = move(c)}
          <button class="pt" class:dim={matches && !matches.has(c.championId)} aria-label="{champName(c.championId)} {pct1(c.winRate)}%" use:tip={() => hoverOf(c)} onclick={() => go({ name: 'builds', championId: c.championId, mode: 'aram' })}>
            <Img src={champIcon(c.championId)} size={40} />
            {#if builds && c.best}<span class="bi"><Img src={itemIcon(c.best.items[0])} size={18} /></span>{/if}
            {#if m}<span class="mv" class:up={m > 0} class:down={m < 0} aria-hidden="true">{m > 0 ? '▲' : '▼'}</span>{/if}
            <span class="wr num t{r.tier}">{pct1(c.winRate)}</span>
          </button>
        {/each}
      </div>
    {/each}
  {/each}
</div>

<style>
  .board {
    display: grid;
    grid-template-columns: 74px repeat(6, minmax(0, 1fr));
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    background: var(--panel);
    overflow: hidden;
  }
  .hd {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 8px 6px;
    text-align: center;
    background: var(--panel-2);
    border-bottom: 1px solid var(--seam);
    font: 600 12px var(--font-score);
    letter-spacing: 0.06em;
    color: var(--text-2);
  }
  .band {
    display: grid;
    align-content: center;
    justify-items: center;
    gap: 2px;
    padding: 8px 4px;
    border-top: 1px solid var(--seam);
    background: color-mix(in oklab, var(--tone) 10%, var(--panel));
    box-shadow: inset 3px 0 0 var(--tone);
    --tone: var(--text-3);
  }
  .band b {
    font: 700 14px var(--font-score);
    color: var(--tone);
  }
  .band small {
    color: var(--text-3);
    font-size: 10.5px;
  }
  .tS {
    --tone: var(--death);
  }
  .tA {
    --tone: var(--objective);
  }
  .tB {
    --tone: var(--win);
  }
  .tC {
    --tone: var(--you);
  }
  .tD {
    --tone: var(--text-3);
  }
  .cell {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 4px;
    padding: 6px;
    border-top: 1px solid var(--seam);
    border-left: 1px solid var(--seam);
    min-height: 58px;
  }
  .pt {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 1px;
    padding: 2px;
    border-radius: var(--r-md);
  }
  .pt:hover,
  .pt:focus-visible {
    background: var(--panel-3);
  }
  .pt.dim {
    opacity: 0.22;
  }
  .wr {
    font-size: 11px;
    font-weight: 700;
    color: var(--tone);
    line-height: 1.1;
  }
  .bi {
    position: absolute;
    top: 0;
    left: 0;
    border-radius: var(--r-sm);
    box-shadow: 0 0 0 2px var(--panel);
    line-height: 0;
  }
  .mv {
    position: absolute;
    top: 1px;
    right: 1px;
    font-size: 9px;
    line-height: 1;
    padding: 1px 2px;
    border-radius: 2px;
    background: var(--panel);
  }
  .mv.up {
    color: var(--win);
  }
  .mv.down {
    color: var(--death);
  }
</style>
