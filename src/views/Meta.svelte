<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { api } from '../lib/api';
  import { buildIndex, searchChamps } from '../lib/champSearch';
  import { champIcon, champName, dd, loadOtherNames, otherNames } from '../lib/dd.svelte';
  import { roleName } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import { ARAM_CREDIT, BRACKETS, aramTier, augment, bracket, delta, loadAugments, pct1, riftTier, setBracket } from '../lib/meta.svelte';
  import { errorText, go } from '../lib/state.svelte';
  import type { AramOverview, BuildMode, RiftTiers } from '../lib/types';

  let { mode: startMode }: { mode?: BuildMode } = $props();

  let mode = $state<BuildMode>('rift');
  $effect(() => {
    if (startMode) mode = startMode;
  });
  let lane = $state('all');
  let aramView = $state<'champions' | 'augments'>('champions');
  let rarity = $state<'all' | 'silver' | 'gold' | 'prismatic'>('all');
  let query = $state('');
  let sortKey = $state('');
  let sortDesc = $state(true);

  let rift = $state<RiftTiers | null>(null);
  let aram = $state<AramOverview | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    if (mode !== 'rift' || rift) return;
    api.riftTiers().then((r) => (rift = r), (e) => (error = errorText(e)));
  });
  $effect(() => {
    if (mode !== 'aram') return;
    const b = bracket();
    loadAugments();
    error = null;
    api.aramOverview(b).then((r) => (aram = r), (e) => (error = errorText(e)));
  });
  $effect(() => {
    // A new table starts with its own default order.
    mode;
    aramView;
    sortKey = '';
    sortDesc = true;
  });

  const index = $derived(buildIndex([dd.data?.champions, otherNames.champions]));
  const matches = $derived(query.trim() ? new Set(searchChamps(index, query, Infinity).map((h) => h.id)) : null);

  function sorted<T>(rows: T[], key: (r: T) => number, fallback: (a: T, b: T) => number): T[] {
    if (!sortKey) return [...rows].sort(fallback);
    return [...rows].sort((a, b) => (sortDesc ? key(b) - key(a) : key(a) - key(b)));
  }

  const riftRows = $derived.by(() => {
    const rows = (rift?.rows ?? []).filter((r) => (lane === 'all' || r.position === lane) && (!matches || matches.has(r.championId)));
    const k: Record<string, (r: (typeof rows)[number]) => number> = {
      tier: (r) => -r.tier * 1000 - r.rank,
      win: (r) => r.winRate,
      pick: (r) => r.pickRate,
      ban: (r) => r.banRate,
      games: (r) => r.games,
    };
    return sorted(rows, k[sortKey] ?? k.tier, (a, b) => a.tier - b.tier || b.winRate - a.winRate);
  });

  const aramChamps = $derived.by(() => {
    const rows = (aram?.champions ?? []).filter((r) => !matches || matches.has(r.championId));
    const k: Record<string, (r: (typeof rows)[number]) => number> = { win: (r) => r.winRate, pick: (r) => r.pickRate, games: (r) => r.games };
    return sorted(rows, k[sortKey] ?? k.win, (a, b) => b.winRate - a.winRate);
  });

  const aramAugs = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const rows = (aram?.augments ?? []).filter((a) => {
      const info = augment(a.id);
      if (!info) return false;
      return (rarity === 'all' || info.rarity === rarity) && (!q || info.name.toLowerCase().includes(q));
    });
    const k: Record<string, (r: (typeof rows)[number]) => number> = { win: (r) => r.winRate, games: (r) => r.games };
    return sorted(rows, k[sortKey] ?? k.win, (a, b) => b.smoothed - a.smoothed);
  });

  function sortBy(key: string) {
    if (sortKey === key) sortDesc = !sortDesc;
    else {
      sortKey = key;
      sortDesc = true;
    }
  }

  const LANES = ['all', 'top', 'jungle', 'mid', 'adc', 'support'];
</script>

