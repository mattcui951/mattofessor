<script lang="ts">
  /**
   * The in-game panel's item build for your champion in Hextech ARAM: the hand-written build when
   * there is one, else the statistics' starting items, boots, core items in order and other picks.
   */
  import { untrack } from 'svelte';
  import ItemIcon from './ItemIcon.svelte';
  import MayhemBuild from './MayhemBuild.svelte';
  import { api } from '../lib/api';
  import { OPGG_TTL, show } from '../lib/cache';
  import { content } from '../lib/content.svelte';
  import { itemName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { Build } from '../lib/types';

  let { championId }: { championId: number } = $props();

  let build = $state<Build | null>(null);
  let loaded = $state(false);
  // Only refetch when the id changes, not on every live update (see AugmentPicks).
  const champ = $derived(championId);

  $effect(() => {
    const id = champ;
    // New hand-written builds (the "content" event drops the kept builds): ask again, keeping
    // the line on screen meanwhile.
    content.data;
    if (untrack(() => build?.championId) !== id) {
      build = null;
      loaded = false;
    }
    // The same key as the Builds page, so either one draws the other's answer at once.
    show(
      `build:${id}::aram`,
      OPGG_TTL,
      () => api.build(id, '', 'aram'),
      (b) => {
        if (id === championId) {
          build = b;
          loaded = true;
        }
      },
      () => {
        if (id === championId) loaded = true;
      },
    );
  });

  const core = $derived(build ? build.items.slice(0, build.core) : []);
  const more = $derived(build ? build.items.slice(build.core, build.core + 4) : []);
  const empty = $derived(loaded && (!build || (!build.mine && !core.length && !build.boots.length)));
</script>

{#snippet item(id: number, size: number)}
  <span class="it" title={itemName(id)}><ItemIcon {id} {size} /></span>
{/snippet}

<section class="build">
  {#if build?.mine}
    <MayhemBuild build={build.mine} compact />
  {:else}
    <div class="line">
      <b class="lbl">{t('mayhem.short')}</b>
      {#if !loaded}
        <span class="wait" aria-busy="true" aria-label={t('common.loading')}>
          {#each Array(6) as _, i (i)}<span class="skel"></span>{/each}
        </span>
      {:else if empty}
        <span class="dim">{t('builds.none')}</span>
      {:else if build}
        {#if build.starters.length}
          <span class="grp">{#each build.starters[0].value as id, i (i)}{@render item(id, 22)}{/each}</span>
          <span class="sep"></span>
        {/if}
        {#if build.boots.length}
          {@render item(build.boots[0].value, 26)}
          <span class="sep"></span>
        {/if}
        <span class="grp core">
          {#each core as it, i (i)}
            {#if i}<span class="arrow">›</span>{/if}{@render item(it.value, 28)}
          {/each}
        </span>
        {#if more.length}
          <span class="sep"></span>
          <span class="dim">{t('builds.more')}</span>
          <span class="grp alt">{#each more as it, i (i)}{@render item(it.value, 22)}{/each}</span>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .build {
    padding: 8px 12px 10px;
    border-bottom: 1px solid var(--seam);
  }
  .line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    min-height: 28px;
  }
  .lbl {
    margin-right: 6px;
    font: 700 11.5px var(--font-score);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--you);
  }
  .grp {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .core {
    gap: 4px;
  }
  .it {
    display: inline-flex;
  }
  .arrow {
    color: var(--text-3);
    font-size: 12px;
  }
  .sep {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--seam-strong);
  }
  .alt {
    margin-left: 4px;
    opacity: 0.8;
  }
  .dim {
    color: var(--text-3);
    font-size: 12px;
  }
  .wait {
    display: inline-flex;
    gap: 6px;
  }
  .wait .skel {
    width: 26px;
    height: 26px;
  }
</style>
