<script lang="ts">
  /** Champ select in ARAM: Hextech ARAM win rates of your team's champions and the bench. */
  import Img from './Img.svelte';
  import { api } from '../lib/api';
  import { champIcon, champName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { ARAM_CREDIT, aramTier, bracket, pct1 } from '../lib/meta.svelte';
  import { app, go } from '../lib/state.svelte';
  import type { AramOverview } from '../lib/types';

  let overview = $state<AramOverview | null>(null);

  $effect(() => {
    const b = bracket();
    api.aramOverview(b).then(
      (o) => (overview = o),
      () => (overview = null),
    );
  });

  const rates = $derived(new Map((overview?.champions ?? []).map((c) => [c.championId, c.winRate])));
  const team = $derived(
    (app.live.teams.find((tm) => tm.ally)?.players ?? []).filter((p) => p.championId > 0).map((p) => ({ id: p.championId, me: p.me })),
  );
  const bench = $derived(app.live.bench.filter((id) => id > 0));
  /** The strongest champion you could take: on the bench or held by a teammate (swappable). */
  const best = $derived.by(() => {
    const pool = [...bench, ...team.filter((p) => !p.me).map((p) => p.id)];
    return pool.reduce((a, b) => ((rates.get(b) ?? 0) > (rates.get(a) ?? 0) ? b : a), pool[0] ?? 0);
  });
</script>

{#snippet champ(id: number, me = false)}
  {@const wr = rates.get(id)}
  <button class="c" class:me class:best={id === best && !me} onclick={() => go({ name: 'builds', championId: id, mode: 'aram' })} title={champName(id)}>
    <Img src={champIcon(id)} size={44} />
    {#if wr !== undefined}<span class="tier t{aramTier(wr)}">{aramTier(wr)}</span>{/if}
    <span class="nm">{me ? t('picks.you') : champName(id)}</span>
    <span class="num wr" class:hi={(wr ?? 0) >= 0.52} class:lo={wr !== undefined && wr < 0.48}>{wr !== undefined ? `${pct1(wr)}%` : '-'}</span>
  </button>
{/snippet}

{#if overview && (team.length || bench.length)}
  <section class="picks">
    <div class="head">
      <b>{t('picks.title')}</b>
      {#if best}<span class="hint">{t('picks.best')}: <b>{champName(best)}</b> {pct1(rates.get(best) ?? 0)}%</span>{/if}
    </div>
    <div class="rows">
      <div class="group">
        <div class="label">{t('picks.team')}</div>
        <div class="list">{#each team as p (p.id)}{@render champ(p.id, p.me)}{/each}</div>
      </div>
      {#if bench.length}
        <div class="group">
          <div class="label">{t('picks.bench')}</div>
          <div class="list">{#each bench as id (id)}{@render champ(id)}{/each}</div>
        </div>
      {/if}
    </div>
    <p class="credit">{t('meta.data')} {ARAM_CREDIT}</p>
  </section>
{/if}

<style>
  .picks {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 12px 14px;
    display: grid;
    gap: 10px;
    margin-bottom: 12px;
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
  .hint b {
    color: var(--win);
  }
  .rows {
    display: flex;
    gap: 24px;
    flex-wrap: wrap;
  }
  .label {
    font: 600 11px var(--font-score);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-bottom: 6px;
  }
  .list {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .c {
    position: relative;
    width: 76px;
    display: grid;
    justify-items: center;
    gap: 2px;
    padding: 6px 4px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    font-size: 11.5px;
  }
  .c:hover {
    background: var(--panel-3);
  }
  .c.me {
    box-shadow: inset 0 0 0 1px var(--you);
  }
  .c.best {
    box-shadow: inset 0 0 0 2px var(--win);
  }
  .nm {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-2);
  }
  .wr {
    font-weight: 600;
  }
  .tier {
    position: absolute;
    top: 4px;
    right: 6px;
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    font: 700 11px var(--font-score);
    color: var(--bg);
    background: var(--text-3);
  }
  .tier.tS {
    background: var(--death);
  }
  .tier.tA {
    background: var(--objective);
  }
  .tier.tB {
    background: var(--win);
  }
  .tier.tC {
    background: var(--text-2);
  }
  .hi {
    color: var(--win);
  }
  .lo {
    color: var(--death);
  }
  .credit {
    color: var(--text-3);
    font-size: 11px;
  }
</style>