{#snippet th(key: string, label: string)}
  <th><button class="sort" class:on={sortKey === key} onclick={() => sortBy(key)}>{label}{#if sortKey === key}<span>{sortDesc ? '↓' : '↑'}</span>{/if}</button></th>
{/snippet}

<div class="page">
  <header class="head">
    <h2>{t('meta.title')}</h2>
    <Segmented
      label={t('builds.mode')}
      value={mode}
      options={[
        { value: 'rift', label: t('mode.rift') },
        { value: 'aram', label: t('mode.aram') },
      ]}
      onchange={(v) => (mode = v as BuildMode)}
    />
    <div class="search">
      <Icon name="search" size={15} />
      <input class="field" bind:value={query} placeholder={mode === 'aram' && aramView === 'augments' ? t('meta.findAugment') : t('builds.search')} spellcheck="false" onfocus={loadOtherNames} />
    </div>
  </header>

  <div class="filters">
    {#if mode === 'rift'}
      <Segmented label={t('meta.lane')} value={lane} options={LANES.map((l) => ({ value: l, label: l === 'all' ? t('meta.allLanes') : roleName(l) }))} onchange={(v) => (lane = v)} />
    {:else}
      <Segmented
        label={t('meta.view')}
        value={aramView}
        options={[
          { value: 'champions', label: t('meta.champions') },
          { value: 'augments', label: t('meta.augments') },
        ]}
        onchange={(v) => (aramView = v as typeof aramView)}
      />
      <Segmented label={t('meta.bracket')} value={bracket()} options={BRACKETS.map((b) => ({ value: b, label: t(`bracket.${b}`) }))} onchange={(v) => setBracket(v as typeof BRACKETS[number])} />
      {#if aramView === 'augments'}
        <Segmented
          label={t('meta.rarity')}
          value={rarity}
          options={['all', 'silver', 'gold', 'prismatic'].map((r) => ({ value: r, label: r === 'all' ? t('meta.allRarities') : t(`aug.${r}`) }))}
          onchange={(v) => (rarity = v as typeof rarity)}
        />
      {/if}
    {/if}
    <span class="source">
      {#if mode === 'rift'}{t('meta.srcRift')}{:else if aram}{t('meta.srcAram', aram.source.patch, (aram.source.matches / 1e6).toFixed(1))}{/if}
    </span>
  </div>

  <div class="table-wrap">
    {#if error && !(mode === 'rift' ? rift : aram)}
      <p class="empty-row">{error}</p>
    {:else if mode === 'rift'}
      {#if !rift}
        <p class="empty-row">{t('common.loading')}</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th class="n">#</th>
              {@render th('tier', t('meta.tier'))}
              <th>{t('meta.champion')}</th>
              {#if lane === 'all'}<th>{t('meta.lane')}</th>{/if}
              {@render th('win', t('meta.winRate'))}
              {@render th('pick', t('meta.pickRate'))}
              {@render th('ban', t('meta.banRate'))}
              {@render th('games', t('meta.games'))}
            </tr>
          </thead>
          <tbody>
            {#each riftRows as r, i (r.championId + r.position)}
              <tr onclick={() => go({ name: 'builds', championId: r.championId, role: r.position, mode: 'rift' })}>
                <td class="n num">{i + 1}</td>
                <td><span class="tier t{riftTier(r.tier)}">{riftTier(r.tier)}</span></td>
                <td><span class="champ"><Img src={champIcon(r.championId)} size={28} />{champName(r.championId)}</span></td>
                {#if lane === 'all'}<td class="dim">{roleName(r.position)}</td>{/if}
                <td class="num" class:hi={r.winRate >= 0.52} class:lo={r.winRate < 0.48}>{pct1(r.winRate)}%</td>
                <td class="num">{Math.round(r.pickRate * 100)}%</td>
                <td class="num">{Math.round(r.banRate * 100)}%</td>
                <td class="num dim">{r.games.toLocaleString()}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if !aram}
      <p class="empty-row">{t('common.loading')}</p>
    {:else if aramView === 'champions'}
      <table>
        <thead>
          <tr>
            <th class="n">#</th>
            <th>{t('meta.tier')}</th>
            <th>{t('meta.champion')}</th>
            {@render th('win', t('meta.winRate'))}
            {@render th('pick', t('meta.pickRate'))}
            {@render th('games', t('meta.games'))}
          </tr>
        </thead>
        <tbody>
          {#each aramChamps as r, i (r.championId)}
            <tr onclick={() => go({ name: 'builds', championId: r.championId, mode: 'aram' })}>
              <td class="n num">{i + 1}</td>
              <td><span class="tier t{aramTier(r.winRate)}">{aramTier(r.winRate)}</span></td>
              <td><span class="champ"><Img src={champIcon(r.championId)} size={28} />{champName(r.championId)}</span></td>
              <td class="num" class:hi={r.winRate >= 0.52} class:lo={r.winRate < 0.48}>{pct1(r.winRate)}%</td>
              <td class="num">{pct1(r.pickRate)}%</td>
              <td class="num dim">{r.games.toLocaleString()}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <table>
        <thead>
          <tr>
            <th class="n">#</th>
            <th>{t('meta.augment')}</th>
            <th>{t('meta.rarity')}</th>
            {@render th('win', t('meta.winRate'))}
            <th>{t('meta.vsAverage')}</th>
            {@render th('games', t('meta.games'))}
          </tr>
        </thead>
        <tbody>
          {#each aramAugs as a, i (a.id)}
            {@const info = augment(a.id)}
            <tr class="static">
              <td class="n num">{i + 1}</td>
              <td><span class="champ"><Img src={info?.icon ?? null} size={28} />{info?.name}</span></td>
              <td><span class="rar {info?.rarity}">{info?.rarity ? t(`aug.${info.rarity}`) : ''}</span></td>
              <td class="num" class:hi={a.delta > 0.01} class:lo={a.delta < -0.01}>{pct1(a.winRate)}%</td>
              <td class="num dim">{delta(a.delta)}</td>
              <td class="num dim">{a.games.toLocaleString()}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
  <p class="credit">{mode === 'rift' ? t('meta.creditRift') : `${aramView === 'champions' ? t('meta.creditAram') : t('meta.data')} ${ARAM_CREDIT}`}</p>
</div>

<style>
  .page {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px 14px;
    min-height: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .head h2 {
    font-size: 22px;
  }
  .head :global(.seg) {
    width: 280px;
  }
  .search {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-3);
  }
  .search .field {
    width: 220px;
  }
  .filters {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .source {
    margin-left: auto;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .table-wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
  }
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
  .champ {
    display: flex;
    align-items: center;
    gap: 9px;
    font-weight: 500;
  }
  .tier {
    display: inline-grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: var(--r-md);
    font: 700 13px var(--font-score);
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
  .rar {
    font: 600 11.5px var(--font-score);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .rar.silver {
    color: var(--text-2);
  }
  .rar.gold {
    color: var(--objective);
  }
  .rar.prismatic {
    color: var(--assist);
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
  .empty-row {
    padding: 24px;
    color: var(--text-3);
  }
  .credit {
    color: var(--text-3);
    font-size: 11.5px;
  }
</style>
