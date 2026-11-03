<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, go } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  const live = $derived(['ChampSelect', 'InProgress', 'GameStart', 'Reconnect'].includes(app.live.phase));
</script>

<nav class="rail" aria-label={t('common.mainNav')}>
  <button class="item" class:on={app.route.name === 'profile'} aria-label={t('nav.profile')} title={t('nav.profile')} onclick={() => go({ name: 'profile' })}>
    <Icon name="profile" size={21} />
  </button>
  <button class="item" class:on={app.route.name === 'live'} aria-label={t('nav.live')} title={t('nav.live')} onclick={() => go({ name: 'live' })}>
    <Icon name="live" size={21} />
    {#if live}<span class="dot"></span>{/if}
  </button>
  <button class="item" class:on={app.route.name === 'builds'} aria-label={t('nav.builds')} title={t('nav.builds')} onclick={() => go({ name: 'builds' })}>
    <Icon name="builds" size={21} />
  </button>
  <div class="grow"></div>
  <button class="item" class:on={['settings', 'appearance', 'riotGuide'].includes(app.route.name)} aria-label={t('nav.settings')} title={t('nav.settings')} onclick={() => go({ name: 'settings' })}>
    <Icon name="settings" size={21} />
  </button>
</nav>

<style>
  .rail {
    width: 56px;
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 0;
    background: var(--bg-sunk);
    border-right: 1px solid var(--seam);
  }
  .grow {
    flex: 1;
  }
  .item {
    position: relative;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: var(--r-md);
    color: var(--text-3);
    transition: color 120ms, background 120ms;
  }
  .item:hover {
    color: var(--text);
    background: var(--panel-2);
  }
  .item.on {
    color: var(--you);
    background: var(--panel-2);
  }
  .item.on::before {
    content: '';
    position: absolute;
    left: -8px;
    top: 9px;
    bottom: 9px;
    width: 2px;
    background: var(--you);
  }
  .dot {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--death);
  }
</style>
