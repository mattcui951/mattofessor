<script lang="ts">
  /**
   * Shown while League is set to Full screen, where the in-game windows stay closed (nothing can
   * show over such a game); louder after the player tried. Offers the switch to Borderless.
   * `always` also shows the current mode when it is fine (Settings).
   */
  import { listen } from '@tauri-apps/api/event';
  import Callout from './Callout.svelte';
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

  // Read again when the client connects or the game flow moves (champ select, game start). Through
  // a derived: the live state is replaced on every live update, its phase much less often.
  const phase = $derived(app.live.phase);
  $effect(() => {
    app.connected;
    phase;
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
  <Callout tone="warn" strong={tried} title={tried ? t('display.triedTitle') : t('display.title')} onclose={close}>
    <span>{tried ? t('display.triedBody', app.settings?.hotkeyOverlay ?? 'Ctrl+Shift+M') : t('display.body')}</span>
    <span class="dim">{t('display.manual')}</span>
    {#snippet actions()}
      <button class="btn sm primary" disabled={busy || !app.connected} title={app.connected ? '' : t('conn.off')} onclick={fix}>{t('display.fix')}</button>
    {/snippet}
  </Callout>
{:else if always && mode}
  <p class="note">{t('display.current', t(`display.mode.${mode}`))}</p>
{/if}

<style>
  .note {
    color: var(--text-3);
    font-size: 12.5px;
    margin: 6px 0 0;
  }
</style>
