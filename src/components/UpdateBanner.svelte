<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app } from '../lib/state.svelte';

  const u = $derived(app.update);
  const working = $derived(u.phase === 'downloading' || u.phase === 'installing');
  // A new version is announced by the card beside Settings (UpdateNotice); this shows the download.
  const show = $derived(Boolean(u.version) && working);
  const pct = $derived(u.progress == null ? null : Math.round(u.progress * 100));
</script>

{#if show && u.version}
  <div class="banner" role="status">
    <span class="mark"><Icon name="download" size={16} /></span>
    <div class="text">
      {#if u.phase === 'downloading'}
        <b>{pct == null ? t('update.downloadingNoPct', u.version) : t('update.downloading', u.version, pct)}</b>
        <span>{t('update.adminNote')}</span>
      {:else}
        <b>{t('update.installing', u.version)}</b>
        <span>{t('update.adminNote')}</span>
      {/if}
    </div>
    {#if u.phase === 'downloading'}
      <div class="bar" class:indeterminate={pct == null}><i style:width={pct == null ? null : `${pct}%`}></i></div>
    {/if}
  </div>
{/if}

<style>
  .banner {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 16px 9px 14px;
    background: color-mix(in oklab, var(--you) 12%, var(--bg-sunk));
    border-bottom: 1px solid color-mix(in oklab, var(--you) 35%, var(--seam));
    overflow: hidden;
  }
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    color: var(--you);
    background: color-mix(in oklab, var(--you) 18%, transparent);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 1px;
  }
  .text b {
    font-weight: 600;
    font-size: 13.5px;
  }
  .text span {
    color: var(--text-2);
    font-size: 12.5px;
  }
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: color-mix(in oklab, var(--you) 20%, transparent);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--you);
    transition: width 200ms linear;
  }
  .bar.indeterminate i {
    width: 30%;
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
