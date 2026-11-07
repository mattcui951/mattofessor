<script lang="ts">
  /**
   * Win rates above the augment cards in Hextech ARAM. The window spans the game's width just
   * above the cards and ignores the mouse; sizes are in vh so they follow the game's resolution.
   */
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { t } from '../lib/i18n.svelte';
  import { delta, pct1 } from '../lib/meta.svelte';
  import type { CardOffer } from '../lib/types';

  let offer = $state<CardOffer | null>(null);

  $effect(() => {
    invoke<CardOffer | null>('card_offer').then((o) => (offer = o ?? offer));
    const off = listen<CardOffer>('offer', (e) => (offer = e.payload));
    return () => {
      off.then((f) => f());
    };
  });

  /** Good, bad or about even for the champion. */
  const tone = (d: number) => (d > 0.01 ? 'good' : d < -0.01 ? 'bad' : 'even');
</script>

{#if offer}
  {#each offer.cards as c (c.id)}
    <div class="badge {c.stat ? tone(c.stat.delta) : 'none'}" class:best={c.best} style:left="{c.x * 100}%">
      {#if c.best}<span class="tag">{t('cards.best')}</span>{/if}
      {#if c.stat}
        <b class="num">{pct1(c.stat.winRate)}%</b>
        <span class="d num">{delta(c.stat.delta)}</span>
        {#if c.stat.lowSample}<span class="few">{t('cards.few')}</span>{/if}
      {:else}
        <span class="few">{t('cards.noData')}</span>
      {/if}
    </div>
  {/each}
{/if}

<style>
  :global(html.overlay),
  :global(html.overlay body) {
    background: transparent !important;
    overflow: hidden;
  }
  .badge {
    position: absolute;
    bottom: 4vh;
    transform: translateX(-50%);
    display: flex;
    align-items: baseline;
    gap: 9vh;
    padding: 10vh 22vh;
    border-radius: 14vh;
    background: color-mix(in srgb, var(--bg) 88%, transparent);
    border: 3vh solid var(--seam-strong);
    white-space: nowrap;
    font-family: var(--font-score);
  }
  b {
    font-size: 36vh;
    font-weight: 700;
    line-height: 1;
  }
  .d {
    font-size: 24vh;
    font-weight: 600;
  }
  .few {
    font-size: 19vh;
    color: var(--text-3);
  }
  .good {
    border-color: color-mix(in srgb, var(--win) 70%, transparent);
  }
  .good b,
  .good .d {
    color: var(--win);
  }
  .bad {
    border-color: color-mix(in srgb, var(--death) 70%, transparent);
  }
  .bad b,
  .bad .d {
    color: var(--death);
  }
  .even b {
    color: var(--text);
  }
  .even .d {
    color: var(--text-2);
  }
  .best {
    border-color: var(--win);
    box-shadow: 0 0 18vh color-mix(in srgb, var(--win) 55%, transparent);
  }
  .tag {
    align-self: center;
    font-size: 17vh;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--bg);
    background: var(--win);
    padding: 2vh 8vh;
    border-radius: 6vh;
  }
</style>
