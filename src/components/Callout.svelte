<script lang="ts">
  /**
   * A notice: an icon in a tinted circle, a title with lines under it, then buttons. `banner` spans
   * the page's width under the title bar; otherwise it is a card. Lines are <span>s in `children`;
   * `extra` is drawn last (the download's progress bar). `slim` (with `banner`) is one short line: the
   * title, then the lines cut to fit, all of them in the tooltip (`hint`).
   */
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    title,
    tone = 'info',
    icon = 'warn',
    banner = false,
    strong = false,
    slim = false,
    hint,
    children,
    actions,
    extra,
    onclose,
  }: {
    title: string;
    tone?: 'info' | 'warn' | 'bad';
    icon?: string;
    banner?: boolean;
    /** A louder card (something the player tried did not work). */
    strong?: boolean;
    /** One short line (a banner that repeats on several pages). */
    slim?: boolean;
    /** Tooltip: the whole text when `slim` cuts it. */
    hint?: string;
    children?: Snippet;
    actions?: Snippet;
    extra?: Snippet;
    onclose?: () => void;
  } = $props();
</script>

<div class="callout {tone}" class:banner class:strong class:slim role="status" title={hint}>
  <span class="mark"><Icon name={icon} size={slim ? 13 : 16} /></span>
  <div class="text">
    <b>{title}</b>
    {@render children?.()}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
  {#if onclose}
    <button class="x icon-btn sm" aria-label={t('common.close')} title={t('common.close')} onclick={onclose}><Icon name="close" size={14} /></button>
  {/if}
  {@render extra?.()}
</div>

<style>
  .callout {
    --tone: var(--you);
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 10px 14px;
    border-radius: var(--r-lg);
    background: color-mix(in oklab, var(--tone) 10%, var(--panel));
    border: 1px solid color-mix(in oklab, var(--tone) 40%, var(--seam));
  }
  .warn {
    --tone: var(--warn);
  }
  .bad {
    --tone: var(--death);
  }
  .strong {
    background: color-mix(in oklab, var(--tone) 16%, var(--panel));
    border-color: var(--tone);
  }
  .banner {
    flex: none;
    overflow: hidden;
    padding: 9px 16px 9px 14px;
    border: 0;
    border-radius: 0;
    background: color-mix(in oklab, var(--tone) 12%, var(--bg-sunk));
    border-bottom: 1px solid color-mix(in oklab, var(--tone) 35%, var(--seam));
  }
  .slim {
    flex-wrap: nowrap;
    padding: 5px 12px 5px 14px;
  }
  .slim .mark {
    width: 22px;
    height: 22px;
  }
  .slim .text {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex: 1 1 auto;
    white-space: nowrap;
    overflow: hidden;
  }
  .slim .text b {
    flex: none;
    font-size: 13px;
  }
  .slim .text :global(span) {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    color: var(--tone);
    background: color-mix(in oklab, var(--tone) 18%, transparent);
  }
  .text {
    flex: 1 1 360px;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .text b {
    font-weight: 600;
    font-size: 13.5px;
  }
  .text :global(span) {
    color: var(--text-2);
    font-size: 12.5px;
  }
  .text :global(.dim) {
    color: var(--text-3);
  }
  /* Too narrow a window: the buttons move under the text instead of squeezing it. */
  .actions {
    flex: none;
    display: flex;
    gap: 6px;
    margin-left: auto;
  }
  .x {
    flex: none;
    align-self: flex-start;
    margin: -4px -6px 0 -4px;
  }
</style>
