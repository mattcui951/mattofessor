<script lang="ts" module>
  import type { ItemClass } from '../lib/types';
  import { day } from '../lib/format';

  // What the page shows, kept while the app runs: back to this page lands on the same tabs.
  let itemCls = $state<'all' | ItemClass>('all');
  let names = $state(false);
</script>

<script lang="ts">
  /**
   * The admin's Hextech ARAM item tier list. Everyone sees it; in admin mode icons are dragged
   * between tiers (or clicked, then S/A/B/C/D or 1-5 pressed). Champions are ranked by win rate
   * on the Meta page's champion board instead (their old hand-made tiers stay in the content).
   */
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import PublishState from '../components/PublishState.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { api } from '../lib/api';
  import { adminMode, content, editContent, loadContent } from '../lib/content.svelte';
  import { dd, ITEM_CLASSES, itemClasses, itemIcon, itemName, OTHER_ITEMS } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { pct1 } from '../lib/meta.svelte';
  import { errorText, go, toast } from '../lib/state.svelte';
  import { tip } from '../lib/hover.svelte';
  import { itemTip } from '../lib/tips.svelte';
  import type { Tier, Tiers } from '../lib/types';

  const TIERS: Tier[] = ['S', 'A', 'B', 'C', 'D'];

  /** The one ranking, all of it or one shop role's items in it. */
  let query = $state('');
  const admin = $derived(adminMode());
  loadContent();

  // Win rates for the tooltips.
  let itemWin = $state<Record<number, number>>({});
  api.aramItems('all').then((o) => (itemWin = Object.fromEntries(o.items.map((r) => [r.id, r.winRate]))), () => {});

  const board = $derived<Tiers>(content.data.itemTiers ?? {});
  const placed = $derived(new Set(TIERS.flatMap((tier) => board[tier] ?? [])));
  /** Whether an item shows in the shop role picked. */
  const shown = (id: number) => itemCls === 'all' || itemClasses(id).includes(itemCls);
  const empty = $derived(![...placed].some(shown));

  /**
   * What can be ranked, highest win rate first: Hextech ARAM's finished items, boots and
   * `OTHER_ITEMS` (a search finds any item it sells).
   */
  const pool = $derived.by(() => {
    const searching = query.trim() !== '';
    const ids = Object.entries(dd.data?.items ?? {}).filter(([id, it]) => (it.aram && (searching || it.aramLegendary || it.boots)) || OTHER_ITEMS.includes(Number(id)));
    const win = itemWin;
    return ids
      .map(([id]) => Number(id))
      .filter(shown)
      .sort((a, b) => (win[b] ?? 0) - (win[a] ?? 0) || label(a).localeCompare(label(b)));
  });
  const unranked = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return pool.filter((id) => !placed.has(id) && (!q || label(id).toLowerCase().includes(q)));
  });
  /** A tier's name: S to D, or the Chinese tier list's own names. */
  const tierName = (tier: Tier) => t(`tiers.${tier}`);

  function label(id: number) {
    return itemName(id);
  }
  function icon(id: number) {
    return itemIcon(id);
  }
  /** The hover card: stats and effects, and Hextech ARAM's win rate (all champions). */
  function card(id: number) {
    const w = itemWin[id];
    return itemTip(id, w ? [{ label: t('meta.winRate'), value: `${pct1(w)}%` }] : []);
  }

  async function save(next: Tiers) {
    // Show the move at once; the saved copy follows.
    content.data.itemTiers = next;
    try {
      await editContent({ kind: 'itemTiers', tiers: next });
    } catch (e) {
      toast(errorText(e), 'error');
    }
  }

  /**
   * Moves an id into a tier (`pool` takes it out of the list), before the `index`th icon shown
   * there: with one item role picked, its place among the others is kept in the whole ranking.
   */
  function place(id: number, to: Tier | 'pool', index = Infinity) {
    const next: Tiers = {};
    for (const tier of TIERS) next[tier] = (board[tier] ?? []).filter((x) => x !== id);
    if (to !== 'pool') {
      const list = next[to]!;
      const visible = list.filter(shown);
      const at = index < visible.length ? list.indexOf(visible[index]!) : visible.length ? list.indexOf(visible[visible.length - 1]!) + 1 : list.length;
      list.splice(at, 0, id);
    }
    save(next);
  }

  // Dragging with the pointer (the page's own drag and drop is taken by the window).
  let selected = $state<number | null>(null);
  let drag = $state<{ id: number; sx: number; sy: number; x: number; y: number; moving: boolean } | null>(null);
  let drop = $state<{ to: Tier | 'pool'; index: number } | null>(null);

  function down(e: PointerEvent, id: number) {
    if (!admin || e.button !== 0) return;
    e.preventDefault();
    drag = { id, sx: e.clientX, sy: e.clientY, x: e.clientX, y: e.clientY, moving: false };
  }

  function target(x: number, y: number): typeof drop {
    const el = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-to]');
    if (!el) return null;
    const to = el.dataset.to as Tier | 'pool';
    if (to === 'pool') return { to, index: 0 };
    // Count the icons before the pointer, in reading order.
    let index = 0;
    for (const c of el.querySelectorAll<HTMLElement>('[data-id]')) {
      if (Number(c.dataset.id) === drag?.id) continue;
      const r = c.getBoundingClientRect();
      if (r.bottom < y || (r.top <= y && y <= r.bottom && r.left + r.width / 2 < x)) index++;
    }
    return { to, index };
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    if (!drag.moving && Math.hypot(e.clientX - drag.sx, e.clientY - drag.sy) < 5) return;
    drag.moving = true;
    drag.x = e.clientX;
    drag.y = e.clientY;
    drop = target(e.clientX, e.clientY);
  }

  function up() {
    if (!drag) return;
    if (drag.moving) {
      if (drop) place(drag.id, drop.to, drop.index);
      selected = null;
    } else {
      selected = selected === drag.id ? null : drag.id;
    }
    drag = null;
    drop = null;
  }

  function key(e: KeyboardEvent) {
    if (drag && e.key === 'Escape') {
      drag = null;
      drop = null;
      return;
    }
    if (!admin || selected === null || (e.target as HTMLElement).closest('input')) return;
    // S to D, or 1 to 5 (best first).
    const tier = (TIERS[Number(e.key) - 1] ?? e.key.toUpperCase()) as Tier;
    if (TIERS.includes(tier)) place(selected, tier);
    else if (e.key === 'Delete' || e.key === 'Backspace') place(selected, 'pool');
    else if (e.key !== 'Escape') return;
    selected = null;
    e.preventDefault();
  }

  /** A click on a row's free space moves the selected icon to its end. */
  function rowClick(e: MouseEvent, to: Tier | 'pool') {
    if (selected === null || (e.target as HTMLElement).closest('[data-id]')) return;
    place(selected, to);
    selected = null;
  }

  $effect(() => {
    itemCls;
    selected = null;
    query = '';
  });

  const updated = $derived(content.data.updated ? day(content.data.updated) : '');
