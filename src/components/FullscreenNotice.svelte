<script lang="ts">
  /**
   * Shown while League is set to Full screen: no window can show over a full-screen game, so the
   * in-game panel, spell timers and card win rates stay behind it. Offers the switch to Borderless.
   * `always` also shows the current mode when it is fine (Settings).
   */
  import { listen } from '@tauri-apps/api/event';
  import Icon from './Icon.svelte';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { app, errorText, inGame, toast } from '../lib/state.svelte';

  let { always = false }: { always?: boolean } = $props();

  let mode = $state<string | null>(null);
  let busy = $state(false);
  /** The panel was asked for during a Full screen game and kept closed. */
  let tried = $state(false);

  $effect(() => {
    api.fullscreenTried().then(
      (v) => (tried = v),
      () => {},
    );
    const off = listen<boolean>('fullscreen-tried', (e) => (tried = e.payload));
    return () => {
      off.then((f) => f());
    };
  });

  // Read again when the client connects or the game flow moves (champ select, game start).
  $effect(() => {
    app.connected;
    app.live.phase;
    api.leagueWindowMode().then(
      (m) => (mode = m),
      () => (mode = null),
    );
  });

  /** Hidden until the app restarts; a kept-closed panel is forgotten too (not told again). */
  function close() {
    app.fullscreenNoticeClosed = true;
    if (tried) {
      tried = false;
      api.dismissFullscreenTried().catch(() => {});
    }
  }

  async function fix() {
    busy = true;
    try {
      await api.setBorderless();
      mode = 'borderless';
      tried = false;
      toast(inGame() ? t('display.doneNextGame') : t('display.done'), 'good');
    } catch (e) {
      toast(`${t('display.failed')} ${errorText(e)}`, 'error');
    } finally {
      busy = false;
    }
  }
</script>

{#if mode === 'fullscreen' && !app.fullscreenNoticeClosed}
  <div class="notice" class:tried role="status">
    <span class="mark"><Icon name="warn" size={16} /></span>
    <div class="text">
      {#if tried}
        <b>{t('display.triedTitle')}</b>
        <span>{t('display.triedBody', app.settings?.hotkeyOverlay ?? 'Ctrl+Shift+M')}</span>
      {:else}
        <b>{t('display.title')}</b>
        <span>{t('display.body')}</span>
      {/if}
      <span class="how">{t('display.manual')}</span>
    </div>
    <button class="btn sm primary" disabled={busy || !app.connected} title={app.connected ? '' : t('conn.off')} onclick={fix}>{t('display.fix')}</button>
    <button class="x" aria-label={t('common.close')} title={t('common.close')} onclick={close}><Icon name="close" size={14} /></button>
  </div>
{:else if always && mode}
  <p class="note">{t('display.current', t(`display.mode.${mode}`))}</p>
{/if}

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--r-lg);
    background: color-mix(in oklab, var(--warn) 10%, var(--panel));
    border: 1px solid color-mix(in oklab, var(--warn) 40%, var(--seam));
    margin-bottom: 12px;
  }
  .notice.tried {
    background: color-mix(in oklab, var(--warn) 16%, var(--panel));
    border-color: var(--warn);
  }
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    color: var(--warn);
    background: color-mix(in oklab, var(--warn) 18%, transparent);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .text b {
    font-weight: 600;
    font-size: 13.5px;
  }
  .text span {
    color: var(--text-2);
    font-size: 12.5px;
  }
  .text .how {
    color: var(--text-3);
  }
  .x {
    flex: none;
    align-self: flex-start;
    width: 26px;
    height: 26px;
    margin: -4px -6px 0 -4px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--text-3);
  }
  .x:hover {
    color: var(--text);
    background: var(--panel-2);
  }
  .note {
    color: var(--text-3);
    font-size: 12.5px;
    margin: 6px 0 0;
  }
</style>
