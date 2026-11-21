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
    /** White artwork (augments): `Img`'s dark plate. */
    plate?: boolean;
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

  const RARITIES = ['silver', 'gold', 'prismatic'];

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
    <div class="nm"><span class={RARITIES.includes(tone) ? `rar-${tone}` : ''}>{label}</span></div>
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
          <span class="icons">{#if c.glyph}<span class="glyph" aria-hidden="true">{c.glyph}</span>{:else}<Img src={c.icon} size={30} round={c.round} plate={c.plate} />{/if}{#if c.icon2 !== undefined}<Img src={c.icon2} size={30} round={c.round} />{/if}</span>
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
    align-self: start;
    display: grid;
    /* One height for the label column and every card, so the three lines line up. */
    grid-template-rows: 76px repeat(3, 22px);
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--panel-3);
    color: var(--text-2);
    --tone: var(--text-2);
  }
  .lab.prismatic {
    --tone: var(--rar-prismatic);
  }
  .lab.gold {
    --tone: var(--rar-gold);
  }
  .lab.silver {
    --tone: var(--rar-silver);
  }
  /* Prismatic: the side bar is the rainbow too. */
  .lab.prismatic .nm {
    box-shadow: none;
    background:
      var(--rar-prismatic-bar) left / 3px 100% no-repeat,
      color-mix(in oklab, var(--tone) 16%, var(--panel-3));
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
  /* Cards wrap onto more lines instead of scrolling sideways, and stretch to fill the width; each
     line keeps the label column's order (win rate, difference, share). */
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
    gap: 4px;
  }
  .sc {
    min-width: 0;
    display: grid;
    /* One height for the label column and every card, so the three lines line up. */
    grid-template-rows: 76px repeat(3, 22px);
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
  .sc.click:hover {
    transform: translateY(-2px);
  }
  .sc {
    transition: background 120ms, border-color 120ms, transform 140ms var(--ease-out);
  }
  .empty {
    grid-column: 1 / -1;
    align-self: center;
    margin: 0;
    padding: 0 8px;
    color: var(--text-3);
    font-size: 12.5px;
  }
</style>
