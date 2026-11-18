<script lang="ts">
  /** Tooltip markup from the game (`richtext.ts`), as colored text. */
  import { parseRich } from '../lib/richtext';

  let { text }: { text: string } = $props();
  const lines = $derived(parseRich(text));
</script>

<div class="rich">
  {#each lines as line, i (i)}
    {#if line.length}
      <p>{#each line as s, j (j)}<span class="{s.tone} {s.dim ? 'dim' : ''}">{s.text}</span>{/each}</p>
    {:else}
      <div class="gap"></div>
    {/if}
  {/each}
</div>

<style>
  .rich {
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  p {
    margin: 0;
  }
  .gap {
    height: 6px;
  }
  .title {
    color: var(--text);
    font-weight: 700;
  }
  .stat {
    color: var(--text);
  }
  .key {
    color: var(--text);
    font-weight: 600;
  }
  .magic,
  .ap {
    color: var(--assist);
  }
  .physical,
  .ad {
    color: var(--warn);
  }
  .true {
    color: var(--text);
    font-weight: 700;
  }
  .heal {
    color: var(--win);
  }
  .defense,
  .gold {
    color: var(--objective);
  }
  .speed,
  .mana {
    color: var(--you);
  }
  .crit {
    color: var(--death);
  }
  .dim {
    color: var(--text-3);
    font-size: 11.5px;
  }
</style>
