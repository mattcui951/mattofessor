<script lang="ts">
  /** One line over a few labelled steps (the champion's win rate patch by patch). */
  import { tip, type HoverInfo } from '../lib/hover.svelte';
  import { pct1 } from '../lib/meta.svelte';

  let { points, current }: { points: { label: string; value: number; hover: HoverInfo }[]; current?: string } = $props();

  let width = $state(320);
  const H = 150;
  const PAD = { l: 40, r: 18, t: 22, b: 24 };

  const scale = $derived.by(() => {
    const vals = points.map((p) => p.value);
    const lo = Math.floor((Math.min(...vals) - 0.005) * 100) / 100;
    const hi = Math.ceil((Math.max(...vals) + 0.005) * 100) / 100;
    const y = (v: number) => PAD.t + (1 - (v - lo) / (hi - lo || 1)) * (H - PAD.t - PAD.b);
    const step = hi - lo > 0.06 ? 0.02 : 0.01;
    const ticks: number[] = [];
    for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-9; v += step) ticks.push(Math.round(v * 1000) / 1000);
    return { y, ticks };
  });
  const x = (i: number) => PAD.l + (points.length < 2 ? (width - PAD.l - PAD.r) / 2 : (i / (points.length - 1)) * (width - PAD.l - PAD.r));
  const path = $derived(points.map((p, i) => `${i ? 'L' : 'M'}${x(i)},${scale.y(p.value)}`).join(' '));
  let active = $state<number | null>(null);
</script>

<div class="chart" bind:clientWidth={width}>
  <svg viewBox="0 0 {width} {H}" width={width} height={H} role="img" aria-label={points.map((p) => `${p.label} ${pct1(p.value)}%`).join(', ')}>
    {#each scale.ticks as v (v)}
      <line class="grid" x1={PAD.l} x2={width - PAD.r} y1={scale.y(v)} y2={scale.y(v)} />
      <text class="axis" x={PAD.l - 6} y={scale.y(v) + 3.5} text-anchor="end">{Math.round(v * 100)}%</text>
    {/each}
    {#if active !== null}<line class="cross" x1={x(active)} x2={x(active)} y1={PAD.t - 8} y2={H - PAD.b} />{/if}
    <path class="line" d={path} />
    {#each points as p, i (p.label)}
      {@const px = x(i)}
      {@const py = scale.y(p.value)}
      <g
        class="pt"
        class:now={p.label === current}
        use:tip={() => p.hover}
        role="listitem"
        aria-label="{p.label} {pct1(p.value)}%"
        onmouseenter={() => (active = i)}
        onmouseleave={() => (active = null)}
        onfocus={() => (active = i)}
        onblur={() => (active = null)}
      >
        <rect class="hit" x={px - 24} y={PAD.t - 16} width={48} height={H - PAD.t - PAD.b + 16} />
        <circle cx={px} cy={py} r={p.label === current ? 5 : 4} />
        {#if i === points.length - 1 || p.label === current || active === i}<text class="val" x={px} y={py - 10} text-anchor="middle">{pct1(p.value)}%</text>{/if}
        <text class="axis lab" x={px} y={H - 7} text-anchor="middle">{p.label}</text>
      </g>
    {/each}
  </svg>
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
  }
  .cross {
    stroke: var(--seam-strong);
  }
  .axis {
    fill: var(--text-3);
    font-size: 10.5px;
  }
  .lab {
    font-size: 11.5px;
    fill: var(--text-2);
  }
  .line {
    fill: none;
    stroke: var(--you);
    stroke-width: 2;
    stroke-linejoin: round;
  }
  .hit {
    fill: transparent;
  }
  .pt {
    outline: none;
  }
  circle {
    fill: var(--you);
    stroke: var(--panel);
    stroke-width: 2;
  }
  .pt.now .lab {
    fill: var(--text);
    font-weight: 700;
  }
  .val {
    fill: var(--text);
    font: 600 11.5px var(--font-ui);
  }
</style>
