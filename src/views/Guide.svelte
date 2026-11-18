<script lang="ts">
  /**
   * Hextech ARAM at a glance: the mode's rules, this season's headliner bands, and every
   * champion's adjustments. All of it comes from the refreshed data (`data` branch).
   */
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { champIcon, champName } from '../lib/dd.svelte';
  import { ago } from '../lib/format';
  import { bi, gameData, loadGameData, MOD_KEYS, modGood, modText, type ModKey } from '../lib/gamedata.svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { go } from '../lib/state.svelte';

  loadGameData();
  let view = $state<'rules' | 'bands' | 'champions'>('rules');
  let filter = $state<'all' | 'buffed' | 'nerfed'>('all');
  let query = $state('');
  let sortKey = $state<ModKey | 'name'>('dealt');
  let sortDesc = $state(false);

  const m = $derived(gameData.data?.mayhem);
  const checked = $derived(gameData.data?.checkedAt ? ago(Date.parse(gameData.data.checkedAt)) : '');

  /** Other modifiers than damage dealt and taken, shown together. */
  const OTHER: ModKey[] = MOD_KEYS.filter((k) => k !== 'dealt' && k !== 'received');

  const rows = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const list = Object.entries(m?.champions ?? {}).map(([id, c]) => {
      const vals = MOD_KEYS.filter((k) => (c.stats[k] ?? 0) !== 0).map((k) => modGood(k, c.stats[k]!));
      return {
        id: Number(id),
        name: champName(Number(id)),
        c,
        buffed: vals.length > 0 && vals.every(Boolean),
        nerfed: vals.length > 0 && vals.every((g) => !g),
        abilityLines: c.abilities.reduce((n, a) => n + a.lines.length, 0),
      };
    });
    const shown = list.filter((r) => (!q || r.name.toLowerCase().includes(q) || r.c.name.toLowerCase().includes(q)) && (filter === 'all' || (filter === 'buffed' ? r.buffed : r.nerfed)));
    const val = (r: (typeof list)[number]) => (sortKey === 'name' ? 0 : (r.c.stats[sortKey] ?? 0));
    return shown.sort((a, b) => (sortKey === 'name' ? a.name.localeCompare(b.name) : (val(a) - val(b)) * (sortDesc ? -1 : 1) || a.name.localeCompare(b.name)));
  });

  function sortBy(k: ModKey | 'name') {
    if (sortKey === k) sortDesc = !sortDesc;
    else {
      sortKey = k;
      sortDesc = false;
    }
  }
  const tone = (k: ModKey, v: number | undefined) => (v === undefined || v === 0 ? '' : modGood(k, v) ? 'good' : 'bad');
</script>

<div class="page">
  <header class="head">
    <h2>{t('mguide.title')}</h2>
    <Segmented
      label={t('meta.view')}
      value={view}
      options={[
        { value: 'rules', label: t('mguide.rules') },
        { value: 'bands', label: t('mguide.bands') },
        { value: 'champions', label: t('mguide.champions') },
      ]}
      onchange={(v) => (view = v as typeof view)}
    />
    {#if view === 'champions'}
      <Segmented
        label={t('mguide.filter')}
        value={filter}
        options={[
          { value: 'all', label: t('mguide.all') },
          { value: 'buffed', label: t('mguide.buffed') },
          { value: 'nerfed', label: t('mguide.nerfed') },
        ]}
        onchange={(v) => (filter = v as typeof filter)}
      />
    {/if}
    <span class="grow"></span>
    {#if view === 'champions'}
      <label class="search"><Icon name="search" size={15} /><input class="field" bind:value={query} placeholder={t('builds.search')} spellcheck="false" /></label>
    {/if}
    {#if m}<span class="dim">{t('mods.source', m.patch, checked)}</span>{/if}
  </header>

  <div class="scroll">
    {#if !m}
      <p class="dim">{t('common.loading')}</p>
    {:else if view === 'rules'}
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
    {:else if view === 'bands'}
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
              <th><button onclick={() => sortBy('name')}>{t('mguide.champion')}</button></th>
              <th><button class:on={sortKey === 'dealt'} onclick={() => sortBy('dealt')}>{t('mods.dealt')}</button></th>
              <th><button class:on={sortKey === 'received'} onclick={() => sortBy('received')}>{t('mods.received')}</button></th>
              <th>{t('mguide.other')}</th>
              <th class="r">{t('mguide.abilities')}</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as r (r.id)}
              <tr onclick={() => go({ name: 'builds', championId: r.id, mode: 'aram' })}>
                <td><span class="who"><Img src={champIcon(r.id)} size={28} />{r.name}</span></td>
                <td class="num {tone('dealt', r.c.stats.dealt)}">{r.c.stats.dealt !== undefined ? modText('dealt', r.c.stats.dealt) : '100%'}</td>
                <td class="num {tone('received', r.c.stats.received)}">{r.c.stats.received !== undefined ? modText('received', r.c.stats.received) : '100%'}</td>
                <td>
                  {#each OTHER.filter((k) => r.c.stats[k] !== undefined && r.c.stats[k] !== 0) as k (k)}
                    <span class="mini {tone(k, r.c.stats[k])}">{t(`mods.${k}`)} {modText(k, r.c.stats[k]!)}</span>
                  {/each}
                </td>
                <td class="r num dim">{r.abilityLines || ''}</td>
              </tr>
            {:else}
              <tr><td colspan="5" class="dim">{t('mguide.noMatch')}</td></tr>
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
  .head {
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
    width: 200px;
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
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover td {
    background: var(--panel-2);
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--text);
    font-weight: 600;
  }
  .r {
    text-align: right;
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
  .mini.good,
  .mini.bad {
    background: var(--panel-2);
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
