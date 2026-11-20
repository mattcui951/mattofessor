<script lang="ts">
  /** An augment's icon in a ring of its rarity color; its name shows on hover. */
  import Img from './Img.svelte';
  import { hasKey, t } from '../lib/i18n.svelte';
  import { augment } from '../lib/meta.svelte';

  let { id, size = 20 }: { id: number; size?: number } = $props();

  const info = $derived(augment(id));
  const title = $derived.by(() => {
    if (!info) return `#${id}`;
    const r = `aug.${info.rarity}`;
    return info.rarity && hasKey(r) ? `${info.name} · ${t(r)}` : info.name;
  });
</script>

<span class="aug {info?.rarity ?? ''}" {title}>
  <Img src={info?.icon ?? null} {size} alt={info?.name ?? ''} />
</span>

<style>
  .aug {
    display: inline-flex;
    flex: none;
    border-radius: var(--r-md);
    box-shadow: 0 0 0 1.5px var(--seam);
  }
  .aug :global(.img) {
    background: var(--bg-sunk);
  }
  .silver {
    box-shadow: 0 0 0 1.5px var(--rar-silver);
  }
  .gold {
    box-shadow: 0 0 0 1.5px var(--rar-gold);
  }
  /* The rainbow ring: the gradient behind the icon, showing 1.5px around it. */
  .prismatic {
    position: relative;
    isolation: isolate;
    box-shadow: none;
  }
  .prismatic::before {
    content: '';
    position: absolute;
    inset: -1.5px;
    z-index: -1;
    border-radius: calc(var(--r-md) + 1.5px);
    background: var(--rar-prismatic-grad);
  }
</style>
