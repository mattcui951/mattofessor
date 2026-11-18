<script lang="ts" module>
  import type { StatSort } from './StatGrid.svelte';

  // Sort orders and toggles, kept while the app runs (the same on every champion).
  const sorts = $state<Record<string, StatSort>>({ prismatic: 'pick', gold: 'pick', silver: 'pick', items: 'pick', bad: 'pick', boots: 'pick', spells: 'pick' });
  const opts = $state({ small: false, allAugs: false });
</script>

<script lang="ts">
  /**
   * A champion's Hextech ARAM page from AramHex: augments by rarity, build styles and three-item
   * routes, boots and summoner spells, finished items and the common ones that do worse, its
   * recent changes, and its win rate by skill bracket and by patch. Follows the shared patch
   * and bracket (`HexFilter`).
   */
  import BandChart from './BandChart.svelte';
  import BandPicks from './BandPicks.svelte';
  import Img from './Img.svelte';
  import MayhemChanges from './MayhemChanges.svelte';
  import MayhemMods from './MayhemMods.svelte';
  import StatGrid, { type StatCard } from './StatGrid.svelte';
  import { api } from '../lib/api';
  import { dd, itemIcon, itemName, spell, spellIcon } from '../lib/dd.svelte';
  import { tip, type HoverInfo } from '../lib/hover.svelte';
  import { augmentTip, itemText } from '../lib/tips.svelte';
  import { loadGameData } from '../lib/gamedata.svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { ARAM_CREDIT, augment, bracket, delta, hex, loadAugments, notePatches, pct1, share } from '../lib/meta.svelte';
  import { errorText } from '../lib/state.svelte';
  import type { AugStat, ChampionPage, Core, ItemStat, SpellStat } from '../lib/types';
  import TrendChart from './TrendChart.svelte';

  let { championId, page = $bindable(null) }: { championId: number; page?: ChampionPage | null } = $props();

  loadGameData();
  let error = $state<string | null>(null);
  let run = 0;
  $effect(() => {
    const id = championId;
    const p = hex.patch;
    const b = bracket();
    const mine = ++run;
    loadAugments();
    error = null;
    api.aramChampionPage(id, p, b).then(
      (d) => {
        if (mine !== run) return;
        page = d;
        notePatches(d.patches);
      },
      (e) => {
        if (mine === run) error = errorText(e);
      },
    );
  });

  const base = $derived(page?.champion?.winRate ?? 0.5);
  /** Every number a card's hover shows. */
  function stats(s: { winRate: number; delta: number; pickRate: number; games: number }, pickLabel = t('hex.pick')): HoverInfo['stats'] {
    return [
      { label: t('hex.win'), value: `${pct1(s.winRate)}%`, tone: s.winRate > base + 0.005 ? 'up' : s.winRate < base - 0.005 ? 'down' : '' },
      { label: t('hex.deltaLong'), value: delta(s.delta), tone: s.delta > 0.005 ? 'up' : s.delta < -0.005 ? 'down' : '' },
      { label: pickLabel, value: `${share(s.pickRate)}%` },
      { label: t('hex.games'), value: s.games.toLocaleString() },
    ];
  }
  const small = (low: boolean) => (low ? t('meta.lowSample') : undefined);

  // Augments, by rarity: the most taken (all of them with "show all").
  const PER_RARITY = 12;
  function augCards(r: 'prismatic' | 'gold' | 'silver'): StatCard[] {
    const list = (page?.augments ?? []).filter((a) => augment(a.id)?.rarity === r && (opts.small || !a.lowSample));
    return (opts.allAugs ? list : list.slice(0, PER_RARITY)).map(augCard);
  }
  function augCard(a: AugStat): StatCard {
    const info = augment(a.id);
    const name = info?.name ?? `#${a.id}`;
    return {
      key: a.id,
      icon: info?.icon ?? null,
      round: true,
      name,
      winRate: a.winRate,
      delta: a.delta,
      pickRate: a.pickRate,
      hover: augmentTip(a.id, stats(a, t('hex.held')), { title: name, note: small(a.lowSample) }),
    };
  }

  /** Data Dragon's name, or AramHex's (Chinese) for an item it does not have yet. */
  const known = (id: number) => Boolean(dd.data?.items[String(id)]);
  const iname = (i: { id: number; name: string }) => (i.id === 0 ? t('hex.noBoots') : known(i.id) ? itemName(i.id) : i18n.lang === 'zh' && i.name ? i.name : t('hex.item', i.id));
  const iicon = (i: { id: number; icon: string }) => (i.id === 0 ? null : known(i.id) ? itemIcon(i.id) : i.icon || null);
  const byId = $derived(new Map((page?.items ?? []).map((i) => [i.id, i])));

  function itemCard(i: ItemStat, on = false, onclick?: () => void): StatCard {
    const name = iname(i);
    const s = stats(i);
    if (i.id !== 0) s.push({ label: t('hex.without'), value: `${pct1(i.without)}%`, tone: i.without > base + 0.005 ? 'up' : i.without < base - 0.005 ? 'down' : '' });
    return { key: i.id, icon: iicon(i), glyph: i.id === 0 ? '∅' : undefined, name, winRate: i.winRate, delta: i.delta, pickRate: i.pickRate, on, onclick, hover: { icon: i.id === 0 ? undefined : iicon(i), title: name, sub: i.id === 0 ? '' : t('hex.itemKind'), body: i.id === 0 ? undefined : itemText(i.id), stats: s, note: i.id === 0 ? t('hex.noBootsNote') : small(i.lowSample) } };
  }
  function spellCard(s: SpellStat): StatCard {
    const name = s.ids.map((id) => spell(id)?.name ?? `#${id}`).join(' + ');
    return { key: s.ids.join('+'), icon: spellIcon(s.ids[0]), icon2: spellIcon(s.ids[1]), name, winRate: s.winRate, delta: s.delta, pickRate: s.pickRate, hover: { title: name, stats: stats(s), note: small(s.lowSample) } };
  }

  /** Common items that do worse: made by at least 4% of the champion's players, 2.5+ points under it. */
  const BAD_PICK = 0.04;
  const BAD_DELTA = -0.025;
  const reliable = $derived((page?.items ?? []).filter((i) => opts.small || !i.lowSample));
  const bad = $derived(reliable.filter((i) => i.pickRate >= BAD_PICK && i.delta <= BAD_DELTA));

  // Build styles: the three-item routes grouped by their first item.
  type Style = { item: number; games: number; wins: number; cores: Core[] };
  const styles = $derived.by((): Style[] => {
    const m = new Map<number, Style>();
    for (const c of page?.cores ?? []) {
      const first = c.items[0];
      const s = m.get(first) ?? { item: first, games: 0, wins: 0, cores: [] };
      s.games += c.games;
      s.wins += c.games * c.winRate;
      s.cores.push(c);
      m.set(first, s);
    }
    return [...m.values()].sort((a, b) => b.games - a.games);
  });
  let style = $state<number | null>(null);
  $effect(() => {
    championId;
    style = null;
  });
  const routes = $derived((style === null ? (page?.cores ?? []) : (styles.find((s) => s.item === style)?.cores ?? [])).toSorted((a, b) => b.winRate - a.winRate));
  const bestRoute = $derived(Math.max(0, ...(page?.cores ?? []).map((c) => c.winRate)));
  const itemOf = (id: number) => byId.get(id) ?? { id, name: '', icon: '' };

  const bandBars = $derived(
    (page?.bands ?? [])
      .filter((b) => b.bracket !== 'all')
      .map((b) => ({ label: t(`bracket.${b.bracket}`), value: b.winRate, hover: { title: t(`bracket.${b.bracket}`), sub: t(`bracket.${b.bracket}Hint`), stats: [{ label: t('hex.win'), value: `${pct1(b.winRate)}%` }, { label: t('hex.games'), value: b.games.toLocaleString() }] } })),
  );
  const allPlayers = $derived(page?.bands.find((b) => b.bracket === 'all')?.winRate ?? null);
  const trend = $derived((page?.history ?? []).map((h) => ({ label: h.patch, value: h.winRate, hover: { title: h.patch, stats: [{ label: t('hex.win'), value: `${pct1(h.winRate)}%` }, { label: t('hex.games'), value: h.games.toLocaleString() }] } })));
  const itemsNote = $derived(!page ? '' : !page.itemsBracket ? t('hex.noItems', page.source.patch) : page.itemsBracket !== bracket() ? t('hex.itemsAll') : '');
