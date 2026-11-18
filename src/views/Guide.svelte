<script lang="ts" module>
  import type { ChampClass } from '../lib/types';
  import type { ModKey } from '../lib/gamedata.svelte';

  /**
   * What the page shows, kept while the app runs: going back to it (after opening a champion)
   * lands on the same tab, filters, open row and scroll position.
   */
  const ui = $state({
    view: 'rules' as 'rules' | 'bands' | 'champions',
    filter: 'all' as 'all' | 'buffed' | 'nerfed',
    cls: 'all' as 'all' | ChampClass,
    query: '',
    sortKey: 'dealt' as ModKey | 'name',
    sortDesc: false,
    open: 0,
    scroll: { rules: 0, bands: 0, champions: 0 } as Record<string, number>,
  });
</script>

<script lang="ts">
  /**
   * Hextech ARAM at a glance: the mode's rules, this season's headliner bands, and every
   * champion's adjustments. All of it comes from the refreshed data (`data` branch).
   */
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { findChamps, loadChampSearch } from '../lib/champFind.svelte';
  import { champIcon, champName, dd } from '../lib/dd.svelte';
  import { ago } from '../lib/format';
  import { bi, gameData, loadGameData, MOD_KEYS, modGood, modText, type AbilityMods } from '../lib/gamedata.svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { go } from '../lib/state.svelte';
  import { tick } from 'svelte';

  loadGameData();
  loadChampSearch();
  const CLASSES: ChampClass[] = ['Fighter', 'Assassin', 'Mage', 'Tank', 'Support', 'Marksman'];

  const m = $derived(gameData.data?.mayhem);
  const checked = $derived(gameData.data?.checkedAt ? ago(Date.parse(gameData.data.checkedAt)) : '');

  /** Other modifiers than damage dealt and taken, shown together. */
  const OTHER: ModKey[] = MOD_KEYS.filter((k) => k !== 'dealt' && k !== 'received');

  /** An ability's lines of one kind (Hextech ARAM only, or regular ARAM's) in the UI language. */
  function linesOf(a: AbilityMods, mode: 'aram' | 'mayhem') {
    return a.lines.filter((l) => l.mode === mode).map((l) => (i18n.lang === 'zh' ? l.zh : l.text)).filter((x): x is string => Boolean(x));
  }
  function changes(abilities: AbilityMods[], mode: 'aram' | 'mayhem') {
    return abilities.map((a) => ({ a, lines: linesOf(a, mode) })).filter((x) => x.lines.length);
  }

  const rows = $derived.by(() => {
    const found = findChamps(ui.query);
    const rank = found ? new Map(found.map((id, i) => [id, i])) : null;
    const list = Object.entries(m?.champions ?? {}).map(([id, c]) => {
      const vals = MOD_KEYS.filter((k) => (c.stats[k] ?? 0) !== 0).map((k) => modGood(k, c.stats[k]!));
      return {
        id: Number(id),
        name: champName(Number(id)),
        c,
        tags: dd.data?.champions[id]?.tags ?? [],
        buffed: vals.length > 0 && vals.every(Boolean),
        nerfed: vals.length > 0 && vals.every((g) => !g),
        mayhem: changes(c.abilities, 'mayhem'),
        aram: changes(c.abilities, 'aram'),
      };
    });
    const shown = list.filter(
      (r) => (!rank || rank.has(r.id)) && (ui.cls === 'all' || r.tags.includes(ui.cls)) && (ui.filter === 'all' || (ui.filter === 'buffed' ? r.buffed : r.nerfed)),
    );
    if (rank && !ui.sortKey) return shown.sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
    const val = (r: (typeof list)[number]) => (ui.sortKey === 'name' ? 0 : (r.c.stats[ui.sortKey] ?? 0));
    return shown.sort((a, b) => (ui.sortKey === 'name' ? a.name.localeCompare(b.name) : (val(a) - val(b)) * (ui.sortDesc ? -1 : 1) || a.name.localeCompare(b.name)));
  });

  function sortBy(k: ModKey | 'name') {
    if (ui.sortKey === k) ui.sortDesc = !ui.sortDesc;
    else {
      ui.sortKey = k;
      ui.sortDesc = false;
    }
  }
  const tone = (k: ModKey, v: number | undefined) => (v === undefined || v === 0 ? '' : modGood(k, v) ? 'good' : 'bad');
  const count = (list: { lines: string[] }[]) => list.reduce((n, x) => n + x.lines.length, 0);

  // Scroll position per tab, back where it was when the page comes back.
  let scroller = $state<HTMLElement>();
  $effect(() => {
    const view = ui.view;
    const el = scroller;
    if (!el || !m) return;
    tick().then(() => (el.scrollTop = ui.scroll[view] ?? 0));
  });
</script>

