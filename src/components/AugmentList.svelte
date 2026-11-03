<script lang="ts">
  /** A champion's Hextech ARAM augments by rarity, best (difference from the champion's own win rate) first. */
  import Img from './Img.svelte';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { augment, bracket, delta, loadAugments, pct1 } from '../lib/meta.svelte';
  import type { AramChampion, AugRow } from '../lib/types';

  let { championId, per = 6 }: { championId: number; per?: number } = $props();

  let data = $state<AramChampion | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const id = championId;
    const b = bracket();
    loadAugments();
    error = null;
    api.aramChampion(id, b).then(
      (d) => {
        if (id === championId) data = d;
      },
      (e) => (error = String(e)),
    );
  });

  const groups = $derived.by(() => {
    const out: Record<string, AugRow[]> = { silver: [], gold: [], prismatic: [] };
    for (const a of data?.augments ?? []) {
      const r = augment(a.id)?.rarity;
      if (r && out[r].length < per) out[r].push(a);
    }
    return out;
  });
</script>

{#if error}
  <p class="dim">{error}</p>
{:else if !data}
  <p class="dim">{t('common.loading')}</p>
{:else if !data.augments.length}
  <p class="dim">{t('meta.noAugments')}</p>
{:else}
  <div class="rar">
    {#each ['silver', 'gold', 'prismatic'] as r (r)}
      <div class="col">
        <h4 class={r}>{t(`aug.${r}`)}</h4>
        {#each groups[r] as a, i (a.id)}
          {@const info = augment(a.id)}
          <div class="aug" class:top={i === 0} class:low={a.lowSample} title={a.lowSample ? t('meta.lowSample') : ''}>
            <Img src={info?.icon ?? null} size={26} />
            <span class="nm">{info?.name ?? `#${a.id}`}</span>
            <span class="st num">
              <b class:hi={a.delta > 0.01} class:lo={a.delta < -0.01}>{pct1(a.winRate)}%</b>
              <small>{delta(a.delta)} · {a.games.toLocaleString()}</small>
            </span>
          </div>
        {/each}
      </div>
    {/each}
  </div>
{/if}

<style>
  .rar {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
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
  h4.silver {
    color: var(--text-2);
  }
  h4.gold {
    color: var(--objective);
  }
  h4.prismatic {
    color: var(--assist);
  }
  .aug {
    display: grid;
    grid-template-columns: 26px minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    padding: 4px 6px;
    border-radius: var(--r-md);
    background: var(--panel-2);
  }
  .aug.top {
    box-shadow: inset 0 0 0 1px var(--win);
  }
  .aug.low {
    opacity: 0.6;
  }
  .nm {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
  @media (max-width: 760px) {
    .rar {
      grid-template-columns: 1fr;
    }
  }
</style>
