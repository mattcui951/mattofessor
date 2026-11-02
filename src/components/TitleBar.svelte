<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from './Icon.svelte';
  import { app } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  const win = getCurrentWindow();
  const phase = $derived(app.live.phase);
  const status = $derived(
    !app.connected
      ? { color: 'var(--death)', text: t('conn.off') }
      : phase === 'ChampSelect'
        ? { color: 'var(--objective)', text: t('live.champSelect') }
        : ['InProgress', 'GameStart', 'Reconnect'].includes(phase)
          ? { color: 'var(--you)', text: t('live.inGame') }
          : { color: 'var(--win)', text: t('conn.on') },
  );
</script>

<header class="bar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="10.5" cy="10.5" r="6" fill="none" stroke="var(--you)" stroke-width="2.6" />
      <path d="M15 15l5 5" stroke="var(--objective)" stroke-width="2.8" stroke-linecap="round" />
    </svg>
    <span class="name" data-tauri-drag-region>Mattofessor</span>
    <span class="status" data-tauri-drag-region><i style="background:{status.color}"></i>{status.text}</span>
  </div>
  <div class="tools">
    <button class="win-btn" aria-label="Minimize" onclick={() => win.minimize()}><Icon name="minimize" size={16} /></button>
    <button class="win-btn" aria-label="Maximize" onclick={() => win.toggleMaximize()}><Icon name="maximize" size={14} /></button>
    <button class="win-btn close" aria-label={t('common.close')} onclick={() => win.hide()}><Icon name="close" size={16} /></button>
  </div>
</header>

<style>
  .bar {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 14px;
    background: var(--bg-sunk);
    border-bottom: 1px solid var(--seam);
    flex: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }
  .name {
    font-family: var(--font-score);
    font-weight: 700;
    font-size: 14px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: 14px;
    font-size: 12.5px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .status i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .tools {
    display: flex;
    height: 100%;
  }
  .win-btn {
    width: 44px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .win-btn:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .win-btn.close:hover {
    background: var(--death);
    color: #fff;
  }
</style>
