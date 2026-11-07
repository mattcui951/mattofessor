<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import AramPicks from '../components/AramPicks.svelte';
  import AugmentPicks from '../components/AugmentPicks.svelte';
  import Teams from '../components/Teams.svelte';
  import { api } from '../lib/api';
  import { champName } from '../lib/dd.svelte';
  import { positionRole, queueName } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import { isAramMode } from '../lib/meta.svelte';
  import { app, errorText, go, toast } from '../lib/state.svelte';

  const phase = $derived(app.live.phase);
  const inGame = $derived(['InProgress', 'GameStart', 'Reconnect'].includes(phase));
  const champSelect = $derived(phase === 'ChampSelect');
  const active = $derived(inGame || champSelect);
  const me = $derived(app.live.teams.flatMap((tm) => tm.players).find((p) => p.me));
  const hasTeams = $derived(app.live.teams.some((tm) => tm.players.length > 0));
  let importing = $state(false);
  const hextech = $derived(app.live.mode === 'KIWI');
  const buildMode = $derived(isAramMode(app.live.mode) ? 'aram' : 'rift');

  async function importBuild() {
    if (!me || me.championId <= 0) return;
    importing = true;
    try {
      // Hextech ARAM has no runes.
      const r = await api.importBuild(me.championId, positionRole(me.position), buildMode, app.live.mode !== 'KIWI', true);
      toast(t('live.imported', [r.runePage, r.itemSet].filter(Boolean).join(' + ')), 'good');
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      importing = false;
    }
  }
</script>

<div class="page">
  <header class="head">
    <div>
      <div class="label">{active ? (champSelect ? t('live.champSelect') : t('live.inGame')) : hasTeams ? t('live.lastGame') : t('live.title')}</div>
      <h2>{hasTeams ? queueName(app.live.queueId) : t('live.idle')}</h2>
    </div>
    <div class="actions">
      {#if champSelect && me && me.championId > 0}
        <button class="btn" onclick={importBuild} disabled={importing}><Icon name="download" size={16} />{t('live.importBuild')} · {champName(me.championId)}</button>
        <button class="btn ghost" onclick={() => go({ name: 'builds', championId: me.championId, role: positionRole(me.position), mode: buildMode })}>{t('nav.builds')}</button>
      {/if}
      {#if inGame}
        <span class="hint">{t('live.hotkey', app.settings?.hotkeyOverlay ?? 'Ctrl+Shift+M')}</span>
        <button class="btn primary" onclick={() => api.toggleOverlay()}>{t('live.showOverlay')}</button>
        <button class="btn" onclick={() => api.toggleSpells()}><Icon name="timer" size={16} />{t('spells.title')}</button>
      {/if}
    </div>
  </header>

  {#if !app.connected && !inGame}
    <div class="empty">
      <Icon name="warn" size={30} />
      <h3>{t('conn.off')}</h3>
      <p>{t('conn.offHint')}</p>
    </div>
  {:else if hasTeams}
    <div class="body" class:past={!active}>
      {#if champSelect && isAramMode(app.live.mode)}<AramPicks />{/if}
      {#if active && hextech && me && me.championId > 0}<AugmentPicks championId={me.championId} />{/if}
      <Teams live={app.live} onprofile={(puuid) => go({ name: 'profile', puuid })} />
      {#if champSelect && app.live.teams.some((tm) => !tm.ally && tm.players.every((p) => p.status === 'hidden'))}
        <p class="note">{t('live.hiddenHint')}</p>
      {/if}
    </div>
  {:else}
    <div class="empty">
      <Icon name="live" size={30} />
      <h3>{t('live.idle')}</h3>
      <p>{t('live.idleHint')}</p>
    </div>
  {/if}
</div>

<style>
  .page {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 16px;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--seam);
    flex-wrap: wrap;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--you);
  }
  h2 {
    font-size: 20px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .hint {
    color: var(--text-3);
    font-size: 12.5px;
    margin-right: 4px;
  }
  .body {
    overflow-y: auto;
    padding: 6px 10px 20px;
  }
  .body.past {
    opacity: 0.92;
  }
  .note {
    color: var(--text-3);
    font-size: 12.5px;
    padding: 4px 18px;
  }
</style>
