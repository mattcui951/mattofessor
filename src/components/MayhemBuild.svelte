<script lang="ts">
  /** A hand-written Hextech ARAM build: a card (Builds page) or one line (`compact`, in game). */
  import ItemIcon from './ItemIcon.svelte';
  import Img from './Img.svelte';
  import { itemIcon, itemName } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { MayhemBuild } from '../lib/types';

  let { build, compact = false }: { build: MayhemBuild; compact?: boolean } = $props();
</script>

{#if compact}
  <div class="line">
    <b class="lbl">{t('mayhem.short')}</b>
    {#each build.starters as id, i (i)}<ItemIcon id={id} size={22} />{/each}
    {#if build.starters.length && build.order.length}<span class="sep"></span>{/if}
    {#each build.order as id, i (i)}
      {#if i}<span class="arrow">›</span>{/if}<ItemIcon id={id} size={26} />
    {/each}
    {#if build.situational.length}
      <span class="sep"></span><span class="dim">{t('mayhem.situational')}</span>
      {#each build.situational as id, i (i)}<span class="alt"><ItemIcon id={id} size={22} /></span>{/each}
    {/if}
  </div>
{:else}
  <section class="card">
    <div class="head">
      <div class="label">{t('mayhem.title')}</div>
      {#if build.updated}<span class="dim">{t('mayhem.updated', build.updated)}</span>{/if}
    </div>
    {#if build.starters.length}
      <div class="sub">{t('mayhem.starters')}</div>
      <div class="row">{#each build.starters as id, i (i)}<ItemIcon id={id} size={36} />{/each}</div>
    {/if}
    {#if build.order.length}
      <div class="sub">{t('mayhem.order')}</div>
      <div class="order">
        {#each build.order as id, i (i)}
          <div class="it">
            <span class="step num">{i + 1}</span>
            <ItemIcon id={id} size={44} />
            <span class="iname">{itemName(id)}</span>
          </div>
        {/each}
      </div>
    {/if}
    {#if build.situational.length}
      <div class="sub">{t('mayhem.situational')}</div>
      <div class="row">
        {#each build.situational as id, i (i)}<span class="opt"><ItemIcon id={id} size={30} /><span>{itemName(id)}</span></span>{/each}
      </div>
    {/if}
    {#if build.note.trim()}<p class="note">{build.note}</p>{/if}
  </section>
{/if}

<style>
  .card {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 14px 16px;
    display: grid;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--you);
  }
  .sub {
    margin-top: 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-3);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .order {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
    gap: 8px;
  }
  .it {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 10px 6px 8px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    text-align: center;
    font-size: 12px;
  }
  .step {
    position: absolute;
    top: 4px;
    left: 6px;
    font-size: 11px;
    color: var(--text-3);
  }
  .iname {
    line-height: 1.25;
  }
  .opt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 10px 2px 2px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    font-size: 12.5px;
  }
  .note {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text-2);
    white-space: pre-wrap;
  }
  .dim {
    color: var(--text-3);
    font-size: 12px;
  }
  .line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
  }
  .lbl {
    margin-right: 6px;
    font: 700 11.5px var(--font-score);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--you);
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
    opacity: 0.75;
  }
</style>
