<script lang="ts">
  /**
   * Champ select in ARAM: Hextech ARAM win rates of your team's champions and the bench, each with
   * its tier and its best build's style (the three-item build it does best with, named after its
   * first item). In Hextech ARAM, your champion's three best augments too.
   */
  import Grade from './Grade.svelte';
  import Img from './Img.svelte';
  import { api } from '../lib/api';
  import { HEX_TTL, show } from '../lib/cache';
  import { champIcon, champName, itemIcon, itemName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { ARAM_CREDIT, aramTier, augTier, augment, bracket, delta, loadAugments, pct1 } from '../lib/meta.svelte';
  import { app, go } from '../lib/state.svelte';
  import type { AramOverview, AramPicks, AugRow, Board } from '../lib/types';

  /** `onpick`: a champion was clicked (default: open its ARAM build here). */
  let { onpick }: { onpick?: (championId: number) => void } = $props();
  const pick = (id: number) => (onpick ? onpick(id) : go({ name: 'builds', championId: id, mode: 'aram' }));

  let overview = $state<AramOverview | null>(null);
  let board = $state<Board | null>(null);
  let mine = $state<AramPicks | null>(null);

  $effect(() => {
    const b = bracket();
    show(`hex:overview:${b}`, HEX_TTL, () => api.aramOverview(b), (o) => (overview = o), () => (overview = null));
    show(`hex:board::${b}`, HEX_TTL, () => api.aramBoard('', b), (o) => (board = o), () => (board = null));
  });

  const rates = $derived(new Map((overview?.champions ?? []).map((c) => [c.championId, c.winRate])));
  /** Each champion's best three-item build's first item: its style. */
  const styles = $derived(new Map((board?.champions ?? []).filter((c) => c.best?.items.length).map((c) => [c.championId, c.best!.items[0]])));
  const team = $derived(
    (app.live.teams.find((tm) => tm.ally)?.players ?? []).filter((p) => p.championId > 0).map((p) => ({ id: p.championId, me: p.me })),
  );
  const bench = $derived(app.live.bench.filter((id) => id > 0));
  /** The strongest champion you could take: on the bench or held by a teammate (swappable). */
  const best = $derived.by(() => {
    const pool = [...bench, ...team.filter((p) => !p.me).map((p) => p.id)];
    return pool.reduce((a, b) => ((rates.get(b) ?? 0) > (rates.get(a) ?? 0) ? b : a), pool[0] ?? 0);
  });

  // Your champion's three best augments (Hextech ARAM only: classic ARAM has none). A derived id,
  // so the live updates (about once a second) do not refetch them.
  const myChamp = $derived(app.live.mode === 'KIWI' ? (team.find((p) => p.me)?.id ?? 0) : 0);
  $effect(() => {
    const id = myChamp;
    const b = bracket();
    mine = null;
    if (!id) return;
    loadAugments();
    show(`hex:picks:${id}:${b}`, HEX_TTL, () => api.aramPicks(id, b), (d) => {
      if (id === myChamp) mine = d;
    }, () => {});
  });
  const top: AugRow[] = $derived(
    (mine?.rarities ?? [])
      .flatMap((r) => r.take)
      .toSorted((a, b) => b.delta - a.delta)
      .slice(0, 3),
  );
</script>

{#snippet champ(id: number, me = false)}
  {@const wr = rates.get(id)}
  {@const style = styles.get(id)}
  <button class="c" class:me class:best={id === best && !me} onclick={() => pick(id)} title={champName(id)}>
    <Img src={champIcon(id)} size={44} />
    {#if wr !== undefined}<span class="tier"><Grade tier={aramTier(wr)} /></span>{/if}
    <span class="nm">{me ? t('picks.you') : champName(id)}</span>
    <span class="num wr" class:hi={(wr ?? 0) >= 0.52} class:lo={wr !== undefined && wr < 0.48}>{wr !== undefined ? `${pct1(wr)}%` : '-'}</span>
    {#if style}
      <span class="style" title={t('picks.styleHint', t('hex.style', itemName(style)))}>
        <Img src={itemIcon(style)} size={14} /><span>{t('hex.style', itemName(style))}</span>
      </span>
    {/if}
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
    {#if myChamp && top.length}
      <div class="mine">
        <span class="label">{t('picks.topAugs', champName(myChamp))}</span>
        {#each top as a (a.id)}
          {@const info = augment(a.id)}
          {@const tier = augTier(a)}
          <span class="aug" title={info?.name}>
            {#if tier}<Grade {tier} size={16} />{/if}
            <Img src={info?.icon ?? null} size={20} plate />
            <span class="an">{info?.name ?? `#${a.id}`}</span>
            <b class="num hi">{delta(a.delta)}</b>
          </span>
        {/each}
      </div>
    {/if}
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
    width: 88px;
    display: grid;
    justify-items: center;
    align-content: start;
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
    display: flex;
  }
  /* The champion's best build, named after its first item. */
  .style {
    max-width: 100%;
    display: grid;
    justify-items: center;
    gap: 2px;
    margin-top: 2px;
    font-size: 10.5px;
    line-height: 1.2;
    color: var(--text-3);
  }
  /* Up to two lines: item names are long ("Rite of Ruin build"). */
  .style span {
    max-width: 100%;
    text-align: center;
    overflow: hidden;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .mine {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 10px;
    padding-top: 8px;
    border-top: 1px solid var(--seam);
  }
  .mine .label {
    margin: 0 4px 0 0;
  }
  .aug {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 9px 3px 4px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    font-size: 12.5px;
  }
  .an {
    max-width: 12em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
