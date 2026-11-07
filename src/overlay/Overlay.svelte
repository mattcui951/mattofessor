<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import AugmentPicks from '../components/AugmentPicks.svelte';
  import Teams from '../components/Teams.svelte';
  import { api } from '../lib/api';
  import { queueName } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import { appearance } from '../lib/appearance.svelte';
  import { app } from '../lib/state.svelte';

  const hasTeams = $derived(app.live.teams.some((tm) => tm.players.length > 0));
  const loadingScreen = $derived(!app.ingame || app.ingame.gameTime < 2);
  const me = $derived(app.live.teams.flatMap((tm) => tm.players).find((p) => p.me));
  const pending = $derived(app.live.teams.flatMap((tm) => tm.players).filter((p) => p.status === 'loading').length);

  let header: HTMLElement;
  let content: HTMLElement;
  // Size the window to what is shown (expanding a player grows it).
  $effect(() => {
    const report = () => api.fitOverlay((header.offsetHeight + content.offsetHeight + 16) * appearance.scale);
    const ro = new ResizeObserver(report);
    ro.observe(content);
    report();
    return () => ro.disconnect();
  });
</script>

<div class="panel">
  <header bind:this={header}>
    <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="10.5" cy="10.5" r="6" fill="none" stroke="var(--you)" stroke-width="2.6" />
      <path d="M15 15l5 5" stroke="var(--objective)" stroke-width="2.8" stroke-linecap="round" />
    </svg>
    <span class="brand">Mattofessor</span>
    {#if hasTeams}<span class="queue">{queueName(app.live.queueId)}</span>{/if}
    {#if loadingScreen && hasTeams}<span class="chip">{t('live.loadingScreen')}</span>{/if}
    {#if pending > 0}<span class="chip dim">{t('live.loading')} {pending}</span>{/if}
    <span class="grow"></span>
    <span class="hint">{t('overlay.hide')} <kbd>{app.settings?.hotkeyOverlay ?? 'Ctrl+Shift+M'}</kbd></span>
    <button class="x" aria-label={t('overlay.hide')} onclick={() => api.hideOverlay()}><Icon name="close" size={16} /></button>
  </header>
  <div class="body">
    <div bind:this={content}>
      {#if hasTeams && app.live.mode === 'KIWI' && me && me.championId > 0}<AugmentPicks championId={me.championId} compact />{/if}
      {#if hasTeams}
        <Teams live={app.live} onprofile={(puuid) => api.openProfile(puuid)} />
      {:else}
        <p class="wait">{app.connected ? t('overlay.waiting') : t('conn.off')}</p>
      {/if}
    </div>
  </div>
</div>

<style>
  :global(html.overlay),
  :global(html.overlay body) {
    background: transparent !important;
  }
  .panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--bg) var(--overlay-alpha), transparent);
    border: 1px solid var(--seam-strong);
    border-radius: var(--r-lg);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 6px 0 14px;
    border-bottom: 1px solid var(--seam);
    flex: none;
  }
  .brand {
    font-family: var(--font-score);
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    font-size: 13.5px;
  }
  .queue {
    font-family: var(--font-score);
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
  }
  .chip {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--objective);
    background: color-mix(in oklab, var(--objective) 13%, transparent);
    border-radius: var(--r-sm);
    padding: 1px 7px;
  }
  .chip.dim {
    color: var(--text-2);
    background: var(--panel-3);
  }
  .grow {
    flex: 1;
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  kbd {
    font-family: var(--font-score);
    font-size: 11px;
    border: 1px solid var(--seam-strong);
    border-bottom-width: 2px;
    border-radius: var(--r-sm);
    padding: 0 5px;
    color: var(--text-2);
    background: var(--bg-sunk);
  }
  .x {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: var(--r-md);
    color: var(--text-2);
  }
  .x:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 4px 4px 10px;
  }
  .wait {
    padding: 40px;
    text-align: center;
    color: var(--text-3);
  }
</style>
