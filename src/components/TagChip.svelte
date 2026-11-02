<script lang="ts">
  import { tagHint, tagText } from '../lib/format';
  import type { Tag } from '../lib/types';
  let { tag, size = 'md' }: { tag: Tag; size?: 'sm' | 'md' } = $props();
</script>

<span class="tag {tag.tone} {size}" title={tagHint(tag)}>{tagText(tag)}</span>

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: var(--r-sm);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    background: var(--panel-3);
    color: var(--text-2);
  }
  .tag.sm {
    height: 18px;
    padding: 0 6px;
    font-size: 11px;
  }
  .pos {
    color: var(--win);
    background: color-mix(in oklab, var(--win) 13%, transparent);
  }
  .neg {
    color: var(--death);
    background: color-mix(in oklab, var(--death) 14%, transparent);
  }
  .info {
    color: var(--objective);
    background: color-mix(in oklab, var(--objective) 13%, transparent);
  }
  .duo {
    color: var(--assist);
    background: color-mix(in oklab, var(--assist) 16%, transparent);
  }
</style>
