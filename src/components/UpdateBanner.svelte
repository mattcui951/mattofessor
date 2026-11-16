<script lang="ts">
  /** The update downloading and installing (a new version is announced by UpdateNotice). */
  import Callout from './Callout.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app } from '../lib/state.svelte';

  const u = $derived(app.update);
  const show = $derived(Boolean(u.version) && (u.phase === 'downloading' || u.phase === 'installing'));
  const pct = $derived(u.progress == null ? null : Math.round(u.progress * 100));
  const title = $derived.by(() => {
    const v = u.version ?? '';
    if (u.phase === 'installing') return t('update.installing', v);
    return pct == null ? t('update.downloadingNoPct', v) : t('update.downloading', v, pct);
  });
</script>

{#if show}
  <Callout banner icon="download" {title}>
    <span>{t('update.adminNote')}</span>
    {#snippet extra()}
      {#if u.phase === 'downloading'}
        <div class="bar" class:indeterminate={pct == null}><i style:width={pct == null ? null : `${pct}%`}></i></div>
      {/if}
    {/snippet}
  </Callout>
{/if}

<style>
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
