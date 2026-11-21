<script lang="ts">
  /**
   * A tier letter (S to D) on the app's tier colours (app.css `--tier-*`): champions by win rate
   * (`aramTier`), augments by how much they lift the champion (`augTier`).
   */
  import type { Tier } from '../lib/types';

  let { tier, size = 18, title = '' }: { tier: Tier; size?: number; title?: string } = $props();
</script>

<span class="grade t{tier}" style:--s="{size}px" {title} aria-label={title || tier}>{tier}</span>

<style>
  .grade {
    flex: none;
    width: var(--s);
    height: var(--s);
    display: inline-grid;
    place-items: center;
    border-radius: calc(var(--s) * 0.24);
    font: 700 calc(var(--s) * 0.62) / 1 var(--font-score);
    color: var(--bg);
    background: var(--text-3);
  }
  .tS {
    background: var(--tier-S);
  }
  .tA {
    background: var(--tier-A);
  }
  .tB {
    background: var(--tier-B);
  }
  .tC {
    background: var(--tier-C);
  }
  .tD {
    background: var(--tier-D);
  }
</style>
