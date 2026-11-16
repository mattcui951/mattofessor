<script lang="ts">
  /** A new version is ready: a small card beside the Settings button (the rail shows a dot). */
  import { openUrl } from '@tauri-apps/plugin-opener';
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app, inGame, installUpdate, releaseUrl } from '../lib/state.svelte';

  const u = $derived(app.update);
  const show = $derived(u.phase === 'available' && Boolean(u.version) && app.updateDismissed !== u.version);
</script>

{#if show && u.version}
  <div class="notice" role="status">
    <button class="x icon-btn sm" aria-label={t('common.close')} title={t('common.close')} onclick={() => (app.updateDismissed = u.version)}><Icon name="close" size={14} /></button>
    <div class="top">
      <span class="mark"><Icon name="download" size={15} /></span>
      <b>{t('update.available', u.version)}</b>
    </div>
    <p>{inGame() ? t('update.afterGame') : t('update.current', app.version)}</p>
    <div class="actions">
      <button class="btn sm primary" disabled={inGame()} onclick={installUpdate}>{t('update.now')}</button>
      <button class="btn sm ghost" onclick={() => openUrl(releaseUrl(u.version!))}>{t('update.whatsNew')}</button>
    </div>
  </div>
{/if}

<style>
  .notice {
    position: fixed;
    left: 64px;
    bottom: 12px;
    z-index: 50;
    width: 272px;
    display: grid;
    gap: 6px;
    padding: 12px 12px 12px 14px;
    background: var(--panel);
    border: 1px solid color-mix(in oklab, var(--you) 40%, var(--seam));
    border-radius: var(--r-lg);
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.35);
  }
  /* Points at the Settings button. */
  .notice::before {
    content: '';
    position: absolute;
    left: -6px;
    bottom: 22px;
    width: 10px;
    height: 10px;
    background: var(--panel);
    border-left: 1px solid color-mix(in oklab, var(--you) 40%, var(--seam));
    border-bottom: 1px solid color-mix(in oklab, var(--you) 40%, var(--seam));
    transform: rotate(45deg);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 20px;
  }
  .top b {
    font-size: 13.5px;
    font-weight: 600;
  }
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    color: var(--you);
    background: color-mix(in oklab, var(--you) 18%, transparent);
  }
  p {
    margin: 0;
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.4;
  }
  .actions {
    display: flex;
    gap: 6px;
    margin-top: 2px;
  }
  .x {
    position: absolute;
    top: 7px;
    right: 7px;
  }
</style>
