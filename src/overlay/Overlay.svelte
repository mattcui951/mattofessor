<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import AramPicks from '../components/AramPicks.svelte';
  import AugmentPicks from '../components/AugmentPicks.svelte';
  import FullscreenNotice from '../components/FullscreenNotice.svelte';
  import Teams from '../components/Teams.svelte';
  import { api } from '../lib/api';
  import { queueName } from '../lib/format';
  import { isAramMode } from '../lib/meta.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app, updateSettings } from '../lib/state.svelte';
  import { OVERLAY_SIZES } from '../lib/appearance.svelte';

  const hasTeams = $derived(app.live.teams.some((tm) => tm.players.length > 0));
  const loadingScreen = $derived(!app.ingame || app.ingame.gameTime < 2);
  const me = $derived(app.live.teams.flatMap((tm) => tm.players).find((p) => p.me));
  const champSelect = $derived(app.live.phase === 'ChampSelect');
  const pending = $derived(app.live.teams.flatMap((tm) => tm.players).filter((p) => p.status === 'loading').length);

  // The panel's size, right in its header (the same setting as in Settings).
  const sizeIx = $derived(Math.max(0, OVERLAY_SIZES.indexOf(app.settings?.overlaySize ?? 'regular')));
  function resize(step: number) {
    const next = OVERLAY_SIZES[Math.min(OVERLAY_SIZES.length - 1, Math.max(0, sizeIx + step))];
    if (next !== OVERLAY_SIZES[sizeIx]) updateSettings({ overlaySize: next });
  }

  let header: HTMLElement;
  let content: HTMLElement;
  // Size the window to what is shown (expanding a player grows it). CSS pixels: the app applies
  // the panel's zoom.
  $effect(() => {
    const report = () => api.fitOverlay(header.offsetHeight + content.offsetHeight + 16);
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
    {#if champSelect}<span class="chip">{t('live.champSelect')}</span>
    {:else if loadingScreen && hasTeams}<span class="chip">{t('live.loadingScreen')}</span>{/if}
    {#if pending > 0}<span class="chip dim">{t('live.loading')} {pending}</span>{/if}
    <span class="grow"></span>
    <div class="size" role="group" aria-label={t('panel.size')}>
      <button class="a sm" aria-label={t('panel.smaller')} title={t('panel.smaller')} disabled={sizeIx === 0} onclick={() => resize(-1)}>A<sub>−</sub></button>
      <span class="sz">{t(`panel.size.${OVERLAY_SIZES[sizeIx]}`)}</span>
      <button class="a lg" aria-label={t('panel.bigger')} title={t('panel.bigger')} disabled={sizeIx === OVERLAY_SIZES.length - 1} onclick={() => resize(1)}>A<sub>+</sub></button>
    </div>
    <span class="hint">{t('overlay.hide')} <kbd>{app.settings?.hotkeyOverlay ?? 'Ctrl+Shift+M'}</kbd></span>
    <button class="x" aria-label={t('overlay.hide')} onclick={() => api.hideOverlay()}><Icon name="close" size={16} /></button>
  </header>
  <div class="body">
    <div bind:this={content}>
      <!-- Over a Full screen game the panel cannot show without the game stepping aside: say so first. -->
      <div class="fs"><FullscreenNotice /></div>
      {#if champSelect && isAramMode(app.live.mode)}
        <div class="picks"><AramPicks onpick={(id) => api.openBuild(id, 'aram')} /></div>
      {:else if hasTeams && app.live.mode === 'KIWI' && me && me.championId > 0}
        <AugmentPicks championId={me.championId} compact />
      {/if}
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
  .size {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    background: var(--bg-sunk);
  }
  .size .a {
    width: 28px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    border-radius: var(--r-sm);
    color: var(--text-2);
    font-family: var(--font-score);
    font-weight: 700;
    line-height: 1;
  }
  .size .a.sm {
    font-size: 11.5px;
  }
  .size .a.lg {
    font-size: 15px;
  }
  .size .a sub {
    font-size: 11px;
    line-height: 1;
    vertical-align: baseline;
    margin-left: 0.5px;
    position: relative;
    top: 0.12em;
  }
  .size .a:hover:not(:disabled) {
    color: var(--text);
    background: var(--panel-2);
  }
  .size .a:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .size .sz {
    min-width: 3.2em;
    text-align: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
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
  .fs :global(.notice) {
    margin: 8px 8px 0;
  }
  .picks {
    padding: 8px 8px 0;
  }
  .wait {
    padding: 40px;
    text-align: center;
    color: var(--text-3);
  }
</style>
