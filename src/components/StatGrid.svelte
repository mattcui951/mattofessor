<script lang="ts" module>
  import type { HoverInfo } from '../lib/hover.svelte';

  export type StatCard = {
    key: string | number;
    icon: string | null;
    /** A second icon beside the first (summoner spell pairs). */
    icon2?: string | null;
    /** Shown in the icon's place (no boots). */
    glyph?: string;
    round?: boolean;
    name: string;
    winRate: number;
    delta: number;
    pickRate: number;
    hover?: HoverInfo;
    /** Picked (a build style's item). */
    on?: boolean;
    onclick?: () => void;
  };
  export type StatSort = 'win' | 'delta' | 'pick';
</script>

<script lang="ts">
  /**
   * A row of stat cards with three lines each (win rate, difference from the champion, share
   * of its players), headed by a label column whose line names sort the row.
   */
  import Img from './Img.svelte';
  import { tip } from '../lib/hover.svelte';
  import { t } from '../lib/i18n.svelte';
  import { delta, pct1, share } from '../lib/meta.svelte';

  let {
    label,
    tone = '',
    cards,
    base,
    sort = $bindable('pick'),
    empty = '',
  }: { label: string; tone?: string; cards: StatCard[]; base: number; sort?: StatSort; empty?: string } = $props();

  const key = (c: StatCard) => (sort === 'win' ? c.winRate : sort === 'delta' ? c.delta : c.pickRate);
  const sorted = $derived([...cards].sort((a, b) => key(b) - key(a)));
  const LINES: { s: StatSort; label: string }[] = $derived([
    { s: 'win', label: t('hex.win') },
    { s: 'delta', label: t('hex.delta') },
    { s: 'pick', label: t('hex.pick') },
  ]);
</script>

<div class="sg">
  <div class="lab {tone}">
    <div class="nm">{label}</div>
    {#each LINES as l (l.s)}
      <button class="rk" class:on={sort === l.s} aria-pressed={sort === l.s} title={t('hex.sortBy', l.label)} onclick={() => (sort = l.s)}>{l.label}</button>
    {/each}
  </div>
  <div class="cards">
    {#each sorted as c (c.key)}
      <svelte:element
        this={c.onclick ? 'button' : 'div'}
        class="sc"
        class:on={c.on}
        class:click={Boolean(c.onclick)}
        role={c.onclick ? undefined : 'group'}
        tabindex={c.onclick ? undefined : 0}
        aria-label={c.name}
        onclick={c.onclick}
        use:tip={() => c.hover ?? null}
      >
        <div class="id">
          <span class="icons">{#if c.glyph}<span class="glyph" aria-hidden="true">{c.glyph}</span>{:else}<Img src={c.icon} size={30} round={c.round} />{/if}{#if c.icon2 !== undefined}<Img src={c.icon2} size={30} round={c.round} />{/if}</span>
          <span class="name">{c.name}</span>
        </div>
        <div class="v num" class:up={c.winRate >= base + 0.005} class:down={c.winRate <= base - 0.005}>{pct1(c.winRate)}%</div>
        <div class="v num" class:up={c.delta > 0.005} class:down={c.delta < -0.005}>{delta(c.delta)}</div>
        <div class="v num pk">{share(c.pickRate)}%</div>
      </svelte:element>
    {:else}
      <p class="empty">{empty || t('hex.none')}</p>
    {/each}
  </div>
</div>

<style>
  .sg {
    display: grid;
    grid-template-columns: 82px minmax(0, 1fr);
    gap: 6px;
  }
  .lab {
    display: grid;
    grid-template-rows: 1fr repeat(3, 22px);
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--panel-3);
    color: var(--text-2);
    --tone: var(--text-2);
  }
  .lab.prismatic {
    --tone: var(--assist);
  }
  .lab.gold {
    --tone: var(--objective);
  }
  .lab.silver {
    --tone: var(--text-2);
  }
  .lab.items,
  .lab.boots,
  .lab.spells {
    --tone: var(--you);
  }
  .lab.bad {
    --tone: var(--death);
  }
  .nm {
    display: grid;
    place-items: center;
    padding: 6px 4px;
    text-align: center;
    font: 700 12.5px/1.25 var(--font-score);
    letter-spacing: 0.01em;
    white-space: pre-line;
    overflow-wrap: anywhere;
    hyphens: auto;
    letter-spacing: 0.04em;
    color: var(--tone);
    background: color-mix(in oklab, var(--tone) 16%, var(--panel-3));
    box-shadow: inset 3px 0 0 var(--tone);
  }
  .rk {
    font-size: 11.5px;
    color: var(--text-3);
    border-top: 1px solid var(--seam);
    background: color-mix(in oklab, var(--tone) 6%, var(--panel-2));
  }
  .rk:hover {
    color: var(--text);
  }
  .rk.on {
    color: var(--text);
    font-weight: 700;
    box-shadow: inset 3px 0 0 var(--tone);
  }
  .rk.on::after {
    content: ' ↓';
    color: var(--tone);
  }
  .cards {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    padding-bottom: 4px;
    scrollbar-width: thin;
  }
  .sc {
    flex: none;
    width: 80px;
    display: grid;
    grid-template-rows: 1fr repeat(3, 22px);
    border-radius: var(--r-md);
    background: var(--panel-2);
    border: 1px solid transparent;
    text-align: center;
    font: inherit;
    color: inherit;
    padding: 0;
    cursor: default;
    outline: none;
  }
  .sc.click {
    cursor: pointer;
  }
  .sc:hover,
  .sc:focus-visible {
    border-color: var(--seam-strong);
    background: var(--panel-3);
  }
  .sc.on {
    border-color: var(--you);
    box-shadow: inset 0 0 0 1px var(--you);
  }
  .id {
    display: grid;
    justify-items: center;
    align-content: start;
    gap: 4px;
    padding: 7px 4px 5px;
  }
  .icons {
    display: flex;
    gap: 2px;
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: var(--r-md);
    background: var(--panel-3);
    color: var(--text-3);
    font-size: 17px;
  }
  .name {
    font-size: 11.5px;
    line-height: 1.25;
    color: var(--text);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
    hyphens: auto;
  }
  .v {
    display: grid;
    place-items: center;
    border-top: 1px solid var(--seam);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
  .v.up {
    color: var(--win);
  }
  .v.down {
    color: var(--death);
  }
  .v.pk {
    color: var(--text);
  }
  .empty {
    align-self: center;
    margin: 0;
    padding: 0 8px;
    color: var(--text-3);
    font-size: 12.5px;
  }
</style>
