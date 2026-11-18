<script lang="ts">
  /** Champion × augment pairs, ranked (AramHex: every pair with enough games). */
  import Img from './Img.svelte';
  import { num } from '../lib/format';
  import { tip } from '../lib/hover.svelte';
  import { augmentTip } from '../lib/tips.svelte';
  import { loadGameData } from '../lib/gamedata.svelte';
  import { champIcon, champName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { augment, delta, pct1 } from '../lib/meta.svelte';
  import { go } from '../lib/state.svelte';
  import type { Combos } from '../lib/types';

  type Sort = 'win' | 'delta' | 'games';
  let { combos, sort, onsort }: { combos: Combos; sort: Sort; onsort: (s: Sort) => void } = $props();
  loadGameData();
</script>

{#snippet th(key: Sort, label: string)}
  <th class="r"><button class="sort" class:on={sort === key} aria-pressed={sort === key} onclick={() => onsort(key)}>{label}{#if sort === key}<span>↓</span>{/if}</button></th>
{/snippet}

<table>
  <thead>
    <tr>
      <th class="n">#</th>
      <th>{t('hex.combo')}</th>
      {@render th('win', t('hex.win'))}
      {@render th('delta', t('hex.deltaLong'))}
      {@render th('games', t('hex.games'))}
    </tr>
  </thead>
  <tbody>
    {#each combos.rows as c, i (`${c.championId}-${c.augmentId}`)}
      {@const a = augment(c.augmentId)}
      <tr onclick={() => go({ name: 'builds', championId: c.championId, mode: 'aram' })}>
        <td class="n num">{i + 1}</td>
        <td>
          <span class="pair">
            <span class="who"><Img src={champIcon(c.championId)} size={28} />{champName(c.championId)}</span>
            <span class="x">×</span>
            <span class="who" use:tip={() => augmentTip(c.augmentId, [{ label: t('hex.win'), value: `${pct1(c.winRate)}%` }, { label: t('hex.deltaLong'), value: delta(c.delta) }, { label: t('hex.games'), value: num(c.games) }])}><Img src={a?.icon ?? null} size={28} round /><span>{a?.name ?? `#${c.augmentId}`}</span></span>
          </span>
        </td>
        <td class="num r">{pct1(c.winRate)}%</td>
        <td class="num r" class:hi={c.delta > 0.005} class:lo={c.delta < -0.005}>{delta(c.delta)}</td>
        <td class="num r dim">{num(c.games)}</td>
      </tr>
    {:else}
      <tr class="static"><td colspan="5" class="empty">{t('hex.noCombos')}</td></tr>
    {/each}
  </tbody>
</table>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel-2);
    text-align: left;
    font: 600 11.5px var(--font-score);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
    padding: 8px 10px;
  }
  th.r,
  td.r {
    text-align: right;
  }
  td {
    padding: 6px 10px;
    border-top: 1px solid var(--seam);
  }
  tbody tr {
    cursor: pointer;
  }
  tbody tr.static {
    cursor: default;
  }
  tbody tr:hover {
    background: var(--panel-2);
  }
  .n {
    width: 44px;
    color: var(--text-3);
  }
  .sort {
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    display: inline-flex;
    gap: 4px;
  }
  .sort.on {
    color: var(--you);
  }
  .pair {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 500;
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .x {
    color: var(--text-3);
  }
  .hi {
    color: var(--win);
  }
  .lo {
    color: var(--death);
  }
  .dim {
    color: var(--text-3);
  }
  .empty {
    padding: 24px;
    color: var(--text-3);
  }
</style>
