<script lang="ts">
  /** An item's icon with its hover card: stats and effects (Data Dragon), plus any numbers given. */
  import Img from './Img.svelte';
  import { itemIcon } from '../lib/dd.svelte';
  import { tip, type HoverInfo } from '../lib/hover.svelte';
  import { itemTip } from '../lib/tips.svelte';

  let { id, size = 32, stats = [] }: { id: number; size?: number; stats?: HoverInfo['stats'] } = $props();
</script>

<span class="ii" role="img" aria-label={itemTip(id).title} use:tip={() => itemTip(id, stats)}><Img src={itemIcon(id)} {size} /></span>

<style>
  .ii {
    display: inline-flex;
    flex: none;
    border-radius: var(--r-md);
  }
</style>