</script>

<svelte:window onpointermove={move} onpointerup={up} onkeydown={key} />

{#snippet tile(id: number, gap = false)}
  <button
    class="tile"
    class:sel={selected === id}
    class:ghosted={drag?.moving && drag.id === id}
    class:gap
    data-id={id}
    aria-label={label(id)}
    use:tip={() => (drag?.moving ? null : card(id))}
    onpointerdown={(e) => down(e, id)}
  >
    <Img src={icon(id)} size={52} />
    {#if names}<span class="name">{label(id)}</span>{/if}
  </button>
{/snippet}

<div class="page">
  <header class="head">
    <h2>{t('tiers.itemsTitle')}</h2>
    <Segmented
      label={t('meta.class')}
      value={itemCls}
      options={(['all', ...ITEM_CLASSES] as const).map((c) => ({ value: c, label: c === 'all' ? t('meta.allClasses') : t(`itemClass.${c}`) }))}
      onchange={(v) => (itemCls = v as typeof itemCls)}
    />
    <span class="grow"></span>
    <button class="btn sm ghost" onclick={() => go({ name: 'meta', mode: 'aram', view: 'champions' })}>{t('tiers.champsMoved')} ›</button>
    {#if admin}<PublishState />{:else if updated}<span class="dim">{t('tiers.updated', updated)}</span>{/if}
    <button class="btn sm ghost" class:on={names} aria-pressed={names} onclick={() => (names = !names)}>{t('tiers.names')}</button>
  </header>

  <div class="scroll">
    {#if empty && !admin}
      <div class="empty"><Icon name="rows" size={30} /><p>{t('tiers.empty')}</p></div>
    {:else}
      <div class="board" class:editing={admin}>
        {#each TIERS as tier (tier)}
          {@const ids = (board[tier] ?? []).filter(shown)}
          <!-- The icon a dragged one would land before (none: after the last). -->
          {@const before = drop?.to === tier ? ids.filter((x) => !(drag?.moving && x === drag.id))[drop.index] : undefined}
          {#if admin || ids.length}
            <div class="row" class:target={drop?.to === tier} data-to={tier} role="presentation" onclick={(e) => rowClick(e, tier)}>
              <div class="label t{tier}" class:word={tierName(tier).length > 1}>{tierName(tier)}</div>
              <div class="tiles" class:names>
                {#each ids as id (id)}{@render tile(id, before === id)}{/each}
                {#if drop?.to === tier && before === undefined}<span class="end"></span>{/if}
              </div>
            </div>
          {/if}
        {/each}
      </div>

      {#if admin}
        <section class="pool" class:target={drop?.to === 'pool'} data-to="pool" role="presentation" onclick={(e) => rowClick(e, 'pool')}>
          <div class="pool-head">
            <span>{t('tiers.unranked', unranked.length)}</span>
            <span class="hint">{t('tiers.hint')}</span>
            <span class="grow"></span>
            <Icon name="search" size={15} />
            <input class="field" bind:value={query} placeholder={t('tiers.findItems')} spellcheck="false" />
          </div>
          <div class="tiles" class:names>
            {#each unranked as id (id)}{@render tile(id)}{/each}
          </div>
        </section>
      {/if}
    {/if}
  </div>
</div>

{#if drag?.moving}
  <div class="dragged" style="left:{drag.x}px;top:{drag.y}px"><Img src={icon(drag.id)} size={52} /></div>
{/if}

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
  .btn.on {
    color: var(--you);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .board {
    /* Full height: the page scrolls rather than squeezing the rows. */
    flex-shrink: 0;
    display: grid;
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--panel);
  }
  .row {
    display: grid;
    grid-template-columns: 76px 1fr;
    min-height: 66px;
    border-top: 1px solid var(--seam);
  }
  .row:first-child {
    border-top: 0;
  }
  .row.target {
    background: var(--panel-2);
  }
  .label {
    display: grid;
    place-items: center;
    font: 700 26px var(--font-score);
    text-align: center;
  }
  /* A name rather than a letter (the Chinese tiers): the interface font, wrapping in two lines. */
  .label.word {
    padding: 0 8px;
    font: 800 19px/1.15 var(--font-ui);
    word-break: keep-all;
  }
  /* Tier colours from the theme (app.css `--tier-*`), the same as every other tier in the app:
     a tinted band with the label in the tier's colour. */
  .label {
    --tone: var(--text-3);
    background: color-mix(in oklab, var(--tone) 16%, var(--panel));
    box-shadow: inset 3px 0 0 var(--tone);
    color: var(--tone);
  }
  .tS {
    --tone: var(--tier-S);
  }
  .tA {
    --tone: var(--tier-A);
  }
  .tB {
    --tone: var(--tier-B);
  }
  .tC {
    --tone: var(--tier-C);
  }
  .tD {
    --tone: var(--tier-D);
  }
  .tiles {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 4px;
    padding: 6px;
  }
  .tile {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 2px;
    padding: 1px;
    border-radius: var(--r-md);
    touch-action: none;
  }
  .editing .tile,
  .pool .tile {
    cursor: grab;
  }
  /* Outside editing (where tiles are dragged) a tile lifts under the pointer. */
  .tile {
    transition: transform 140ms var(--ease-out);
  }
  .board:not(.editing) .tile:hover {
    transform: translateY(-2px);
  }
  .tiles.names .tile {
    width: 64px;
  }
  .name {
    max-width: 62px;
    font-size: 11px;
    line-height: 1.2;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tile.sel {
    box-shadow: 0 0 0 2px var(--you);
  }
  .tile.ghosted {
    opacity: 0.3;
  }
  /* Where a dragged icon lands: a bar before the icon, or after the last one. */
  .tile.gap::before,
  .end {
    content: '';
    width: 3px;
    height: 52px;
    border-radius: 2px;
    background: var(--you);
  }
  .tile.gap::before {
    position: absolute;
    left: -4px;
    top: 1px;
  }
  .pool {
    border: 1px dashed var(--seam-strong);
    border-radius: var(--r-lg);
    background: var(--bg-sunk);
  }
  .pool.target {
    border-color: var(--you);
  }
  .pool-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }
  .pool-head .hint {
    font-weight: 400;
    font-size: 12px;
    color: var(--text-3);
  }
  .pool-head .field {
    width: 180px;
  }
  .pool-head :global(svg) {
    color: var(--text-3);
  }
  .empty {
    margin: auto;
    display: grid;
    justify-items: center;
    gap: 10px;
    color: var(--text-3);
  }
  .dragged {
    position: fixed;
    z-index: 50;
    pointer-events: none;
    /* Beside the pointer, so the drop marker under it stays visible. */
    transform: translate(12px, 10px);
    opacity: 0.9;
    filter: drop-shadow(0 6px 14px rgb(0 0 0 / 0.45));
  }
</style>
