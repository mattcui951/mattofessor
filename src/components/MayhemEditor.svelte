<script lang="ts">
  /** Writes a champion's Hextech ARAM build (admin): items added from a searchable list. */
  import Icon from './Icon.svelte';
  import Img from './Img.svelte';
  import { dd, itemIcon, itemName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { editContent } from '../lib/content.svelte';
  import { errorText, toast } from '../lib/state.svelte';
  import type { Item, MayhemBuild } from '../lib/types';

  let { championId, build, onclose }: { championId: number; build: MayhemBuild | null; onclose: () => void } = $props();

  type List = 'starters' | 'order' | 'situational';
  const LISTS: List[] = ['starters', 'order', 'situational'];

  // An editable copy; the saved build changes only on Save.
  let draft = $state<MayhemBuild>({ starters: [], order: [], situational: [], note: '', updated: '' });
  $effect(() => {
    championId;
    draft = build ? structuredClone($state.snapshot(build)) : { starters: [], order: [], situational: [], note: '', updated: '' };
  });
  let target = $state<List>('order');
  let query = $state('');
  let saving = $state(false);

  /** Items sold in Hextech ARAM (`aram`, its own ids only): finished items and boots first, then by price. */
  const items = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const byName = new Map<string, { id: number } & Item>();
    for (const [id, it] of Object.entries(dd.data?.items ?? {})) {
      if (!it.aram || it.gold <= 0) continue;
      const seen = byName.get(it.name);
      if (!seen || Number(id) < seen.id) byName.set(it.name, { id: Number(id), ...it });
    }
    return [...byName.values()]
      .filter((it) => !q || it.name.toLowerCase().includes(q) || String(it.id) === q)
      .sort((a, b) => Number(b.aramLegendary || b.boots) - Number(a.aramLegendary || a.boots) || b.gold - a.gold);
  });

  function add(id: number) {
    draft[target] = [...draft[target], id];
  }
  function move(list: List, i: number, by: number) {
    const a = [...draft[list]];
    const j = i + by;
    if (j < 0 || j >= a.length) return;
    [a[i], a[j]] = [a[j], a[i]];
    draft[list] = a;
  }
  function remove(list: List, i: number) {
    draft[list] = draft[list].filter((_, k) => k !== i);
  }

  async function save(b: MayhemBuild | null) {
    saving = true;
    try {
      await editContent({ kind: 'build', championId, build: b });
      toast(t('mayhem.saved'), 'good');
      onclose();
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      saving = false;
    }
  }
</script>

<section class="card">
  <div class="label">{t('mayhem.edit')}</div>
  {#each LISTS as list (list)}
    <div class="list" class:on={target === list}>
      <button class="name" onclick={() => (target = list)} title={t('mayhem.addTo')}>
        <span class="radio"></span>{t(`mayhem.${list}`)}
      </button>
      <div class="chips">
        {#each draft[list] as id, i (i)}
          <span class="chip">
            {#if list === 'order'}<span class="n num">{i + 1}</span>{/if}
            <Img src={itemIcon(id)} size={30} title={itemName(id)} />
            <span class="acts">
              <button aria-label={t('mayhem.earlier')} title={t('mayhem.earlier')} disabled={i === 0} onclick={() => move(list, i, -1)}>‹</button>
              <button aria-label={t('mayhem.remove')} title={t('mayhem.remove')} onclick={() => remove(list, i)}>×</button>
              <button aria-label={t('mayhem.later')} title={t('mayhem.later')} disabled={i === draft[list].length - 1} onclick={() => move(list, i, 1)}>›</button>
            </span>
          </span>
        {:else}
          <span class="empty">{target === list ? t('mayhem.empty') : '-'}</span>
        {/each}
      </div>
    </div>
  {/each}

  <div class="pick">
    <div class="search">
      <Icon name="search" size={15} />
      <input class="field" bind:value={query} placeholder={t('mayhem.search')} spellcheck="false" />
      <span class="dim">{t('mayhem.addTo')} <b>{t(`mayhem.${target}`)}</b></span>
    </div>
    <div class="results">
      {#each items as it (it.id)}
        <button class="res" class:done={it.legendary || it.boots} title={`${it.name} · ${it.gold}`} onclick={() => add(it.id)}>
          <Img src={itemIcon(it.id)} size={32} />
          <span>{it.name}</span>
        </button>
      {/each}
    </div>
  </div>

  <textarea class="field note" rows="2" bind:value={draft.note} placeholder={t('mayhem.note')}></textarea>
  <div class="buttons">
    {#if build}<button class="btn sm ghost danger" disabled={saving} onclick={() => save(null)}>{t('mayhem.delete')}</button>{/if}
    <span class="grow"></span>
    <button class="btn sm ghost" disabled={saving} onclick={onclose}>{t('mayhem.cancel')}</button>
    <button class="btn sm primary" disabled={saving || (!draft.order.length && !draft.starters.length)} onclick={() => save(draft)}>{t('mayhem.save')}</button>
  </div>
</section>

<style>
  .card {
    background: var(--panel);
    border: 1px solid color-mix(in oklab, var(--you) 45%, var(--seam));
    border-radius: var(--r-lg);
    padding: 14px 16px;
    display: grid;
    gap: 10px;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--you);
  }
  .list {
    display: grid;
    grid-template-columns: 110px 1fr;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: var(--r-md);
  }
  .list.on {
    background: var(--panel-2);
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
    text-align: left;
  }
  .list.on .name {
    color: var(--you);
  }
  .radio {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--seam-strong);
  }
  .list.on .radio {
    border-color: var(--you);
    background: radial-gradient(var(--you) 0 40%, transparent 45%);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-height: 40px;
    align-items: center;
  }
  .chip {
    position: relative;
    display: grid;
    justify-items: center;
    padding: 4px 4px 2px;
    border-radius: var(--r-md);
    background: var(--bg-sunk);
  }
  .n {
    position: absolute;
    top: 1px;
    left: 3px;
    font-size: 10px;
    color: var(--text-3);
  }
  .acts {
    display: flex;
  }
  .acts button {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    font-size: 15px;
    font-weight: 700;
    line-height: 1;
    color: var(--text-2);
    border-radius: 3px;
  }
  .acts button:hover:not(:disabled) {
    color: var(--text);
    background: var(--panel-3);
  }
  .acts button:disabled {
    opacity: 0.25;
  }
  .empty {
    font-size: 12.5px;
    color: var(--text-3);
  }
  .pick {
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    overflow: hidden;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    color: var(--text-3);
    border-bottom: 1px solid var(--seam);
  }
  .search .field {
    flex: 1;
  }
  .search b {
    color: var(--you);
  }
  .results {
    max-height: 260px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 2px;
    padding: 6px;
  }
  .res {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: var(--r-sm);
    font-size: 12.5px;
    color: var(--text-2);
    text-align: left;
    min-width: 0;
  }
  .res span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .res.done {
    color: var(--text);
  }
  .res:hover {
    background: var(--panel-2);
  }
  .note {
    height: auto;
    padding: 8px 10px;
    resize: vertical;
    font: inherit;
    font-size: 13px;
  }
  .buttons {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .grow {
    flex: 1;
  }
  .danger {
    color: var(--death);
  }
  .dim {
    font-size: 12px;
  }
</style>