</script>

{#if error && !page}
  <section class="card"><p class="dim">{error}</p></section>
{:else if !page}
  <section class="card"><p class="dim">{t('common.loading')}</p></section>
{:else}
  <section class="card">
    <div class="label-row">
      <div class="label">{t('meta.augments')} <span class="sub">{t('hex.augsHint')}</span></div>
      <label class="check"><input type="checkbox" bind:checked={opts.allAugs} /> {t('hex.allAugs')}</label>
      <label class="check"><input type="checkbox" bind:checked={opts.small} /> {t('hex.small')}</label>
    </div>
    {#if !page.augments.length}
      <p class="dim">{t('meta.noAugments')}</p>
    {:else}
      <div class="grids">
        {#each ['prismatic', 'gold', 'silver'] as const as r (r)}
          <StatGrid label={t(`aug.${r}`)} tone={r} cards={augCards(r)} {base} bind:sort={sorts[r]} />
        {/each}
      </div>
    {/if}
  </section>

  {#if page.itemsBracket}
    <div class="two">
      <section class="card">
        <div class="label-row"><div class="label">{t('hex.routes')} <span class="sub">{t('hex.routesHint')}</span></div></div>
        {#if styles.length}
          <div class="styles" role="radiogroup" aria-label={t('hex.styles')}>
            <button class="st all" role="radio" aria-checked={style === null} class:on={style === null} onclick={() => (style = null)}>{t('hex.allStyles')}</button>
            {#each styles as s (s.item)}
              {@const it = itemOf(s.item)}
              {@const wr = s.wins / s.games}
              <button
                class="st"
                role="radio"
                aria-checked={style === s.item}
                class:on={style === s.item}
                onclick={() => (style = style === s.item ? null : s.item)}
                use:tip={() => ({
                  icon: iicon(it),
                  title: t('hex.style', iname(it)),
                  stats: [
                    { label: t('hex.win'), value: `${pct1(wr)}%`, tone: wr > base + 0.005 ? 'up' : wr < base - 0.005 ? 'down' : '' },
                    { label: t('hex.stylePick', iname(it)), value: `${share(byId.get(s.item)?.pickRate ?? 0)}%` },
                    { label: t('hex.games'), value: s.games.toLocaleString() },
                  ],
                  note: t('hex.styleNote'),
                })}
              >
                <Img src={iicon(it)} size={30} />
                <span>
                  <b class="num" class:up={wr >= base} class:down={wr < base}>{pct1(wr)}%</b>
                  <small>{t('hex.style', iname(it))}</small>
                </span>
              </button>
            {/each}
          </div>
          <div class="routes">
            {#each routes as c (c.items.join('-'))}
              <div class="route" class:best={c.winRate === bestRoute}>
                <span class="steps">
                  {#each c.items as id, i (i)}
                    {@const it = itemOf(id)}
                    {#if i}<span class="arrow">›</span>{/if}
                    <span class="step" use:tip={() => ({ icon: iicon(it), title: iname(it), sub: t('hex.nth', i + 1), body: itemText(id), stats: byId.get(id) ? stats(byId.get(id)!) : [] })}><Img src={iicon(it)} size={36} /></span>
                  {/each}
                </span>
                <span class="rname">{c.items.map((id) => iname(itemOf(id))).join(' › ')}</span>
                <span class="rnum">
                  <b class="num" class:up={c.winRate >= base}>{pct1(c.winRate)}%</b>
                  <small class="num">{t('hex.pick')} {share(c.pickRate)}% · {c.games.toLocaleString()}</small>
                </span>
                {#if c.winRate === bestRoute && (page.cores.length ?? 0) > 1}<span class="badge">{t('hex.bestRoute')}</span>{/if}
              </div>
            {/each}
          </div>
        {:else}
          <p class="dim">{t('hex.noRoutes')}</p>
        {/if}
      </section>
      <section class="card">
        <div class="label-row"><div class="label">{t('hex.bootsSpells')}</div></div>
        <div class="grids">
          <StatGrid label={t('builds.boots')} tone="boots" cards={page.boots.filter((b) => opts.small || !b.lowSample).map((b) => itemCard(b))} {base} bind:sort={sorts.boots} />
          <StatGrid label={t('hex.spells')} tone="spells" cards={page.spells.filter((s) => opts.small || !s.lowSample).map(spellCard)} {base} bind:sort={sorts.spells} />
        </div>
      </section>
    </div>

    <section class="card">
      <div class="label-row">
        <div class="label">{t('meta.items')} <span class="sub">{t('hex.itemsHint')}</span></div>
        {#if itemsNote}<span class="sub">{itemsNote}</span>{/if}
      </div>
      <div class="grids">
        <StatGrid label={t('hex.finished')} tone="items" cards={reliable.map((i) => itemCard(i, style === i.id))} {base} bind:sort={sorts.items} />
        {#if bad.length}
          <StatGrid label={t('hex.bad')} tone="bad" cards={bad.map((i) => itemCard(i))} {base} bind:sort={sorts.bad} />
        {/if}
      </div>
      {#if bad.length}<p class="note">{t('hex.badHint')}</p>{/if}
    </section>
  {:else}
    <section class="card">
      <div class="label-row"><div class="label">{t('hex.spells')}</div><span class="sub">{itemsNote}</span></div>
      <StatGrid label={t('hex.spells')} tone="spells" cards={page.spells.filter((s) => opts.small || !s.lowSample).map(spellCard)} {base} bind:sort={sorts.spells} />
    </section>
  {/if}

  <section class="card">
    <div class="label-row"><div class="label">{t('hex.changes')} <span class="sub">{t('hex.changesHint')}</span></div></div>
    <MayhemChanges {championId} {page} />
  </section>
  <MayhemMods {championId} />

  <div class="charts">
    {#if bandBars.length}
      <section class="card">
        <div class="label">{t('hex.byBracket')}</div>
        <BandChart bars={bandBars} reference={allPlayers} referenceLabel={t('hex.allPlayers')} />
      </section>
    {/if}
    {#if trend.length > 1}
      <section class="card">
        <div class="label">{t('hex.byPatch')}</div>
        <TrendChart points={trend} current={page.source.patch} />
        <p class="note">{t('hex.byPatchHint')}</p>
      </section>
    {/if}
    {#if page.bandPicks.length}
      <section class="card">
        <div class="label">{t('hex.augsByBracket')}</div>
        <BandPicks bands={page.bandPicks} />
      </section>
    {/if}
  </div>

  <details class="card about">
    <summary>{t('hex.about')}</summary>
    <ul>
      <li>{t('hex.about1', page.source.patch, page.source.matches.toLocaleString(), page.source.snapshotAt.slice(0, 10))}</li>
      <li>{t('hex.about2')}</li>
      <li>{t('hex.about3')}</li>
      <li>{t('hex.about4')}</li>
    </ul>
    <p class="note">{ARAM_CREDIT}</p>
  </details>
{/if}

<style>
  .card {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 14px 16px;
    min-width: 0;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
    margin-bottom: 10px;
  }
  .note {
    color: var(--text-3);
    font-size: 12px;
    margin: 8px 0 0;
  }
  .label-row {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 10px;
  }
  .label-row .label {
    flex: 1;
    margin: 0;
  }
  .sub {
    color: var(--text-3);
    font: 400 12px var(--font-ui);
    letter-spacing: 0;
    text-transform: none;
  }
  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
    font-size: 12.5px;
    white-space: nowrap;
  }
  .grids {
    display: grid;
    gap: 8px;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 14px;
  }
  @media (max-width: 1380px) {
    .two {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .styles {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 10px;
  }
  .st {
    flex: 1 1 150px;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px 5px 6px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    border: 1px solid var(--seam);
    text-align: left;
  }
  .st.all {
    flex: 0 0 auto;
    padding: 5px 12px;
    color: var(--text-2);
    font-weight: 600;
    font-size: 12.5px;
  }
  .st:hover {
    background: var(--panel-3);
  }
  .st.on {
    border-color: var(--you);
    box-shadow: inset 0 0 0 1px var(--you);
    color: var(--text);
  }
  .st span {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .st small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .st b {
    font-size: 13.5px;
  }
  .st small {
    color: var(--text-3);
    font-size: 11.5px;
  }
  .up {
    color: var(--win);
  }
  .down {
    color: var(--death);
  }
  .routes {
    display: grid;
    gap: 6px;
  }
  .route {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 7px 10px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    border: 1px solid transparent;
  }
  .route.best {
    border-color: color-mix(in oklab, var(--objective) 70%, transparent);
  }
  .steps {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .step {
    display: inline-flex;
    border-radius: var(--r-md);
  }
  .arrow {
    color: var(--text-3);
  }
  .rname {
    font-size: 12.5px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rnum {
    display: grid;
    justify-items: end;
    line-height: 1.25;
  }
  .rnum b {
    font-size: 14px;
  }
  .rnum small {
    color: var(--text-3);
    font-size: 11px;
  }
  .badge {
    position: absolute;
    top: -8px;
    right: 10px;
    padding: 0 6px;
    border-radius: var(--r-sm);
    background: var(--objective);
    color: var(--bg);
    font: 700 10.5px/16px var(--font-score);
    letter-spacing: 0.04em;
  }
  .charts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 14px;
  }
  .charts .label {
    margin-bottom: 8px;
  }
  .about summary {
    cursor: pointer;
    color: var(--text-2);
    font-weight: 600;
  }
  .about ul {
    margin: 10px 0 0;
    padding-left: 18px;
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.6;
  }
  .dim {
    color: var(--text-3);
    margin: 0;
  }
</style>
