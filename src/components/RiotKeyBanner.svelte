<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app, go } from '../lib/state.svelte';

  /** Pages that show other players' ranked stats, the data a key unlocks. */
  const onRankedPage = $derived(app.route.name === 'profile' || app.route.name === 'live');
  const missing = $derived(app.riotStatus === 'none' || app.riotStatus === 'invalid');
  const show = $derived(app.ready && onRankedPage && missing && app.riotKeyDismissed !== app.riotStatus);
</script>

{#if show}
  <div class="banner" class:bad={app.riotStatus === 'invalid'} role="status">
    <span class="mark"><Icon name="warn" size={16} /></span>
    <div class="text">
      <b>{t(`riotBanner.${app.riotStatus}.title`)}</b>
      <span>{t(`riotBanner.${app.riotStatus}.body`)}</span>
    </div>
    <div class="actions">
      <button class="btn sm ghost" onclick={() => (app.riotKeyDismissed = app.riotStatus)}>{t('riotBanner.later')}</button>
      <button class="btn sm primary" onclick={() => go({ name: 'riotGuide' })}><Icon name="image" size={14} />{t('riotBanner.how')}</button>
    </div>
  </div>
{/if}

<style>
  .banner {
    --tone: var(--warn);
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 9px 16px 9px 14px;
    background: color-mix(in oklab, var(--tone) 12%, var(--bg-sunk));
    border-bottom: 1px solid color-mix(in oklab, var(--tone) 35%, var(--seam));
  }
  .banner.bad {
    --tone: var(--death);
  }
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    color: var(--tone);
    background: color-mix(in oklab, var(--tone) 18%, transparent);
  }
  .text {
    flex: 1 1 360px;
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
  /* When the window is too narrow, the buttons move under the text instead of squeezing it. */
  .actions {
    flex: none;
    display: flex;
    gap: 6px;
    margin-left: auto;
  }
</style>
