<script lang="ts">
  /**
   * The team gold strip: while Tab is held, each team's item value on each side of the timer at
   * the top of the game, and the gap on your team's side (+ ahead, − behind). The window spans
   * both panels and the timer between them and ignores the mouse; sizes are in vh so they follow
   * the game's height. Placed by `core::gold_rect` (the panels are `GOLD_PANEL` of its 638 units).
   */
  import { dd } from '../lib/dd.svelte';
  import { compact } from '../lib/format';
  import { teamGold } from '../lib/gold';
  import { t } from '../lib/i18n.svelte';
  import { app } from '../lib/state.svelte';

  const price = (id: number) => dd.data?.items[String(id)]?.gold;
  const gold = $derived(app.ingame && dd.data ? teamGold(app.ingame, price) : null);
  /** Your side; ORDER is blue, on the left as in the game's scoreboard. */
  const mine = $derived(app.ingame?.myTeam === 'CHAOS' ? 'chaos' : 'order');
  const gap = $derived(gold ? (mine === 'order' ? gold.order - gold.chaos : gold.chaos - gold.order) : 0);
  const signed = (n: number) => (n > 0 ? '+' : n < 0 ? '−' : '') + compact(Math.abs(n));
</script>

{#snippet coin()}
  <svg class="coin" viewBox="0 0 24 24" aria-hidden="true">
    <circle cx="12" cy="12" r="9" fill="var(--objective)" />
    <circle cx="12" cy="12" r="5.6" fill="none" stroke="color-mix(in oklab, var(--objective) 55%, #000)" stroke-width="1.8" />
  </svg>
{/snippet}

{#if gold}
  <div class="strip" role="status" aria-label={t('gold.label')}>
    <div class="side order" class:mine={mine === 'order'}>
      <div class="row">{@render coin()}<b class="num">{compact(gold.order)}</b></div>
      {#if mine === 'order'}<div class="gap num">{signed(gap)}</div>{/if}
    </div>
    <div class="timer"></div>
    <div class="side chaos" class:mine={mine === 'chaos'}>
      <div class="row"><b class="num">{compact(gold.chaos)}</b>{@render coin()}</div>
      {#if mine === 'chaos'}<div class="gap num">{signed(gap)}</div>{/if}
    </div>
  </div>
{/if}

<style>
  :global(html.overlay),
  :global(html.overlay body) {
    background: transparent !important;
    overflow: hidden;
  }
  .strip {
    height: 100vh;
    display: grid;
    /* 170 | 298 | 170 of 638: the panels and the game's timer between them. */
    grid-template-columns: 26.65% 46.7% 26.65%;
    font-family: var(--font-score);
  }
  .side {
    --side: var(--assist);
    display: grid;
    align-content: center;
    gap: 2vh;
    padding: 0 12vh;
    background: color-mix(in srgb, var(--bg) var(--overlay-alpha), transparent);
    border: 2vh solid var(--seam-strong);
    border-top: 0;
    color: var(--side);
  }
  .order {
    justify-items: start;
    border-radius: 0 0 0 14vh;
  }
  .chaos {
    --side: var(--death);
    justify-items: end;
    border-radius: 0 0 14vh 0;
  }
  .side.mine {
    border-color: color-mix(in oklab, var(--side) 60%, var(--seam-strong));
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6vh;
  }
  .row b {
    font-size: 30vh;
    font-weight: 700;
    line-height: 1;
  }
  .coin {
    width: 26vh;
    height: 26vh;
  }
  .gap {
    font-size: 24vh;
    font-weight: 700;
    line-height: 1;
  }
</style>
