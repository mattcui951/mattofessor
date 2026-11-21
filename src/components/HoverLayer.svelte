<script lang="ts">
  /** The floating card of `hover.svelte.ts`: below its anchor, or above when there is no room. */
  import Img from './Img.svelte';
  import RichText from './RichText.svelte';
  import { hover } from '../lib/hover.svelte';

  let w = $state(0);
  let h = $state(0);

  const pos = $derived.by(() => {
    const r = hover.rect;
    if (!r) return null;
    const gap = 8;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const left = Math.min(Math.max(8, r.left + r.width / 2 - w / 2), vw - w - 8);
    const below = r.bottom + gap + h <= vh - 8 || r.top - gap - h < 8;
    return { left, top: below ? r.bottom + gap : r.top - gap - h };
  });
</script>

{#if hover.info && pos}
  {@const i = hover.info}
  <div class="hc" class:wide={Boolean(i.body)} role="tooltip" bind:offsetWidth={w} bind:offsetHeight={h} style="left:{pos.left}px;top:{pos.top}px">
    <div class="top">
      {#if i.icon !== undefined}<Img src={i.icon} size={34} round={i.round} plate={i.plate} />{/if}
      <div class="ttl">
        <b>{i.title}</b>
        {#if i.sub}<small><span class={i.subTone ? `rar-${i.subTone}` : ''}>{i.sub}</span></small>{/if}
      </div>
    </div>
    {#if i.body}<div class="body"><RichText text={i.body} /></div>{/if}
    {#if i.stats.length}
      <dl>
        {#each i.stats as s (s.label)}
          <dt>{s.label}</dt>
          <dd class="num {s.tone ?? ''}">{s.value}</dd>
        {/each}
      </dl>
    {/if}
    {#if i.note}<p>{i.note}</p>{/if}
  </div>
{/if}

<style>
  .hc {
    position: fixed;
    z-index: 1000;
    width: max-content;
    min-width: 190px;
    max-width: 280px;
    padding: 10px 12px;
    border-radius: var(--r-lg);
    background: var(--panel-2);
    border: 1px solid var(--seam-strong);
    box-shadow: var(--shadow-pop);
    pointer-events: none;
    font-size: 12.5px;
  }
  .hc.wide {
    width: 330px;
    max-width: calc(100vw - 16px);
  }
  .body {
    margin-top: 9px;
    padding-top: 8px;
    border-top: 1px solid var(--seam);
  }
  .top {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .ttl {
    display: grid;
    line-height: 1.3;
    min-width: 0;
  }
  .ttl b {
    font-size: 13.5px;
  }
  .ttl small {
    color: var(--text-3);
    font: 600 11px var(--font-score);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  dl {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 3px 16px;
    margin: 9px 0 0;
    padding-top: 8px;
    border-top: 1px solid var(--seam);
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    text-align: right;
    font-weight: 600;
  }
  dd.up {
    color: var(--win);
  }
  dd.down {
    color: var(--death);
  }
  p {
    margin: 8px 0 0;
    color: var(--text-3);
    font-size: 11.5px;
    line-height: 1.4;
  }
</style>
