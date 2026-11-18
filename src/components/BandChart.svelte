<script lang="ts">
  /**
   * Bars from a shared baseline: the champion's win rate in each skill bracket (or any few
   * labelled values), with all players' rate as a dashed reference line.
   */
  import { tip, type HoverInfo } from '../lib/hover.svelte';
  import { pct1 } from '../lib/meta.svelte';

  let {
    bars,
    reference,
    referenceLabel,
  }: { bars: { label: string; value: number; hover: HoverInfo }[]; reference: number | null; referenceLabel: string } = $props();

  let width = $state(320);
  const H = 150;
  const PAD = { l: 40, r: 10, t: 22, b: 24 };

  const scale = $derived.by(() => {
    const vals = [...bars.map((b) => b.value), ...(reference !== null ? [reference] : [])];
    const lo = Math.floor((Math.min(...vals) - 0.01) * 100) / 100;
    const hi = Math.ceil((Math.max(...vals) + 0.005) * 100) / 100;
    const y = (v: number) => PAD.t + (1 - (v - lo) / (hi - lo || 1)) * (H - PAD.t - PAD.b);
    const step = hi - lo > 0.06 ? 0.02 : 0.01;
    const ticks: number[] = [];
    for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-9; v += step) ticks.push(Math.round(v * 1000) / 1000);
    return { lo, y, ticks };
  });
  const slot = $derived((width - PAD.l - PAD.r) / Math.max(1, bars.length));
  const barW = $derived(Math.min(44, slot * 0.5));
</script>

<div class="chart" bind:clientWidth={width}>
  <svg viewBox="0 0 {width} {H}" width={width} height={H} role="img" aria-label={bars.map((b) => `${b.label} ${pct1(b.value)}%`).join(', ')}>
    {#each scale.ticks as v (v)}
      <line class="grid" x1={PAD.l} x2={width - PAD.r} y1={scale.y(v)} y2={scale.y(v)} />
      <text class="axis" x={PAD.l - 6} y={scale.y(v) + 3.5} text-anchor="end">{Math.round(v * 100)}%</text>
    {/each}
    {#each bars as b, i (b.label)}
      {@const x = PAD.l + slot * i + slot / 2}
      {@const top = scale.y(b.value)}
      {@const base = H - PAD.b}
      <g class="bar" use:tip={() => b.hover} role="listitem" aria-label="{b.label} {pct1(b.value)}%">
        <rect class="hit" x={x - slot / 2} y={PAD.t - 16} width={slot} height={base - PAD.t + 16} />
        <path class="fill" d="M{x - barW / 2},{base} V{top + 4} q0,-4 4,-4 h{barW - 8} q4,0 4,4 V{base} Z" />
        <text class="val" x={x} y={top - 6} text-anchor="middle">{pct1(b.value)}%</text>
        <text class="axis lab" x={x} y={H - 7} text-anchor="middle">{b.label}</text>
      </g>
    {/each}
    {#if reference !== null}
      <line class="ref" x1={PAD.l} x2={width - PAD.r} y1={scale.y(reference)} y2={scale.y(reference)} />
    {/if}
  </svg>
  {#if reference !== null}<p class="cap"><span class="dash"></span>{referenceLabel} {pct1(reference)}%</p>{/if}
</div>

<style>
  .chart {
    width: 100%;
    min-width: 0;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--seam);
    stroke-width: 1;
  }
  .axis {
    fill: var(--text-3);
    font-size: 10.5px;
  }
  .lab {
    font-size: 11.5px;
    fill: var(--text-2);
  }
  .hit {
    fill: transparent;
  }
  .fill {
    fill: var(--you);
    opacity: 0.85;
  }
  .bar {
    outline: none;
  }
  .bar:hover .fill,
  .bar:focus-visible .fill {
    opacity: 1;
  }
  .val {
    fill: var(--text);
    font: 600 11.5px var(--font-ui);
  }
  .ref {
    stroke: var(--text-2);
    stroke-width: 1.5;
    stroke-dasharray: 4 3;
  }
  .cap {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 4px 0 0;
    color: var(--text-3);
    font-size: 11.5px;
  }
  .dash {
    width: 16px;
    border-top: 1.5px dashed var(--text-2);
  }
</style>
