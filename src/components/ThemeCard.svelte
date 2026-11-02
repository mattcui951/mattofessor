<script lang="ts">
  import Icon from './Icon.svelte';
  import { resolve, type Theme } from '../lib/themes';

  let { theme, active, name, onpick }: { theme: Theme; active: boolean; name: string; onpick: () => void } = $props();
  const c = $derived(resolve(theme));
</script>

<button class="card" class:active onclick={onpick} aria-pressed={active} title={name}>
  <span class="mini" style="background:{c.bg};border-color:{c.seam};border-radius:{Math.min(theme.radius, 8)}px">
    <span class="bar" style="background:{c.bgSunk}"></span>
    <span class="body">
      <span class="side" style="background:{c.bgSunk};border-color:{c.seam}"></span>
      <span class="panel" style="background:{c.panel};border-color:{c.seam};border-radius:{Math.min(theme.radius, 6) / 2}px">
        <span class="line" style="background:{c.text};width:62%"></span>
        <span class="line" style="background:{c.text3};width:40%"></span>
        <span class="dots">
          {#each [c.you, c.death, c.objective, c.win, c.assist] as dot, i (i)}<i style="background:{dot}"></i>{/each}
        </span>
      </span>
    </span>
  </span>
  <span class="name">
    {#if active}<Icon name="check" size={14} />{/if}
    <span>{name}</span>
  </span>
</button>

<style>
  .card {
    display: grid;
    gap: 7px;
    padding: 6px;
    border-radius: var(--r-lg);
    border: 1px solid transparent;
    text-align: left;
    color: var(--text-2);
    min-width: 0;
  }
  .card:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .card.active {
    border-color: var(--you);
    background: color-mix(in oklab, var(--you) 8%, transparent);
    color: var(--text);
  }
  .mini {
    display: flex;
    flex-direction: column;
    height: 78px;
    border: 1px solid;
    overflow: hidden;
  }
  .bar {
    height: 11px;
    flex: none;
  }
  .body {
    flex: 1;
    display: flex;
    gap: 6px;
    padding: 0 6px 6px 0;
  }
  .side {
    width: 12px;
    border-right: 1px solid;
  }
  .panel {
    flex: 1;
    margin-top: 6px;
    border: 1px solid;
    padding: 6px;
    display: grid;
    gap: 5px;
    align-content: start;
  }
  .line {
    height: 4px;
    border-radius: 2px;
  }
  .dots {
    display: flex;
    gap: 4px;
    margin-top: 2px;
  }
  .dots i {
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12.5px;
    font-weight: 600;
    min-width: 0;
  }
  .name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .active .name {
    color: var(--you);
  }
</style>
