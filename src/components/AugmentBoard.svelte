<script lang="ts">
  /**
   * Every augment on one board: a column per rarity, a row per lift (how much better its players
   * did than their champion usually does, so a strong champion's pick does not look strong by
   * itself). The augments the patch before had and this one does not are listed apart.
   */
  import Img from './Img.svelte';
  import { num } from '../lib/format';
  import { tip, type HoverInfo } from '../lib/hover.svelte';
  import { loadGameData } from '../lib/gamedata.svelte';
  import { augmentTip } from '../lib/tips.svelte';
  import { t } from '../lib/i18n.svelte';
  import { augment, delta, pct1, share } from '../lib/meta.svelte';
  import type { AugBoard, AugStat } from '../lib/types';

  let { board, matches = null, onpick }: { board: AugBoard; matches?: Set<number> | null; onpick: (id: number) => void } = $props();

  const RARITIES = ['prismatic', 'gold', 'silver'] as const;
  /** Lift lines in points, best first. */
  const LINES = [0.04, 0.02, 0, -0.02, -Infinity];
  const TONES = ['tS', 'tA', 'tB', 'tC', 'tD'];

  let showRemoved = $state(false);

  const known = $derived(board.augments.filter((a) => augment(a.id)?.rarity));
  const rows = $derived(
    LINES.map((min, i) => {
      const max = i ? LINES[i - 1] : Infinity;
      const inRow = known.filter((a) => a.delta >= min && a.delta < max);
      return { min, max, tone: TONES[i], cells: RARITIES.map((r) => inRow.filter((a) => augment(a.id)?.rarity === r)) };
    }),
  );
  const pts = (v: number) => (v > 0 ? '+' : v < 0 ? '−' : '') + Math.abs(Math.round(v * 100));
  const range = (min: number, max: number) => (max === Infinity ? `≥${pts(min)}` : min === -Infinity ? `<${pts(max)}` : `${pts(min)} ~ ${pts(max)}`);

  loadGameData();

  function hoverOf(a: AugStat, removed = false): HoverInfo {
    return augmentTip(a.id, [
        { label: t('hex.win'), value: `${pct1(a.winRate)}%` },
        { label: t('hex.lift'), value: delta(a.delta), tone: a.delta > 0.005 ? 'up' : a.delta < -0.005 ? 'down' : '' },
        { label: t('hex.held'), value: `${share(a.pickRate)}%` },
        { label: t('hex.games'), value: num(a.games) },
      ],
      { note: removed ? t('hex.removedNote', board.previousPatch ?? '') : a.lowSample ? t('meta.lowSample') : t('hex.augClick') },
    );
  }
</script>

<div class="wrap">
  <div class="top">
    <p class="hint">{t('hex.augBoardHint')}</p>
    {#if board.removed.length}
      <button class="btn sm" aria-expanded={showRemoved} onclick={() => (showRemoved = !showRemoved)}>{t('hex.removed', board.removed.length)} {showRemoved ? '▴' : '▾'}</button>
    {/if}
  </div>
  {#if showRemoved && board.removed.length}
    <div class="removed">
      <p class="hint">{t('hex.removedHint', board.previousPatch ?? '')}</p>
      <div class="tiles">
        {#each board.removed as a (a.id)}
          <span class="pt gone" use:tip={() => hoverOf(a, true)} role="img" aria-label={augment(a.id)?.name ?? ''}>
            <Img src={augment(a.id)?.icon ?? null} size={34} round />
            <span class="v num">{pct1(a.winRate)}</span>
          </span>
        {/each}
      </div>
    </div>
  {/if}

  <div class="board" role="table" aria-label={t('hex.augBoard')}>
    <div class="hd corner" role="columnheader">{t('hex.lift')}</div>
    {#each RARITIES as r (r)}<div class="hd" role="columnheader"><span class="rar-{r}">{t(`aug.${r}`)}</span></div>{/each}
    {#each rows as row (row.min)}
      <div class="band {row.tone}" role="rowheader"><b class="num">{range(row.min, row.max)}</b></div>
      {#each row.cells as cell, ci (ci)}
        <div class="cell" role="cell">
          {#each cell as a (a.id)}
            <button class="pt {row.tone}" class:dim={matches && !matches.has(a.id)} class:low={a.lowSample} aria-label={augment(a.id)?.name ?? ''} use:tip={() => hoverOf(a)} onclick={() => onpick(a.id)}>
              <Img src={augment(a.id)?.icon ?? null} size={34} round />
              <span class="v num">{delta(a.delta)}</span>
            </button>
          {/each}
        </div>
      {/each}
    {/each}
  </div>
</div>

<style>
  .wrap {
    display: grid;
    gap: 10px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .hint {
    flex: 1;
    margin: 0;
    color: var(--text-3);
    font-size: 12px;
  }
  .removed {
    padding: 10px 12px;
    border-radius: var(--r-lg);
    background: var(--panel);
    border: 1px dashed var(--seam-strong);
  }
  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
  .board {
    display: grid;
    grid-template-columns: 84px repeat(3, minmax(0, 1fr));
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
    place-items: center;
    padding: 8px 4px;
    border-top: 1px solid var(--seam);
    background: color-mix(in oklab, var(--tone) 10%, var(--panel));
    box-shadow: inset 3px 0 0 var(--tone);
  }
  .band b {
    font-size: 12.5px;
    color: var(--tone);
  }
  .tS {
    --tone: var(--tier-S);
  }
  .tA {
    --tone: var(--tier-A);
  }
  .tB {
    --tone: var(--tier-B);
  }
  .tC {
    --tone: var(--tier-C);
  }
  .tD {
    --tone: var(--tier-D);
  }
  .cell {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 4px;
    padding: 6px;
    border-top: 1px solid var(--seam);
    border-left: 1px solid var(--seam);
    min-height: 54px;
  }
  .pt {
    display: grid;
    justify-items: center;
    gap: 1px;
    padding: 2px;
    border-radius: var(--r-md);
    outline: none;
  }
  .pt {
    transition: background 120ms, transform 140ms var(--ease-out);
  }
  .pt:hover,
  .pt:focus-visible {
    background: var(--panel-3);
    transform: translateY(-2px);
  }
  .pt.dim {
    opacity: 0.22;
  }
  .pt.low {
    opacity: 0.55;
  }
  .pt.gone {
    filter: grayscale(0.7);
  }
  .v {
    font-size: 11px;
    font-weight: 700;
    color: var(--tone, var(--text-2));
    line-height: 1.1;
  }
</style>
