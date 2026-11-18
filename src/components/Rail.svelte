<script lang="ts">
  /** The main navigation: an icon and a short label per page; the full name is the tooltip. */
  import Icon from './Icon.svelte';
  import UpdateNotice from './UpdateNotice.svelte';
  import { app, go, type Route } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  /** A new version waits (until installed, even after its card is closed). */
  const update = $derived(app.update.phase === 'available' && Boolean(app.update.version));
  const live = $derived(['ChampSelect', 'InProgress', 'GameStart', 'Reconnect'].includes(app.live.phase));

  const PAGES: { route: Route['name']; icon: string; also?: Route['name'][] }[] = [
    { route: 'profile', icon: 'profile' },
    { route: 'live', icon: 'live' },
    { route: 'meta', icon: 'trophy' },
    { route: 'tiers', icon: 'rows' },
    { route: 'builds', icon: 'builds' },
    { route: 'guide', icon: 'book' },
    { route: 'patches', icon: 'news' },
  ];
</script>

{#snippet item(route: Route['name'], icon: string, also: Route['name'][] = [])}
  <button class="item" class:on={app.route.name === route || also.includes(app.route.name)} title={t(`nav.${route}`)} onclick={() => go({ name: route } as Route)}>
    <Icon name={icon} size={19} />
    <span class="label">{t(`rail.${route}`)}</span>
    {#if route === 'live' && live}<span class="dot"></span>{/if}
    {#if route === 'settings' && update}<span class="dot update" title={t('update.available', app.update.version ?? '')}></span>{/if}
  </button>
{/snippet}

<nav class="rail" aria-label={t('common.mainNav')}>
  {#each PAGES as p (p.route)}{@render item(p.route, p.icon)}{/each}
  <div class="grow"></div>
  {@render item('settings', 'settings', ['appearance', 'riotGuide'])}
</nav>
<UpdateNotice />

<style>
  .rail {
    width: 148px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 8px;
    background: var(--bg-sunk);
    border-right: 1px solid var(--seam);
  }
  .grow {
    flex: 1;
  }
  .item {
    position: relative;
    height: 38px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border-radius: var(--r-md);
    color: var(--text-3);
    font-size: 13.5px;
    font-weight: 600;
    text-align: left;
    transition: color 120ms, background 120ms;
  }
  .item :global(svg) {
    flex: none;
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    left: 24px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--death);
  }
  .dot.update {
    box-shadow: 0 0 0 2px var(--bg-sunk);
  }
</style>
