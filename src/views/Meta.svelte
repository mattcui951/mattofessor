<script lang="ts" module>
  import type { BuildMode, ChampClass, ItemClass } from '../lib/types';

  export type AramView = 'champions' | 'augments' | 'combos' | 'items';

  // What the page shows, kept while the app runs: back to this page lands on the same tabs.
  let mode = $state<BuildMode>('rift');
  let lane = $state('all');
  let aramView = $state<AramView>('champions');
  let champMode = $state<'champ' | 'build' | 'list'>('champ');
  let augMode = $state<'board' | 'list'>('board');
  let cls = $state<'all' | ChampClass>('all');
  let itemCls = $state<'all' | ItemClass>('all');
  let rarity = $state<'all' | 'silver' | 'gold' | 'prismatic'>('all');
  let minGames = $state(20000);
  let comboSort = $state<'win' | 'delta' | 'games'>('win');
  /** The pairs of one augment only (from an augment's tile or the search box). */
  let comboAug = $state<number | null>(null);
</script>

<script lang="ts">
  import AugmentBoard from '../components/AugmentBoard.svelte';
  import ChampBoard from '../components/ChampBoard.svelte';
  import ComboTable from '../components/ComboTable.svelte';
  import HexFilter from '../components/HexFilter.svelte';
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { api } from '../lib/api';
  import { tip } from '../lib/hover.svelte';
  import { loadGameData } from '../lib/gamedata.svelte';
  import { augmentTip, itemTip } from '../lib/tips.svelte';
  import { findChamps, loadChampSearch } from '../lib/champFind.svelte';
  import { champIcon, champName, dd, ITEM_CLASSES, itemClasses, itemIcon, itemName, loadOtherNames } from '../lib/dd.svelte';
  import { roleName } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import { ARAM_CREDIT, aramTier, augment, augs, bracket, delta, gamesShort, hex, loadAugments, notePatches, pct1, riftTier, share } from '../lib/meta.svelte';
  import { errorText, go } from '../lib/state.svelte';
  import type { AugBoard, Board, Combos, ItemOverview, ItemRow, RiftTiers } from '../lib/types';

  let { mode: startMode, view: startView, augment: startAug }: { mode?: BuildMode; view?: AramView; augment?: number } = $props();

  $effect(() => {
    if (startMode) mode = startMode;
    if (startView) aramView = startView;
    if (startAug) comboAug = startAug;
  });
  let query = $state('');
  let sortKey = $state('');
  let sortDesc = $state(true);

  let rift = $state<RiftTiers | null>(null);
  let board = $state<Board | null>(null);
  let augBoard = $state<AugBoard | null>(null);
  let combos = $state<Combos | null>(null);
  let items = $state<ItemOverview | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    if (mode !== 'rift' || rift) return;
    api.riftTiers().then((r) => (rift = r), (e) => (error = errorText(e)));
  });
  /** One answer per request: a slower earlier one never replaces a newer one. */
  let run = 0;
  function fetchInto<T extends object>(p: Promise<T>, set: (v: T) => void) {
    const mine = ++run;
    error = null;
    p.then(
      (v) => {
        if (mine !== run) return;
        set(v);
        if ('patches' in v) notePatches((v as { patches: Board['patches'] }).patches);
      },
      (e) => {
        if (mine === run) error = errorText(e);
      },
    );
  }
  $effect(() => {
    if (mode !== 'aram') return;
    const p = hex.patch;
    const b = bracket();
    loadAugments();
    if (aramView === 'champions') fetchInto(api.aramBoard(p, b), (v) => (board = v));
    else if (aramView === 'augments') fetchInto(api.aramAugmentBoard(p, b), (v) => (augBoard = v));
    else if (aramView === 'items') fetchInto(api.aramItems(b, p), (v) => (items = v));
  });

  loadChampSearch();
  loadGameData();
  /** Champions the search finds (any language, nicknames), null without a search. */
  const champHits = $derived(findChamps(query));
  const champSet = $derived(champHits ? new Set(champHits) : null);
  /** Augments whose name has the search, null without a search. */
  const augHits = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    augs.byId;
    return new Set(Object.entries(augs.byId).filter(([, a]) => a.name.toLowerCase().includes(q)).map(([id]) => Number(id)));
  });

  $effect(() => {
    if (mode !== 'aram' || aramView !== 'combos') return;
    const p = hex.patch;
    const b = bracket();
    // A search narrows the pairs to the champions it finds, else to the augments it finds; one
    // that finds neither shows none.
    const champions = champHits ?? [];
    const augments = comboAug ? [comboAug] : champions.length ? [] : [...(augHits ?? [])];
    if (query.trim() && !champions.length && !augments.length) {
      combos = combos ? { ...combos, total: 0, rows: [] } : null;
      return;
    }
    loadAugments();
    fetchInto(api.aramCombos(p, b, minGames, comboSort, champions, augments), (v) => (combos = v));
  });
  $effect(() => {
    // A new table starts with its own default order.
    mode;
    aramView;
    champMode;
    augMode;
    sortKey = '';
    sortDesc = true;
  });

  function sorted<T>(rows: T[], key: (r: T) => number, fallback: (a: T, b: T) => number): T[] {
    if (!sortKey) return [...rows].sort(fallback);
    return [...rows].sort((a, b) => (sortDesc ? key(b) - key(a) : key(a) - key(b)));
  }

  const riftRows = $derived.by(() => {
    const rows = (rift?.rows ?? []).filter((r) => (lane === 'all' || r.position === lane) && (!champSet || champSet.has(r.championId)));
    const k: Record<string, (r: (typeof rows)[number]) => number> = {
      tier: (r) => -r.tier * 1000 - r.rank,
      win: (r) => r.winRate,
      pick: (r) => r.pickRate,
      ban: (r) => r.banRate,
      games: (r) => r.games,
    };
    return sorted(rows, k[sortKey] ?? k.tier, (a, b) => a.tier - b.tier || b.winRate - a.winRate);
  });

  const champRows = $derived.by(() => {
    const rows = (board?.champions ?? []).filter((r) => (!champSet || champSet.has(r.championId)) && (cls === 'all' || dd.data?.champions[String(r.championId)]?.tags.includes(cls)));
    const k: Record<string, (r: (typeof rows)[number]) => number> = { win: (r) => r.winRate, pick: (r) => r.pickRate, games: (r) => r.games, move: (r) => (r.previous === null ? -1 : r.winRate - r.previous) };
    return sorted(rows, k[sortKey] ?? k.win, (a, b) => b.winRate - a.winRate);
  });

  const augRows = $derived.by(() => {
    const rows = (augBoard?.augments ?? []).filter((a) => {
      const info = augment(a.id);
      return info && (rarity === 'all' || info.rarity === rarity) && (!augHits || augHits.has(a.id));
    });
    const k: Record<string, (r: (typeof rows)[number]) => number> = { win: (r) => r.winRate, delta: (r) => r.delta, pick: (r) => r.pickRate, games: (r) => r.games };
    return sorted(rows, k[sortKey] ?? k.delta, (a, b) => b.delta - a.delta);
  });

  /** Data Dragon's name and icon, or AramHex's for items it does not have. */
  const known = (r: ItemRow) => Boolean(dd.data?.items[String(r.id)]);
  const itemLabel = (r: ItemRow) => (known(r) ? itemName(r.id) : r.name || String(r.id));
  const itemPic = (r: ItemRow) => (known(r) ? itemIcon(r.id) : r.icon || null);

  const itemRows = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const rows = (items?.items ?? []).filter((r) => (!q || itemLabel(r).toLowerCase().includes(q)) && (itemCls === 'all' || itemClasses(r.id).includes(itemCls)));
    const k: Record<string, (r: ItemRow) => number> = { win: (r) => r.winRate, delta: (r) => r.delta, pick: (r) => r.pickRate, games: (r) => r.games };
    return sorted(rows, k[sortKey] ?? k.win, (a, b) => b.winRate - a.winRate);
  });

  const CLASSES: ('all' | ChampClass)[] = ['all', 'Fighter', 'Assassin', 'Mage', 'Tank', 'Support', 'Marksman'];

  function sortBy(key: string) {
    if (sortKey === key) sortDesc = !sortDesc;
    else {
      sortKey = key;
      sortDesc = true;
    }
  }

  const LANES = ['all', 'top', 'jungle', 'mid', 'adc', 'support'];
  const MIN_GAMES = [3000, 20000, 100000];

  /** The source line: the patch, its games, and the bracket shown when it differs. */
  const src = $derived.by(() => {
    const s = aramView === 'champions' ? board?.source : aramView === 'augments' ? augBoard?.source : aramView === 'combos' ? combos?.source : items?.source;
    if (!s) return '';
    const all = s.bracket !== bracket() ? `${t('meta.itemsAllPlayers')} · ` : '';
    return all + t('meta.srcAram', s.patch, gamesShort(s.matches));
  });
  const searchHint = $derived(
    mode === 'rift' ? t('builds.search') : aramView === 'augments' ? t('meta.findAugment') : aramView === 'combos' ? t('hex.findCombo') : aramView === 'items' ? t('mayhem.search') : t('builds.search'),
  );
  const current = $derived(aramView === 'champions' ? board : aramView === 'augments' ? augBoard : aramView === 'combos' ? combos : items);
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
    {#if mode === 'aram'}
      <div class="tabs" role="tablist" aria-label={t('meta.view')}>
        {#each [['champions', t('hex.board')], ['augments', t('hex.augBoard')], ['combos', t('hex.combos')], ['items', t('hex.itemBoard')]] as [v, label] (v)}
          <button role="tab" aria-selected={aramView === v} class:on={aramView === v} onclick={() => (aramView = v as AramView)}>{label}</button>
        {/each}
      </div>
    {/if}
    <div class="search">
      <Icon name="search" size={15} />
      <input class="field" bind:value={query} placeholder={searchHint} spellcheck="false" onfocus={loadOtherNames} />
    </div>
  </header>

  <div class="filters">
    {#if mode === 'rift'}
      <Segmented label={t('meta.lane')} value={lane} options={LANES.map((l) => ({ value: l, label: l === 'all' ? t('meta.allLanes') : roleName(l) }))} onchange={(v) => (lane = v)} />
    {:else}
      <HexFilter />
      {#if aramView === 'champions'}
        <Segmented
          label={t('meta.view')}
          value={champMode}
          options={[
            { value: 'champ', label: t('hex.byChamp') },
            { value: 'build', label: t('hex.byBuild'), hint: t('hex.byBuildHint') },
            { value: 'list', label: t('hex.list') },
          ]}
          onchange={(v) => (champMode = v as typeof champMode)}
        />
        {#if champMode === 'list'}
          <Segmented label={t('meta.class')} value={cls} options={CLASSES.map((c) => ({ value: c, label: c === 'all' ? t('meta.allClasses') : t(`class.${c}`) }))} onchange={(v) => (cls = v as typeof cls)} />
        {/if}
      {:else if aramView === 'augments'}
        <Segmented
          label={t('meta.view')}
          value={augMode}
          options={[
            { value: 'board', label: t('hex.boardView') },
            { value: 'list', label: t('hex.list') },
          ]}
          onchange={(v) => (augMode = v as typeof augMode)}
        />
        {#if augMode === 'list'}
          <Segmented
            label={t('meta.rarity')}
            value={rarity}
            options={['all', 'silver', 'gold', 'prismatic'].map((r) => ({ value: r, label: r === 'all' ? t('meta.allRarities') : t(`aug.${r}`) }))}
            onchange={(v) => (rarity = v as typeof rarity)}
          />
        {/if}
      {:else if aramView === 'combos'}
        <Segmented label={t('hex.minGames')} value={minGames} options={MIN_GAMES.map((n) => ({ value: n, label: t(`hex.min${n}`) }))} onchange={(v) => (minGames = v)} />
        {#if comboAug}
          {@const a = augment(comboAug)}
          <button class="chip" onclick={() => (comboAug = null)} title={t('hex.clearFilter')}>
            <Img src={a?.icon ?? null} size={18} round />{a?.name ?? `#${comboAug}`}<Icon name="close" size={12} />
          </button>
        {/if}
      {:else}
        <Segmented
          label={t('meta.class')}
          value={itemCls}
          options={(['all', ...ITEM_CLASSES.filter((c) => c !== 'Other')] as const).map((c) => ({ value: c, label: c === 'all' ? t('meta.allClasses') : t(`itemClass.${c}`) }))}
          onchange={(v) => (itemCls = v as typeof itemCls)}
        />
      {/if}
    {/if}
    <span class="source">{mode === 'rift' ? t('meta.srcRift') : src}</span>
  </div>

  <div class="table-wrap" class:bare={mode === 'aram' && ((aramView === 'champions' && champMode !== 'list') || (aramView === 'augments' && augMode === 'board'))}>
    {#if mode === 'rift'}
      {#if error && !rift}
        <p class="empty-row">{error}</p>
      {:else if !rift}
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
    {:else if error && !current}
      <p class="empty-row">{error}</p>
    {:else if !current}
      <p class="empty-row">{t('common.loading')}</p>
    {:else if aramView === 'champions' && board}
      {#if champMode !== 'list'}
        <ChampBoard {board} builds={champMode === 'build'} matches={champSet} />
        {#if champMode === 'build' && !board.buildsBracket}<p class="credit">{t('hex.noItems', board.source.patch)}</p>{/if}
      {:else}
        <table>
          <thead>
            <tr>
              <th class="n">#</th>
              <th>{t('meta.tier')}</th>
              <th>{t('meta.champion')}</th>
              {@render th('win', t('meta.winRate'))}
              {#if board.previousPatch}{@render th('move', t('hex.since', board.previousPatch))}{/if}
              {@render th('pick', t('meta.pickRate'))}
              {@render th('games', t('meta.games'))}
            </tr>
          </thead>
          <tbody>
            {#each champRows as r, i (r.championId)}
              {@const tier = aramTier(r.winRate)}
              <tr onclick={() => go({ name: 'builds', championId: r.championId, mode: 'aram' })}>
                <td class="n num">{i + 1}</td>
                <td><span class="tier t{tier}">{t(`tiers.${tier}`)}</span></td>
                <td><span class="champ"><Img src={champIcon(r.championId)} size={28} />{champName(r.championId)}</span></td>
                <td class="num" class:hi={r.winRate >= 0.52} class:lo={r.winRate < 0.48}>{pct1(r.winRate)}%</td>
                {#if board.previousPatch}
                  <td class="num" class:hi={r.previous !== null && r.winRate - r.previous >= 0.01} class:lo={r.previous !== null && r.previous - r.winRate >= 0.01}>{r.previous === null ? '' : delta(r.winRate - r.previous)}</td>
                {/if}
                <td class="num">{pct1(r.pickRate)}%</td>
                <td class="num dim">{r.games.toLocaleString()}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if aramView === 'augments' && augBoard}
      {#if augMode === 'board'}
        <AugmentBoard board={augBoard} matches={augHits} onpick={(id) => { comboAug = id; aramView = 'combos'; query = ''; }} />
      {:else}
        <table>
          <thead>
            <tr>
              <th class="n">#</th>
              <th>{t('meta.augment')}</th>
              <th>{t('meta.rarity')}</th>
              {@render th('win', t('meta.winRate'))}
              {@render th('delta', t('hex.lift'))}
              {@render th('pick', t('hex.held'))}
              {@render th('games', t('meta.games'))}
            </tr>
          </thead>
          <tbody>
            {#each augRows as a, i (a.id)}
              {@const info = augment(a.id)}
              <tr onclick={() => { comboAug = a.id; aramView = 'combos'; query = ''; }}>
                <td class="n num">{i + 1}</td>
                <td><span class="champ" use:tip={() => augmentTip(a.id)}><Img src={info?.icon ?? null} size={28} round />{info?.name}</span></td>
                <td><span class="rar {info?.rarity}">{info?.rarity ? t(`aug.${info.rarity}`) : ''}</span></td>
                <td class="num">{pct1(a.winRate)}%</td>
                <td class="num" class:hi={a.delta > 0.01} class:lo={a.delta < -0.01}>{delta(a.delta)}</td>
                <td class="num">{share(a.pickRate)}%</td>
                <td class="num dim">{a.games.toLocaleString()}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if aramView === 'combos' && combos}
      <ComboTable {combos} sort={comboSort} onsort={(s) => (comboSort = s)} />
    {:else if items}
      {#if !items.items.length}
        <p class="empty-row">{t('hex.noItems', items.source.patch)}</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th class="n">#</th>
              <th>{t('meta.item')}</th>
              {@render th('win', t('meta.winRate'))}
              {@render th('delta', t('meta.vsChampion'))}
              {@render th('pick', t('meta.builtBy'))}
              {@render th('games', t('meta.games'))}
            </tr>
          </thead>
          <tbody>
            {#each itemRows as r, i (r.id)}
              <tr class="static">
                <td class="n num">{i + 1}</td>
                <td><span class="champ" use:tip={() => (known(r) ? itemTip(r.id) : null)}><Img src={itemPic(r)} size={28} />{itemLabel(r)}</span></td>
                <td class="num" class:hi={r.winRate >= 0.52} class:lo={r.winRate < 0.48}>{pct1(r.winRate)}%</td>
                <td class="num" class:hi={r.delta > 0.01} class:lo={r.delta < -0.01}>{delta(r.delta)}</td>
                <td class="num">{share(r.pickRate)}%</td>
                <td class="num dim">{r.games.toLocaleString()}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {/if}
  </div>
  <p class="credit">
    {#if mode === 'rift'}{t('meta.creditRift')}{:else}{aramView === 'champions' ? t('meta.creditAram') : aramView === 'items' ? t('meta.itemsHint') : aramView === 'combos' ? t('hex.combosHint', combos?.total.toLocaleString() ?? '0', combos?.rows.length ?? 0) : t('meta.data')} {ARAM_CREDIT}{/if}
  </p>
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
  .head > :global(.seg) {
    width: 280px;
  }
  .tabs {
    display: flex;
    gap: 2px;
  }
  .tabs button {
    height: 32px;
    padding: 0 12px;
    border-radius: var(--r-md);
    color: var(--text-2);
    font-weight: 600;
  }
  .tabs button:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .tabs button.on {
    background: var(--panel-3);
    color: var(--you);
    box-shadow: inset 0 -2px 0 var(--you);
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
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 10px 0 6px;
    border-radius: var(--r-md);
    background: var(--panel-3);
    color: var(--text);
    font-size: 12.5px;
    box-shadow: inset 0 0 0 1px var(--you);
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
  .table-wrap.bare {
    background: none;
    border: 0;
    border-radius: 0;
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
    min-width: 24px;
    height: 24px;
    padding: 0 6px;
    border-radius: var(--r-md);
    font: 700 12.5px var(--font-score);
    color: var(--bg);
    background: var(--text-3);
    white-space: nowrap;
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
    margin: 0;
  }
</style>
