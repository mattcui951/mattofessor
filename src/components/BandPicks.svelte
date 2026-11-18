<script lang="ts">
  /**
   * How often the champion's most taken augments are taken in each skill bracket: one row per
   * augment, a dot per bracket on a shared scale (lighter for casual, full for the top 5%).
   */
  import Img from './Img.svelte';
  import { tip } from '../lib/hover.svelte';
  import { t } from '../lib/i18n.svelte';
  import { augment, pct1 } from '../lib/meta.svelte';
  import type { AramBracket } from '../lib/types';

  let { bands }: { bands: { bracket: AramBracket; picks: [number, number][] }[] } = $props();

  const ids = $derived(bands[0]?.picks.map(([id]) => id) ?? []);
  const all = $derived(bands.flatMap((b) => b.picks.map(([, p]) => p)));
  /** The scale: whole 5% steps around the values (a dot plot needs no zero). */
  const lo = $derived(Math.max(0, Math.floor(Math.min(...all) * 20) / 20));
  const hi = $derived(Math.max(lo + 0.05, Math.ceil(Math.max(...all) * 20) / 20));
  const at = (p: number) => `${((p - lo) / (hi - lo)) * 100}%`;
  const of = (b: (typeof bands)[number], id: number) => b.picks.find(([x]) => x === id)?.[1] ?? 0;
</script>

<div class="bp">
  <div class="legend">
    {#each bands as b (b.bracket)}<span><i class="dot {b.bracket}"></i>{t(`bracket.${b.bracket}`)}</span>{/each}
  </div>
  {#each ids as id (id)}
    {@const info = augment(id)}
    {@const vals = bands.map((b) => of(b, id))}
    <div class="row">
      <span class="aug"><Img src={info?.icon ?? null} size={22} round /><span>{info?.name ?? `#${id}`}</span></span>
      <span class="track">
        <span class="span" style="left:{at(Math.min(...vals))};width:calc({at(Math.max(...vals))} - {at(Math.min(...vals))})"></span>
        {#each bands as b (b.bracket)}
          {@const v = of(b, id)}
          <span
            class="dot {b.bracket}"
            style="left:{at(v)}"
            role="img"
            aria-label="{info?.name} · {t(`bracket.${b.bracket}`)} {pct1(v)}%"
            use:tip={() => ({ icon: info?.icon ?? null, round: true, title: info?.name ?? `#${id}`, sub: t(`bracket.${b.bracket}`), stats: [{ label: t('hex.pick'), value: `${pct1(v)}%` }] })}
          ></span>
        {/each}
      </span>
    </div>
  {/each}
  <div class="row axis">
    <span></span>
    <span class="track ticks">
      {#each [0, 0.5, 1] as f (f)}<span style="left:{f * 100}%">{Math.round((lo + (hi - lo) * f) * 1000) / 10}%</span>{/each}
    </span>
  </div>
</div>

<style>
  .bp {
    display: grid;
    gap: 8px;
  }
  .legend {
    display: flex;
    gap: 14px;
    color: var(--text-2);
    font-size: 11.5px;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 118px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
  }
  .aug {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 12px;
  }
  .aug > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track {
    position: relative;
    height: 22px;
    margin-right: 8px;
    border-left: 1px solid var(--seam);
  }
  .track::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    border-top: 1px solid var(--seam);
  }
  .span {
    position: absolute;
    top: calc(50% - 1px);
    height: 2px;
    background: var(--seam-strong);
  }
  .dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--panel);
    outline: none;
  }
  .track .dot {
    position: absolute;
    top: calc(50% - 5px);
    margin-left: -5px;
    cursor: default;
  }
  .track .dot:hover,
  .track .dot:focus-visible {
    transform: scale(1.35);
  }
  .dot.casual {
    background: color-mix(in oklab, var(--you) 35%, var(--panel-3));
  }
  .dot.mid {
    background: color-mix(in oklab, var(--you) 68%, var(--panel-3));
  }
  .dot.high {
    background: var(--you);
  }
  .axis .track {
    border-left: 0;
    height: 14px;
  }
  .axis .track::before {
    display: none;
  }
  .ticks span {
    position: absolute;
    transform: translateX(-50%);
    color: var(--text-3);
    font-size: 10.5px;
  }
  .ticks span:first-child {
    transform: none;
  }
  .ticks span:last-child {
    transform: translateX(-100%);
  }
</style>
