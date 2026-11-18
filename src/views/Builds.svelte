<script lang="ts" module>
  import type { ChampClass } from '../lib/types';

  // The picker's filters, kept while the app runs: each pick opens the page anew.
  let lane = $state<'all' | 'top' | 'jungle' | 'mid' | 'adc' | 'support' | 'off'>('all');
  let cls = $state<'all' | ChampClass>('all');
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import HexFilter from '../components/HexFilter.svelte';
  import MayhemBuild from '../components/MayhemBuild.svelte';
  import MayhemChampion from '../components/MayhemChampion.svelte';
  import MayhemEditor from '../components/MayhemEditor.svelte';
  import { adminMode, content, loadContent } from '../lib/content.svelte';
  import Icon from '../components/Icon.svelte';
  import PublishState from '../components/PublishState.svelte';
  import ItemIcon from '../components/ItemIcon.svelte';
  import Img from '../components/Img.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { api } from '../lib/api';
  import { searchChamps } from '../lib/champSearch';
  import { champIndex, loadChampSearch } from '../lib/champFind.svelte';
  import { SHARD_NAMES, champIcon, dd, itemIcon, itemName, loadOtherNames, otherNames, rune, runeIcon, spell, spellIcon } from '../lib/dd.svelte';
  import { pct, roleName } from '../lib/format';
  import { i18n, t } from '../lib/i18n.svelte';
  import { aramTier, bracket, pct1, setBracket, share } from '../lib/meta.svelte';
  import { app, errorText, go, toast } from '../lib/state.svelte';
  import type { Build, BuildMode, ChampionPage, TierRow } from '../lib/types';

  const LANES = ['top', 'jungle', 'mid', 'adc', 'support'] as const;
  type Lane = (typeof LANES)[number];
  /** A champion plays a lane regularly when at least this share of its games is there (OP.GG). */
  const REGULAR = 0.2;

  let { championId, role, mode: startMode }: { championId?: number; role?: string; mode?: BuildMode } = $props();

  let selected = $state<number | null>(null);
  let selRole = $state('');
  let mode = $state<BuildMode>('rift');
  let build = $state<Build | null>(null);
  let loading = $state(false);
  /** Admin: writing the selected champion's Hextech ARAM build. */
  let editing = $state(false);
  const admin = $derived(adminMode() && mode === 'aram');

  /** Hextech ARAM: the champion's page from AramHex (`MayhemChampion` loads it). */
  let hexPage = $state<ChampionPage | null>(null);
  const hexChamp = $derived(hexPage && hexPage.champion?.championId === selected ? hexPage : null);
  loadContent();
  const written = $derived(Object.keys(content.data.builds).length);
  let error = $state<string | null>(null);
  let query = $state('');
  let importing = $state(false);
  let doRunes = $state(true);
  let doItems = $state(true);

  let grid = $state<HTMLElement>();

  // Lane tabs (Summoner's Rift): from OP.GG's lane tier lists, which hold every champion played
  // in a lane in at least ~10,000 games. "off" groups the ones rarely played there. Hextech ARAM
  // has no lanes: class tabs instead (Data Dragon's classes; a champion can have two).
  const CLASSES: ChampClass[] = ['Fighter', 'Mage', 'Assassin', 'Marksman', 'Tank', 'Support'];
  let tierRows = $state<TierRow[]>([]);
  $effect(() => {
    api.riftTiers().then(
      (t) => (tierRows = t.rows),
      () => (tierRows = []),
    );
  });
  const laneSets = $derived.by(() => {
    const regular = new Map<Lane, Set<number>>(LANES.map((l) => [l, new Set()]));
    const off = new Map<Lane, Set<number>>(LANES.map((l) => [l, new Set()]));
    for (const r of tierRows) {
      const l = r.position as Lane;
      (r.roleRate >= REGULAR ? regular : off).get(l)?.add(r.championId);
    }
    return { regular, off };
  });
  const lanesOn = $derived(mode === 'rift' && tierRows.length > 0);

  $effect(() => {
    if (startMode) mode = startMode;
    if (championId) {
      selected = championId;
      selRole = role ?? '';
      // Opened from search or a link: bring the champion into view in the picker.
      tick().then(() => grid?.querySelector('.c.on')?.scrollIntoView({ block: 'nearest' }));
    }
  });

  // Any language or pinyin; best matches first while searching, alphabetical otherwise.
  loadChampSearch();
  const index = $derived(champIndex());
  const champs = $derived.by(() => {
    const all = Object.entries(dd.data?.champions ?? {})
      .map(([id, c]) => ({ id: Number(id), ...c }))
      .sort((a, b) => a.name.localeCompare(b.name, i18n.lang === 'zh' ? 'zh-CN' : 'en'));
    const inClass = mode === 'aram' && cls !== 'all' ? all.filter((c) => c.tags.includes(cls)) : all;
    if (!query.trim()) return inClass;
    const rank = new Map(searchChamps(index, query, Infinity).map((h, i) => [h.id, i]));
    return inClass.filter((c) => rank.has(c.id)).sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
  });

  /** What the grid shows: one list, or the off-meta picks grouped by lane. */
  const groups = $derived.by((): { lane: Lane | null; list: typeof champs }[] => {
    if (!lanesOn || lane === 'all') return [{ lane: null, list: champs }];
    if (lane === 'off') {
      return LANES.map((l) => ({ lane: l, list: champs.filter((c) => laneSets.off.get(l)!.has(c.id)) })).filter((g) => g.list.length > 0);
    }
    const l = lane;
    return [{ lane: l, list: champs.filter((c) => laneSets.regular.get(l)!.has(c.id)) }];
  });
  /** A pick is a page of its own: back returns to the champion before. */
  function pickChamp(id: number, l: Lane | null) {
    go({ name: 'builds', championId: id, role: l ?? undefined, mode });
  }

  let run = 0;
  async function load() {
    if (!selected) return;
    const mine = ++run;
    loading = true;
    error = null;
    try {
      const b = await api.build(selected, mode === 'rift' ? selRole : '', mode);
      if (mine === run) build = b;
    } catch (e) {
      if (mine === run) {
        build = null;
        error = errorText(e);
      }
    } finally {
      if (mine === run) loading = false;
    }
  }

  $effect(() => {
    selected;
    selRole;
    mode;
    editing = false;
    load();
  });

  function doneEditing() {
    editing = false;
    load();
  }

  async function doImport() {
    if (!build) return;
    importing = true;
    try {
      const r = await api.importBuild(build.championId, build.role, build.mode, doRunes && build.mode === 'rift', doItems);
      toast(t('live.imported', [r.runePage, r.itemSet].filter(Boolean).join(' + ')), 'good');
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      importing = false;
    }
  }

  const page = $derived(build?.runes[0]?.value);
  const primaryTree = $derived(dd.data?.trees.find((tr) => tr.id === page?.primary));
  const subTree = $derived(dd.data?.trees.find((tr) => tr.id === page?.sub));
  const p = (n: number) => (build ? pct(n, build.games) : 0);
  const showBuild = $derived(build && build.championId === selected && build.mode === mode);
