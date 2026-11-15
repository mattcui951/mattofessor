<script lang="ts">
  import Logo from './Logo.svelte';
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
    <Logo />
    <span class="name" data-tauri-drag-region>Mattofessor</span>
    <span class="status" data-tauri-drag-region><i style="background:{status.color}"></i>{status.text}</span>
  </div>
  <div class="tools">
    <button class="win-btn" aria-label={t('common.minimize')} onclick={() => win.minimize()}><Icon name="minimize" size={16} /></button>
    <button class="win-btn" aria-label={t('common.maximize')} onclick={() => win.toggleMaximize()}><Icon name="maximize" size={14} /></button>
    <button class="win-btn close" aria-label={t('common.close')} onclick={() => win.hide()}><Icon name="close" size={16} /></button>
  </div>
</header>

<style>
  .bar {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 16px;
    background: var(--bg-sunk);
    border-bottom: 1px solid var(--seam);
    flex: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .name {
    font-family: var(--font-score);
    font-weight: 600;
    font-size: 14px;
    white-space: nowrap;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 16px;
    font-size: 13px;
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
