<script lang="ts">
  import Callout from './Callout.svelte';
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app, go } from '../lib/state.svelte';

  /** Pages that show other players' ranked stats, the data a key unlocks. */
  const onRankedPage = $derived(app.route.name === 'profile' || app.route.name === 'live');
  const missing = $derived(app.riotStatus === 'none' || app.riotStatus === 'invalid');
  const show = $derived(app.ready && onRankedPage && missing && app.riotKeyDismissed !== app.riotStatus);
</script>

{#if show}
  <Callout banner tone={app.riotStatus === 'invalid' ? 'bad' : 'warn'} title={t(`riotBanner.${app.riotStatus}.title`)}>
    <span>{t(`riotBanner.${app.riotStatus}.body`)}</span>
    {#snippet actions()}
      <button class="btn sm ghost" onclick={() => (app.riotKeyDismissed = app.riotStatus)}>{t('riotBanner.later')}</button>
      <button class="btn sm primary" onclick={() => go({ name: 'riotGuide' })}><Icon name="image" size={14} />{t('riotBanner.how')}</button>
    {/snippet}
  </Callout>
{/if}