</script>

<div class="page">
  <aside class="picker">
    <div class="search">
      <Icon name="search" size={15} />
      <input class="field" bind:value={query} placeholder={t('builds.search')} spellcheck="false" onfocus={loadOtherNames} />
    </div>
    {#if lanesOn}
      <div class="lanes" role="tablist" aria-label={t('builds.lanes')}>
        {#each ['all', ...LANES, 'off'] as const as l (l)}
          <button role="tab" aria-selected={lane === l} class:on={lane === l} onclick={() => (lane = l)}>
            {l === 'all' ? t('builds.allLanes') : l === 'off' ? t('builds.offMeta') : roleName(l)}
          </button>
        {/each}
      </div>
    {:else if mode === 'aram'}
      <div class="lanes" role="tablist" aria-label={t('meta.class')}>
        {#each ['all', ...CLASSES] as const as c (c)}
          <button role="tab" aria-selected={cls === c} class:on={cls === c} onclick={() => (cls = c)}>
            {c === 'all' ? t('meta.allClasses') : t(`class.${c}`)}
          </button>
        {/each}
      </div>
    {/if}
    {#if admin}<p class="written"><span>{t('mayhem.done', written, champs.length)}</span><PublishState /></p>{/if}
    <div class="grid" bind:this={grid}>
      {#each groups as g (g.lane ?? 'all')}
        {#if lane === 'off' && g.lane}<h4 class="glabel">{t('builds.offMetaLane', roleName(g.lane))}</h4>{/if}
        {#each g.list as c (c.id)}
          <button class="c" class:on={selected === c.id && (!g.lane || selRole === g.lane)} class:has={admin && content.data.builds[c.id]} title={c.name} onclick={() => pickChamp(c.id, g.lane)}>
            <Img src={champIcon(c.id)} size={42} alt={c.name} />
            <span>{c.name}</span>
          </button>
        {/each}
      {:else}
        <p class="gempty">{t('search.noMatch')}</p>
      {/each}
    </div>
  </aside>

  <main class="main">
    <div class="bar">
      <Segmented
        label={t('builds.mode')}
        value={mode}
        options={[
          { value: 'rift', label: t('mode.rift') },
          { value: 'aram', label: t('mode.aram') },
        ]}
        onchange={(v) => go({ name: 'builds', championId: selected ?? undefined, role: selRole || undefined, mode: v as BuildMode })}
      />
      {#if mode === 'aram'}<HexFilter />{/if}
      <span class="src">{mode === 'rift' ? t('builds.srcRift') : t('builds.srcAram')}</span>
    </div>

    {#if editing && selected}
      <header class="head">
        <Img src={champIcon(selected)} size={56} />
        <h2>{dd.data?.champions[String(selected)]?.name}</h2>
      </header>
      <MayhemEditor championId={selected} build={content.data.builds[selected] ?? null} onclose={doneEditing} />
    {:else if !selected}
      <div class="empty"><Icon name="builds" size={30} /><p>{t('builds.pickChamp')}</p></div>
    {:else if loading && !showBuild}
      <div class="empty"><p>{t('common.loading')}</p></div>
    {:else if !showBuild || !build}
      <div class="empty">
        <Img src={champIcon(selected)} size={56} />
        <h3>{t('builds.none')}</h3>
        <p>{error ?? t('builds.noneHint')}</p>
        {#if admin}<button class="btn primary" onclick={() => (editing = true)}>{t('mayhem.write')}</button>{/if}
      </div>
    {:else}
      <header class="head">
        <Img src={champIcon(build.championId)} size={56} />
        <div>
          <h2>{dd.data?.champions[String(build.championId)]?.name}{#if build.mode === 'aram' && hexChamp?.champion}{@const tier = aramTier(hexChamp.champion.winRate)}<span class="tier t{tier}">{t(`tiers.${tier}`)}</span>{/if}</h2>
          {#if build.mode === 'aram' && hexChamp?.champion}
            {@const c = hexChamp.champion}
            <div class="sub num">
              {t('hex.headWin')} <b class="wr">{pct1(c.winRate)}%</b> · {t('hex.headRank', c.rank, hexChamp.champions)} · {t('builds.sample', c.games.toLocaleString())} · {t('hex.headPick', share(c.pickRate))} · {hexChamp.source.patch}
            </div>
            <!-- Its win rate in every skill bracket; picking one picks the bracket for the whole page. -->
            {#if hexChamp.bands.length > 1}
              <div class="roles brackets">
                {#each hexChamp.bands as r (r.bracket)}
                  <button class:on={bracket() === r.bracket} title={t('builds.sample', r.games.toLocaleString())} onclick={() => setBracket(r.bracket)}>
                    {t(`bracket.${r.bracket}`)} <span class="num">{pct1(r.winRate)}%</span>
                  </button>
                {/each}
              </div>
            {/if}
          {:else if build.games > 0}<div class="sub num">{t('builds.sample', build.games.toLocaleString())} · {t('builds.wr', pct(build.wins, build.games))}</div>{/if}
          {#if build.mode === 'rift'}
            <div class="roles">
              {#each build.roles as r (r.value)}
                <button class:on={build.role === r.value} onclick={() => (selRole = r.value)}>{roleName(r.value)} <span class="num">{r.games.toLocaleString()}</span></button>
              {/each}
              <!-- The other lanes: off-meta, OP.GG may have a few games there. -->
              {#each LANES.filter((l) => !build!.roles.some((r) => r.value === l)) as l (l)}
                <button class="rare" class:on={build.role === l} title={t('builds.rareLane')} onclick={() => (selRole = l)}>{roleName(l)}</button>
              {/each}
            </div>
            {#if build.rare}<p class="rare-note">{t('builds.rareNote', build.games.toLocaleString())}</p>{/if}
          {/if}
        </div>
        <div class="imp">
          {#if admin}<button class="btn" onclick={() => (editing = true)}><Icon name="edit" size={15} />{build.mine ? t('mayhem.edit') : t('mayhem.write')}</button>{/if}
          {#if build.mode === 'rift'}<label><input type="checkbox" bind:checked={doRunes} /> {t('builds.importRunes')}</label>{/if}
          <label><input type="checkbox" bind:checked={doItems} /> {t('builds.importItems')}</label>
          <button class="btn primary" onclick={doImport} disabled={importing || !app.connected || (!(doRunes && build.mode === 'rift') && !doItems)}><Icon name="download" size={16} />{t('builds.import')}</button>
        </div>
      </header>

      {#if build.mode === 'aram'}
        {#if build.mine}<MayhemBuild build={build.mine} />{/if}
        {#if build.skillMax.length || build.starters.length}
          <section class="card opgg">
            {#if build.skillMax.length}
              <div>
                <div class="label">{t('builds.skills')}</div>
                <div class="skills">
                  {#each build.skillMax as k, i (i)}{#if i}<span class="arrow">›</span>{/if}<b class="key">{k}</b>{/each}
                </div>
              </div>
              {#if build.skillOrder.length}<div class="order">{#each build.skillOrder as k, i (i)}<span class="num">{k}</span>{/each}</div>{/if}
            {/if}
            {#if build.starters.length && !build.mine}
              <div>
                <div class="label">{t('builds.starters')}</div>
                <div class="opt">{#each build.starters[0].value as id, i (i)}<ItemIcon id={id} size={32} />{/each}</div>
              </div>
            {/if}
            <p class="note src-note">{t('hex.opggNote')}</p>
          </section>
        {/if}
        <MayhemChampion championId={build.championId} bind:page={hexPage} />
      {:else}
        <section class="card">
          <div class="label">{t('builds.runes')} <span class="num">· {t('builds.pick', p(build.runes[0]?.games ?? 0))} · {t('builds.wr', pct(build.runes[0]?.wins ?? 0, build.runes[0]?.games ?? 0))}</span></div>
          {#if page && primaryTree}
            <div class="runes">
              <div class="tree">
                <div class="tree-h"><Img src={runeIcon(primaryTree.id)} size={22} round /><b>{primaryTree.name}</b></div>
                {#each primaryTree.slots as slot, si (si)}
                  <div class="slot" class:keystone={si === 0}>
                    {#each slot as r (r.id)}
                      <span class="r" class:on={page.perks.includes(r.id)} title={r.name}><Img src={runeIcon(r.id)} size={si === 0 ? 40 : 30} round /></span>
                    {/each}
                  </div>
                {/each}
              </div>
              {#if subTree}
                <div class="tree">
                  <div class="tree-h"><Img src={runeIcon(subTree.id)} size={22} round /><b>{subTree.name}</b></div>
                  {#each subTree.slots.slice(1) as slot, si (si)}
                    <div class="slot">
                      {#each slot as r (r.id)}
                        <span class="r" class:on={page.perks.includes(r.id)} title={r.name}><Img src={runeIcon(r.id)} size={30} round /></span>
                      {/each}
                    </div>
                  {/each}
                  {#if page.shards.length}
                    <div class="shards">
                      {#each page.shards as s, i (i)}<span>{SHARD_NAMES[s]?.[i18n.lang] ?? s}</span>{/each}
                    </div>
                  {/if}
                </div>
              {/if}
              <div class="picked">
                {#each page.perks as id, i (i)}<div><Img src={runeIcon(id)} size={20} round /><span>{rune(id)?.name}</span></div>{/each}
              </div>
            </div>
          {/if}
        </section>
      <div class="two">
        <section class="card">
          <div class="label">{t('builds.spells')}</div>
          {#each build.spells as sp, i (i)}
            <div class="opt">
              <Img src={spellIcon(sp.value[0])} size={32} title={spell(sp.value[0])?.name} />
              <Img src={spellIcon(sp.value[1])} size={32} title={spell(sp.value[1])?.name} />
              <span class="num">{t('builds.pick', p(sp.games))}</span>
              <span class="num dim">{t('builds.wr', pct(sp.wins, sp.games))}</span>
            </div>
          {/each}
          {#if build.skillMax.length}
            <div class="label gap">{t('builds.skills')}</div>
            <div class="skills">
              {#each build.skillMax as k, i (i)}{#if i}<span class="arrow">›</span>{/if}<b class="key">{k}</b>{/each}
            </div>
            {#if build.skillOrder.length}
              <div class="order">{#each build.skillOrder as k, i (i)}<span class="num">{k}</span>{/each}</div>
            {/if}
          {/if}
        </section>
        <section class="card">
          {#if build.starters.length}
            <div class="label">{t('builds.starters')}</div>
            <div class="opt">
              {#each build.starters[0].value as id, i (i)}<ItemIcon id={id} size={32} />{/each}
              <span class="num">{t('builds.pick', p(build.starters[0].games))}</span>
              <span class="num dim">{t('builds.wr', pct(build.starters[0].wins, build.starters[0].games))}</span>
            </div>
          {/if}
          <div class="label" class:gap={build.starters.length}>{t('builds.boots')}</div>
          {#each build.boots as b, i (i)}
            <div class="opt">
              <ItemIcon id={b.value} size={32} />
              <span class="iname">{itemName(b.value)}</span>
              <span class="num">{t('builds.pick', p(b.games))}</span>
              <span class="num dim">{t('builds.wr', pct(b.wins, b.games))}</span>
            </div>
          {/each}
        </section>
      </div>

      <section class="card">
        <div class="label">{t('builds.core')} <span class="num">· {t('builds.pick', p(build.items[0]?.games ?? 0))} · {t('builds.wr', pct(build.items[0]?.wins ?? 0, build.items[0]?.games ?? 0))}</span></div>
        <div class="items">
          {#each build.items as it, i (i)}
            {#if i === build.core && i > 0}<div class="sep"><span>{t('builds.more')}</span></div>{/if}
            <div class="it" class:core={i < build.core}>
              {#if i < build.core}<span class="step num">{i + 1}</span>{/if}
              <ItemIcon id={it.value} size={i < build.core ? 46 : 36} />
              <span class="iname">{itemName(it.value)}</span>
              {#if i >= build.core}
                <span class="num">{t('builds.pick', p(it.games))}</span>
                <span class="num dim">{t('builds.wr', pct(it.wins, it.games))}</span>
              {/if}
            </div>
          {/each}
        </div>
        <p class="note">{t('builds.order')}</p>
      </section>
      {/if}
    {/if}
  </main>
</div>

<style>
  .page {
    height: 100%;
    display: grid;
    grid-template-columns: 290px minmax(0, 1fr);
    overflow: hidden;
  }
  .picker {
    border-right: 1px solid var(--seam);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    color: var(--text-3);
    border-bottom: 1px solid var(--seam);
  }
  .search .field {
    flex: 1;
  }
  .lanes {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--seam);
  }
  .lanes button {
    height: 26px;
    padding: 0 9px;
    border-radius: var(--r-sm);
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 600;
  }
  .lanes button:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .lanes button.on {
    background: var(--panel-3);
    color: var(--you);
    box-shadow: inset 0 -2px 0 var(--you);
  }
  .glabel {
    grid-column: 1 / -1;
    margin: 8px 4px 2px;
    font: 700 11.5px var(--font-score);
    letter-spacing: 0.06em;
    color: var(--text-3);
  }
  .glabel:first-child {
    margin-top: 0;
  }
  .gempty {
    grid-column: 1 / -1;
    padding: 20px 8px;
    text-align: center;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .grid {
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    padding: 10px;
    align-content: start;
  }
  .c {
    display: grid;
    justify-items: center;
    gap: 3px;
    padding: 5px 2px;
    border-radius: var(--r-md);
    color: var(--text-2);
    font-size: 11px;
    min-width: 0;
  }
  .c span {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .c:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .c.on {
    background: var(--panel-3);
    color: var(--you);
  }
  .main {
    overflow-y: auto;
    padding: 14px 20px 24px;
    display: grid;
    gap: 14px;
    align-content: start;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .bar :global(.seg) {
    width: 280px;
  }
  .src {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .label.gap {
    margin-top: 14px;
  }
  .skills {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .key {
    display: inline-grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--r-md);
    background: var(--panel-3);
    color: var(--you);
    font: 700 14px var(--font-score);
  }
  .arrow {
    color: var(--text-3);
  }
  .order {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    margin-top: 8px;
  }
  .order span {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    background: var(--panel-2);
    font-size: 11px;
    color: var(--text-2);
  }
  .step {
    position: absolute;
    top: 4px;
    left: 6px;
    font-size: 11px;
    color: var(--text-3);
  }
  .head {
    display: flex;
    gap: 14px;
    align-items: center;
    flex-wrap: wrap;
  }
  .head h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 22px;
  }
  .tier {
    padding: 1px 7px;
    border-radius: var(--r-sm);
    font: 700 12.5px/18px var(--font-score);
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
  .sub {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .sub .wr {
    color: var(--win);
  }
  .opgg {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px 28px;
  }
  .opgg .label {
    margin-bottom: 6px;
  }
  .opgg .order {
    margin-top: 0;
  }
  .opgg .src-note {
    flex-basis: 100%;
    margin-top: 0;
  }
  .roles {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }
  .roles button {
    height: 24px;
    padding: 0 9px;
    border-radius: var(--r-sm);
    background: var(--panel-2);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
  }
  .roles button.on {
    background: var(--panel-3);
    color: var(--you);
    box-shadow: inset 0 -2px 0 var(--you);
  }
  .brackets .num {
    margin-left: 3px;
    color: var(--text);
  }
  .brackets button.on .num {
    color: inherit;
  }
  .roles button.rare {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--seam);
    color: var(--text-3);
  }
  .roles button.rare:hover {
    color: var(--text);
  }
  .roles button.rare.on {
    color: var(--warn);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--warn) 50%, var(--seam));
  }
  .rare-note {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--warn);
  }
  .roles .num {
    color: var(--text-3);
    font-weight: 500;
  }
  .imp {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
    color: var(--text-2);
  }
  .imp label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }
  input[type='checkbox'] {
    accent-color: var(--you);
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 14px 16px;
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
  .label .num {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
  }
  .runes {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    gap: 28px;
    align-items: start;
  }
  .tree {
    display: grid;
    gap: 10px;
    justify-items: start;
  }
  .tree-h {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .slot {
    display: flex;
    gap: 8px;
  }
  .r {
    display: inline-grid;
    border-radius: 50%;
    filter: grayscale(1);
    opacity: 0.32;
  }
  .r.on {
    filter: none;
    opacity: 1;
    box-shadow: 0 0 0 2px var(--objective);
  }
  .shards {
    display: grid;
    gap: 3px;
    font-size: 12px;
    color: var(--text-2);
    margin-top: 4px;
  }
  .picked {
    display: grid;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .picked div {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .two.one {
    grid-template-columns: 1fr;
  }
  .written {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 4px 10px;
    margin: 0;
    padding: 6px 14px;
    font-size: 12px;
    color: var(--text-3);
    border-bottom: 1px solid var(--seam);
  }
  /* Admin: champions that already have a hand-written build. */
  .c.has :global(.img) {
    box-shadow: 0 0 0 2px var(--win);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
    font-size: 12.5px;
  }
  .opt .iname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .items {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 10px;
  }
  .it {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 3px;
    text-align: center;
    font-size: 12px;
    padding: 6px 4px;
    border-radius: var(--r-md);
  }
  .it.core {
    background: var(--panel-2);
  }
  .it .iname {
    color: var(--text);
    line-height: 1.25;
    min-height: 2.5em;
  }
  .sep {
    grid-column: 1 / -1;
    font-family: var(--font-score);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
    border-top: 1px solid var(--seam);
    padding-top: 8px;
  }
  .dim {
    color: var(--text-3);
  }
  .note {
    color: var(--text-3);
    font-size: 12px;
    margin-top: 8px;
  }
</style>
