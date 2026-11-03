<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import { api } from '../lib/api';
  import { buildIndex, searchChamps } from '../lib/champSearch';
  import { SHARD_NAMES, champIcon, dd, itemIcon, itemName, loadOtherNames, otherNames, rune, runeIcon, spell, spellIcon } from '../lib/dd.svelte';
  import { ago, pct, roleName } from '../lib/format';
  import { i18n, t } from '../lib/i18n.svelte';
  import { app, errorText, toast } from '../lib/state.svelte';
  import type { Build, CrawlStatus } from '../lib/types';

  let { championId, role }: { championId?: number; role?: string } = $props();

  let selected = $state<number | null>(null);
  let selRole = $state('');
  let build = $state<Build | null>(null);
  let loading = $state(false);
  let query = $state('');
  let status = $state<CrawlStatus | null>(null);
  let importing = $state(false);
  let doRunes = $state(true);
  let doItems = $state(true);

  let grid = $state<HTMLElement>();

  $effect(() => {
    if (championId) {
      selected = championId;
      selRole = role ?? '';
      // Opened from search or a link: bring the champion into view in the picker.
      tick().then(() => grid?.querySelector('.c.on')?.scrollIntoView({ block: 'center' }));
    }
  });

  // Any language or pinyin; best matches first while searching, alphabetical otherwise.
  const index = $derived(buildIndex([dd.data?.champions, otherNames.champions]));
  const champs = $derived.by(() => {
    const all = Object.entries(dd.data?.champions ?? {})
      .map(([id, c]) => ({ id: Number(id), ...c }))
      .sort((a, b) => a.name.localeCompare(b.name, i18n.lang === 'zh' ? 'zh-CN' : 'en'));
    if (!query.trim()) return all;
    const rank = new Map(searchChamps(index, query, Infinity).map((h, i) => [h.id, i]));
    return all.filter((c) => rank.has(c.id)).sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
  });

  async function load() {
    if (!selected) return;
    loading = true;
    try {
      build = await api.build(selected, selRole);
    } catch (e) {
      build = null;
      toast(errorText(e), 'error');
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    selected;
    selRole;
    load();
  });

  async function refreshStatus() {
    try {
      status = await api.crawlStatus();
    } catch {
      /* not connected */
    }
  }
  $effect(() => {
    refreshStatus();
    const id = setInterval(refreshStatus, 3000);
    return () => clearInterval(id);
  });
  // New data arrived while looking at an empty build: try again.
  $effect(() => {
    if (status?.running && !build && selected && status.done % 25 === 0) load();
  });

  async function doImport() {
    if (!build) return;
    importing = true;
    try {
      const r = await api.importBuild(build.championId, build.role, doRunes, doItems);
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
  const shards = $derived.by(() => {
    const tags = selected ? dd.data?.champions[String(selected)]?.tags ?? [] : [];
    return [tags.includes('Marksman') ? 5005 : 5008, 5008, 5011];
  });
  const p = (n: number) => (build ? pct(n, build.games) : 0);
</script>

<div class="page">
  <aside class="picker">
    <div class="search">
      <Icon name="search" size={15} />
      <input class="field" bind:value={query} placeholder={t('builds.search')} spellcheck="false" onfocus={loadOtherNames} />
    </div>
    <div class="grid" bind:this={grid}>
      {#each champs as c (c.id)}
        <button class="c" class:on={selected === c.id} title={c.name} onclick={() => { selected = c.id; selRole = ''; }}>
          <Img src={champIcon(c.id)} size={42} alt={c.name} />
          <span>{c.name}</span>
        </button>
      {/each}
    </div>
  </aside>

  <main class="main">
    <div class="status">
      {#if status?.running}
        <span class="spin"><Icon name="refresh" size={14} /></span>
        <span>{t('builds.crawling', status.done, status.total)}</span>
      {:else if status}
        <span>{t('builds.dataStatus', status.games, status.champions)}{#if status.lastFinished} · {t('builds.lastRun', ago(status.lastFinished))}{/if}</span>
        <button class="btn sm ghost" onclick={async () => { await api.startCrawl(); refreshStatus(); }} disabled={!app.connected}>{t('builds.crawl')}</button>
      {/if}
    </div>

    {#if !selected}
      <div class="empty"><Icon name="builds" size={30} /><p>{t('builds.pickChamp')}</p></div>
    {:else if loading && !build}
      <div class="empty"><p>{t('common.loading')}</p></div>
    {:else if !build}
      <div class="empty">
        <Img src={champIcon(selected)} size={56} />
        <h3>{t('builds.none')}</h3>
        <p>{t('builds.noneHint')}</p>
      </div>
    {:else}
      <header class="head">
        <Img src={champIcon(build.championId)} size={56} />
        <div>
          <h2>{dd.data?.champions[String(build.championId)]?.name}</h2>
          <div class="sub num">{t('builds.sample', build.games)} · {t('builds.wr', pct(build.wins, build.games))}</div>
          <div class="roles">
            {#each build.roles as r (r.value)}
              <button class:on={build.role === r.value} onclick={() => (selRole = r.value)}>{roleName(r.value)} <span class="num">{r.games}</span></button>
            {/each}
          </div>
        </div>
        <div class="imp">
          <label><input type="checkbox" bind:checked={doRunes} /> {t('builds.importRunes')}</label>
          <label><input type="checkbox" bind:checked={doItems} /> {t('builds.importItems')}</label>
          <button class="btn primary" onclick={doImport} disabled={importing || !app.connected || (!doRunes && !doItems)}><Icon name="download" size={16} />{t('builds.import')}</button>
        </div>
      </header>

      <section class="card">
        <div class="label">{t('builds.runes')} <span class="num">· {t('builds.pick', p(build.runes[0]?.games ?? 0))} · {t('builds.wr', pct(build.runes[0]?.wins ?? 0, build.runes[0]?.games ?? 0))}</span></div>
        {#if page && primaryTree}
          <div class="runes">
            <div class="tree">
              <div class="tree-h"><Img src={runeIcon(primaryTree.id)} size={22} round /><b>{primaryTree.name}</b></div>
              {#each primaryTree.slots as slot, si (si)}
                <div class="slot" class:key={si === 0}>
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
                <div class="shards">
                  {#each shards as s, i (i)}<span>{SHARD_NAMES[s]?.[i18n.lang] ?? s}</span>{/each}
                </div>
                <p class="note">{t('builds.shards')}</p>
              </div>
            {/if}
            <div class="picked">
              {#each page.perks as id (id)}<div><Img src={runeIcon(id)} size={20} round /><span>{rune(id)?.name}</span></div>{/each}
            </div>
          </div>
        {/if}
      </section>

      <div class="two">
        <section class="card">
          <div class="label">{t('builds.spells')}</div>
          {#each build.spells as sp (sp.value.join())}
            <div class="opt">
              <Img src={spellIcon(sp.value[0])} size={32} title={spell(sp.value[0])?.name} />
              <Img src={spellIcon(sp.value[1])} size={32} title={spell(sp.value[1])?.name} />
              <span class="num">{t('builds.pick', p(sp.games))}</span>
              <span class="num dim">{t('builds.wr', pct(sp.wins, sp.games))}</span>
            </div>
          {/each}
        </section>
        <section class="card">
          <div class="label">{t('builds.boots')}</div>
          {#each build.boots as b (b.value)}
            <div class="opt">
              <Img src={itemIcon(b.value)} size={32} title={itemName(b.value)} />
              <span class="iname">{itemName(b.value)}</span>
              <span class="num">{t('builds.pick', p(b.games))}</span>
              <span class="num dim">{t('builds.wr', pct(b.wins, b.games))}</span>
            </div>
          {/each}
        </section>
      </div>

      <section class="card">
        <div class="label">{t('builds.core')}</div>
        <div class="items">
          {#each build.items as it, i (it.value)}
            {#if i === 3}<div class="sep"><span>{t('builds.more')}</span></div>{/if}
            <div class="it" class:core={i < 3}>
              <Img src={itemIcon(it.value)} size={i < 3 ? 46 : 36} title={itemName(it.value)} />
              <span class="iname">{itemName(it.value)}</span>
              <span class="num">{t('builds.pick', p(it.games))}</span>
              <span class="num dim">{t('builds.wr', pct(it.wins, it.games))}</span>
            </div>
          {/each}
        </div>
        <p class="note">{t('builds.order')}</p>
      </section>
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
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-3);
    font-size: 12.5px;
    min-height: 28px;
  }
  .spin {
    display: inline-grid;
    animation: spin 1.2s linear infinite;
    color: var(--you);
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .head {
    display: flex;
    gap: 14px;
    align-items: center;
    flex-wrap: wrap;
  }
  .head h2 {
    font-size: 22px;
  }
  .sub {
    color: var(--text-3);
    font-size: 12.5px;
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
