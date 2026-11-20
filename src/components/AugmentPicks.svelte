<script lang="ts">
  /**
   * Hextech ARAM augment recommendations for one champion: by rarity, the augments that raise its
   * win rate the most (take) and lower it the most (avoid). `compact` is the in-game overlay form:
   * the four best of each rarity, nothing to avoid.
   */
  import Img from './Img.svelte';
  import { num } from '../lib/format';
  import { api } from '../lib/api';
  import { HEX_TTL, show } from '../lib/cache';
  import { champName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { ARAM_CREDIT, augment, bracket, delta, loadAugments, pct1 } from '../lib/meta.svelte';
  import type { AramPicks, AugRow } from '../lib/types';

  let { championId, compact = false }: { championId: number; compact?: boolean } = $props();

  let data = $state<AramPicks | null>(null);
  let error = $state<string | null>(null);
  // The parent passes a field of the live state, which is replaced on every live update (about
  // once a second): a derived only changes when the id does, so the list is not refetched (and
  // shown as loading) every second.
  const champ = $derived(championId);

  $effect(() => {
    const id = champ;
    const b = bracket();
    loadAugments();
    data = null;
    error = null;
    show(
      `hex:picks:${id}:${b}`,
      HEX_TTL,
      () => api.aramPicks(id, b),
      (d) => {
        if (id === championId) data = d;
      },
      (e) => {
        if (id === championId) error = String(e);
      },
    );
  });

</script>

<!-- The in-game panel's form: rows only as wide as their text, the number right after the name. -->
{#snippet line(a: AugRow)}
  {@const info = augment(a.id)}
  <div class="ln" title={t('recs.games', num(a.games))}>
    <Img src={info?.icon ?? null} size={20} />
    <span class="nm">{info?.name ?? `#${a.id}`}</span>
    <b class="num hi">{delta(a.delta)}</b>
    <small class="num">{pct1(a.winRate)}%</small>
  </div>
{/snippet}

{#snippet row(a: AugRow, bad: boolean)}
  {@const info = augment(a.id)}
  <div class="aug" class:bad>
    <Img src={info?.icon ?? null} size={compact ? 22 : 26} />
    <span class="nm">{info?.name ?? `#${a.id}`}</span>
    <span class="st num" title={t('recs.games', num(a.games))}>
      <b class:hi={!bad} class:lo={bad}>{delta(a.delta)}</b>
      {#if !compact}<small>{pct1(a.winRate)}%</small>{/if}
    </span>
  </div>
{/snippet}

<section class="recs" class:compact>
  <div class="head">
    <b>{t('recs.title')}</b>
    {#if data?.champion}
      <span class="hint">{champName(championId)} · {t('recs.base', pct1(data.champion.winRate))}</span>
    {/if}
  </div>
  {#if error}
    <p class="dim">{error}</p>
  {:else if !data}
    <!-- Three rarities of rows, as the list will be. -->
    <div class="wait" aria-busy="true" aria-label={t('common.loading')}>
      {#each Array(3) as _, i (i)}<div>{#each Array(4) as _, j (j)}<span class="skel" style="height:{compact ? 24 : 34}px"></span>{/each}</div>{/each}
    </div>
  {:else if data.rarities.every((r) => !r.take.length && !r.avoid.length)}
    <p class="dim">{t('meta.noAugments')}</p>
  {:else if compact}
    <div class="tight">
      {#each data.rarities as r (r.rarity)}
        <div class="rcol">
          <h4><span class="rar-{r.rarity}">{t(`aug.${r.rarity}`)}</span></h4>
          <div class="list">
            {#each r.take.slice(0, 4) as a (a.id)}{@render line(a)}{/each}
            {#if !r.take.length}<p class="dim small">{t('recs.noneTake')}</p>{/if}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="rar">
      {#each data.rarities as r (r.rarity)}
        <div class="col">
          <h4><span class="rar-{r.rarity}">{t(`aug.${r.rarity}`)}</span></h4>
          {#each r.take.slice(0, 4) as a (a.id)}{@render row(a, false)}{/each}
          {#if !r.take.length}<p class="dim small">{t('recs.noneTake')}</p>{/if}
          {#if r.avoid.length}
            <div class="sub">{t('recs.avoid')}</div>
            {#each r.avoid as a (a.id)}{@render row(a, true)}{/each}
          {/if}
        </div>
      {/each}
    </div>
    <p class="note">{t('recs.hint')} · {ARAM_CREDIT}</p>
  {/if}
</section>

<style>
  .wait {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  .wait > div {
    display: grid;
    gap: 6px;
    align-content: start;
  }
  .recs {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 12px 14px;
    display: grid;
    gap: 10px;
    margin-bottom: 12px;
  }
  .recs.compact {
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--seam);
    border-radius: 0;
    padding: 8px 12px 10px;
    gap: 8px;
    margin-bottom: 0;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 12px;
    flex-wrap: wrap;
  }
  .hint {
    color: var(--text-2);
    font-size: 13px;
  }
  .rar {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  .compact .rar {
    gap: 8px;
  }
  .col {
    display: grid;
    gap: 5px;
    align-content: start;
    min-width: 0;
  }
  h4 {
    margin: 0 0 2px;
    font: 700 11.5px var(--font-score);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .sub {
    margin-top: 6px;
    font: 600 11px var(--font-score);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .aug {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    padding: 4px 6px;
    border-radius: var(--r-md);
    background: var(--panel-2);
  }
  .aug.bad {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--seam);
  }
  .nm {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .compact .nm {
    font-size: 12.5px;
  }
  .st {
    display: grid;
    justify-items: end;
    font-size: 12.5px;
    line-height: 1.2;
  }
  .st small {
    color: var(--text-3);
    font-size: 11px;
  }
  /* Compact (in-game): a column of the best augments per rarity. */
  .tight {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 36px;
  }
  .rcol {
    display: grid;
    gap: 3px;
    align-content: start;
  }
  .list {
    display: grid;
    grid-template-columns: 20px fit-content(9.5em) auto auto;
    gap: 3px 0;
    align-content: start;
  }
  .ln {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: center;
    column-gap: 7px;
    padding: 3px 8px 3px 4px;
    border-radius: var(--r-md);
    background: var(--panel-2);
  }
  .ln .nm {
    font-size: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ln b {
    font-size: 12.5px;
    text-align: right;
  }
  .ln small {
    font-size: 11px;
    color: var(--text-3);
    text-align: right;
  }
  .hi {
    color: var(--win);
  }
  .lo {
    color: var(--death);
  }
  .dim {
    color: var(--text-3);
    font-size: 13px;
  }
  .dim.small {
    font-size: 12px;
  }
  .note {
    color: var(--text-3);
    font-size: 11px;
  }
  @media (max-width: 760px) {
    .rar {
      grid-template-columns: 1fr;
    }
  }
</style>
