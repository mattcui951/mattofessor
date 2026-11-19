<script lang="ts">
  /**
   * One slim line on the pages that show other players' ranked stats (the data a key unlocks).
   * "Later" hides it for a week for that key status (kept in this window's storage; a new status,
   * such as a key that stopped working, shows again).
   */
  import Callout from './Callout.svelte';
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app, go } from '../lib/state.svelte';

  const WEEK_MS = 7 * 24 * 3600_000;
  const KEY = 'riot-banner-later';

  function hiddenUntil(status: string): number {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, number>;
      return v[status] ?? 0;
    } catch {
      return 0;
    }
  }

  function later() {
    const status = app.riotStatus;
    app.riotKeyDismissed = status;
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, number>;
      v[status] = Date.now() + WEEK_MS;
      localStorage.setItem(KEY, JSON.stringify(v));
    } catch {
      /* this session only */
    }
  }

  const onRankedPage = $derived(app.route.name === 'profile' || app.route.name === 'live');
  const missing = $derived(app.riotStatus === 'none' || app.riotStatus === 'invalid');
  const show = $derived(
    app.ready && onRankedPage && missing && app.riotKeyDismissed !== app.riotStatus && hiddenUntil(app.riotStatus) < Date.now(),
  );
</script>

{#if show}
  <Callout
    banner
    slim
    tone={app.riotStatus === 'invalid' ? 'bad' : 'warn'}
    title={t(`riotBanner.${app.riotStatus}.title`)}
    hint={t(`riotBanner.${app.riotStatus}.body`)}
  >
    <span>{t(`riotBanner.${app.riotStatus}.body`)}</span>
    {#snippet actions()}
      <button class="btn sm ghost" onclick={later}>{t('riotBanner.later')}</button>
      <button class="btn sm primary" onclick={() => go({ name: 'riotGuide' })}><Icon name="image" size={14} />{t('riotBanner.how')}</button>
    {/snippet}
  </Callout>
{/if}