{#snippet changeList(list: { a: AbilityMods; lines: string[] }[])}
  {#each list as x (x.a.name.en)}
    <div class="ab">
      <span class="key num">{x.a.key || '·'}</span>
      <div>
        <div class="ab-name">{bi(x.a.name)}</div>
        <ul>
          {#each x.lines as l, i (i)}<li>{l}</li>{/each}
        </ul>
      </div>
    </div>
  {/each}
{/snippet}

<div class="page">
  <header class="head">
    <h2>{t('mguide.title')}</h2>
    <Segmented
      label={t('meta.view')}
      value={ui.view}
      options={[
        { value: 'rules', label: t('mguide.rules') },
        { value: 'bands', label: t('mguide.bands') },
        { value: 'champions', label: t('mguide.champions') },
      ]}
      onchange={(v) => (ui.view = v as typeof ui.view)}
    />
    <span class="grow"></span>
    {#if m}<span class="dim">{t('mods.source', m.patch, checked)}</span>{/if}
  </header>
  {#if ui.view === 'champions'}
    <div class="filters">
      <Segmented
        label={t('meta.class')}
        value={ui.cls}
        options={(['all', ...CLASSES] as const).map((c) => ({ value: c, label: c === 'all' ? t('meta.allClasses') : t(`class.${c}`) }))}
        onchange={(v) => (ui.cls = v as typeof ui.cls)}
      />
      <Segmented
        label={t('mguide.filter')}
        value={ui.filter}
        options={[
          { value: 'all', label: t('mguide.all') },
          { value: 'buffed', label: t('mguide.buffed') },
          { value: 'nerfed', label: t('mguide.nerfed') },
        ]}
        onchange={(v) => (ui.filter = v as typeof ui.filter)}
      />
      <span class="grow"></span>
      <label class="search"><Icon name="search" size={15} /><input class="field" bind:value={ui.query} placeholder={t('builds.search')} spellcheck="false" /></label>
    </div>
  {/if}

  <div class="scroll" bind:this={scroller} onscroll={(e) => (ui.scroll[ui.view] = (e.currentTarget as HTMLElement).scrollTop)}>
    {#if !m}
      <p class="dim">{t('common.loading')}</p>
    {:else if ui.view === 'rules'}
      <div class="sections">
        {#each m.sections as s (s.id)}
          <section class="box">
            <h3>{bi(s.title)}</h3>
            <dl>
              {#each s.items as it (it.label.en)}
                <dt>{bi(it.label)}</dt>
                <dd class:hl={it.highlight}>
                  {bi(it.text)}
                  {#if it.note}<span class="note">{bi(it.note)}</span>{/if}
                </dd>
              {/each}
            </dl>
          </section>
        {/each}
      </div>
    {:else if ui.view === 'bands'}
      <p class="lead">{t('mguide.bandsLead')}</p>
      <div class="bands">
        {#each m.headliners as h (h.id)}
          <section class="box band">
            <h3>{bi(h.name)}{#if !h.official && i18n.lang === 'zh'}<span class="tag" title={t('mguide.unofficialHint')}>{t('mguide.unofficial')}</span>{/if}</h3>
            <p>{bi(h.effect)}</p>
          </section>
        {/each}
      </div>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th><button class:on={ui.sortKey === 'name'} onclick={() => sortBy('name')}>{t('mguide.champion')}</button></th>
              <th><button class:on={ui.sortKey === 'dealt'} onclick={() => sortBy('dealt')}>{t('mods.dealt')}</button></th>
              <th><button class:on={ui.sortKey === 'received'} onclick={() => sortBy('received')}>{t('mods.received')}</button></th>
              <th>{t('mguide.other')}</th>
              <th title={t('mguide.mayhemAbilitiesHint')}>{t('mguide.mayhemAbilities')}</th>
              <th title={t('mguide.aramAbilitiesHint')}>{t('mguide.aramAbilities')}</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as r (r.id)}
              <tr class:open={ui.open === r.id}>
                <td>
                  <button class="who" title={t('mguide.openChampion')} onclick={() => go({ name: 'builds', championId: r.id, mode: 'aram' })}>
                    <Img src={champIcon(r.id)} size={28} />{r.name}
                  </button>
                </td>
                <td class="num {tone('dealt', r.c.stats.dealt)}">{r.c.stats.dealt !== undefined ? modText('dealt', r.c.stats.dealt) : '100%'}</td>
                <td class="num {tone('received', r.c.stats.received)}">{r.c.stats.received !== undefined ? modText('received', r.c.stats.received) : '100%'}</td>
                <td>
                  {#each OTHER.filter((k) => r.c.stats[k] !== undefined && r.c.stats[k] !== 0) as k (k)}
                    <span class="mini {tone(k, r.c.stats[k])}">{t(`mods.${k}`)} {modText(k, r.c.stats[k]!)}</span>
                  {/each}
                </td>
                {#each [r.mayhem, r.aram] as list, i (i)}
                  <td>
                    {#if list.length}
                      <button class="more" aria-expanded={ui.open === r.id} onclick={() => (ui.open = ui.open === r.id ? 0 : r.id)}>
                        {t('mguide.changes', count(list))}
                        {list.map((x) => x.a.key || bi(x.a.name)).join(' ')}
                        <Icon name="chevron-down" size={13} />
                      </button>
                    {:else}<span class="dim">-</span>{/if}
                  </td>
                {/each}
              </tr>
              {#if ui.open === r.id}
                <tr class="detail">
                  <td colspan="6">
                    <div class="split">
                      <section>
                        <h4>{t('mguide.mayhemAbilities')}</h4>
                        {#if r.mayhem.length}{@render changeList(r.mayhem)}{:else}<p class="dim">{t('mguide.none')}</p>{/if}
                      </section>
                      <section>
                        <h4>{t('mguide.aramAbilities')}</h4>
                        {#if r.aram.length}{@render changeList(r.aram)}{:else}<p class="dim">{t('mguide.none')}</p>{/if}
                      </section>
                    </div>
                  </td>
                </tr>
              {/if}
            {:else}
              <tr><td colspan="6" class="dim">{t('mguide.noMatch')}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="dim small">{t('mguide.tableNote')}</p>
    {/if}
    {#if m}
      <p class="credit">
        {t('mguide.credit')}
        <button class="link" onclick={() => openUrl(m.source.url)}>{t('mguide.wiki')}</button> ({m.source.license})
      </p>
    {/if}
  </div>
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
  .head,
  .filters {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .head h2 {
    font-size: 22px;
  }
  .grow {
    flex: 1;
  }
  .dim {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .small {
    font-size: 12px;
    margin: 8px 0 0;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
  }
  .search .field {
    width: 220px;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .sections {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
  }
  .box {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 14px 16px;
    min-width: 0;
  }
  .box h3 {
    font-size: 14px;
    margin: 0 0 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 8px 16px;
    margin: 0;
    font-size: 13px;
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    color: var(--text);
  }
  dd.hl {
    color: var(--objective);
    font-weight: 600;
  }
  .note {
    display: block;
    margin-top: 3px;
    color: var(--warn);
    font-size: 12px;
    font-weight: 400;
  }
  .lead {
    margin: 0 0 12px;
    color: var(--text-2);
    font-size: 13px;
  }
  .bands {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
  }
  .band p {
    margin: 0;
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.55;
  }
  .tag {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--r-sm);
    background: var(--panel-3);
    color: var(--text-3);
  }
  .table-wrap {
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--panel);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    text-align: left;
    padding: 8px 12px;
    color: var(--text-3);
    font-weight: 600;
    font-size: 12px;
    border-bottom: 1px solid var(--seam);
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel);
  }
  th button {
    color: inherit;
    font: inherit;
  }
  th button.on {
    color: var(--you);
  }
  td {
    padding: 6px 12px;
    border-bottom: 1px solid var(--seam);
    color: var(--text-2);
  }
  tbody tr:not(.detail):hover td,
  tr.open td {
    background: var(--panel-2);
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--text);
    font: inherit;
    font-weight: 600;
    padding: 0;
  }
  .who:hover {
    color: var(--you);
  }
  .more {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: var(--r-sm);
    background: var(--panel-3);
    color: var(--text);
    font: inherit;
    font-size: 12px;
  }
  .more:hover {
    color: var(--you);
  }
  .more[aria-expanded='true'] :global(svg) {
    transform: rotate(180deg);
  }
  .good {
    color: var(--win);
  }
  .bad {
    color: var(--death);
  }
  .mini {
    display: inline-block;
    margin: 1px 6px 1px 0;
    padding: 1px 7px;
    border-radius: var(--r-sm);
    background: var(--panel-2);
    font-size: 12px;
  }
  tr.detail td {
    background: var(--bg-sunk);
    padding: 12px 16px;
  }
  .split {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
  }
  .split h4 {
    margin: 0 0 8px;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .ab {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr);
    gap: 10px;
    margin-bottom: 8px;
  }
  .key {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    background: var(--panel-3);
    color: var(--text-2);
    font: 700 12px var(--font-score);
  }
  .ab-name {
    color: var(--text);
    font-weight: 600;
    line-height: 24px;
  }
  .ab ul {
    margin: 0;
    padding-left: 16px;
    font-size: 12.5px;
  }
  .ab li {
    margin: 2px 0;
  }
  .credit {
    margin: 14px 0 0;
    color: var(--text-3);
    font-size: 12px;
  }
  .link {
    color: var(--you);
    font: inherit;
    padding: 0;
  }
</style>
