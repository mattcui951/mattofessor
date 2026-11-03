<script lang="ts">
  import HotkeyInput from '../components/HotkeyInput.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { appearance } from '../lib/appearance.svelte';
  import { app, checkUpdate, errorText, go, inGame, installUpdate, releaseUrl, toast, updateSettings } from '../lib/state.svelte';
  import { ago } from '../lib/format';
  import { hasKey } from '../lib/i18n.svelte';
  import Icon from '../components/Icon.svelte';
  import ThemeCard from '../components/ThemeCard.svelte';
  import type { Settings } from '../lib/types';

  const s = $derived(app.settings);

  const u = $derived(app.update);
  const updateLine = $derived.by(() => {
    if (u.phase === 'checking') return { color: 'var(--objective)', text: t('update.checking') };
    if (u.phase === 'downloading') {
      const v = u.version ?? '';
      const text = u.progress == null ? t('update.downloadingNoPct', v) : t('update.downloading', v, Math.round(u.progress * 100));
      return { color: 'var(--you)', text };
    }
    if (u.phase === 'installing') return { color: 'var(--you)', text: t('update.installing', u.version ?? '') };
    if (u.version) return { color: 'var(--you)', text: t('update.available', u.version) };
    if (u.phase === 'error') return { color: 'var(--warn)', text: t('update.failed') };
    if (u.checkedAt) return { color: 'var(--win)', text: t('update.upToDate', ago(u.checkedAt)) };
    return { color: 'var(--text-3)', text: t('update.notChecked') };
  });

  let riotKey = $state('');
  let savingKey = $state(false);
  const riotColor = $derived(
    { ok: 'var(--win)', invalid: 'var(--death)', unreachable: 'var(--warn)', unchecked: 'var(--objective)', none: 'var(--text-3)' }[app.riotStatus],
  );

  let captures = $state(0);
  async function refreshCaptures() {
    try {
      captures = (await api.captureInfo()).count;
    } catch {
      /* preview */
    }
  }
  $effect(() => {
    refreshCaptures();
  });

  async function saveKey(key: string) {
    savingKey = true;
    try {
      app.riotStatus = await api.setRiotKey(key);
      riotKey = '';
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      savingKey = false;
    }
  }
</script>

{#snippet toggleRow(label: string, key: keyof Settings)}
  <div class="row">
    <span>{label}</span>
    <Toggle checked={Boolean(s?.[key])} {label} onchange={(v) => updateSettings({ [key]: v })} />
  </div>
{/snippet}

<div class="page">
  {#if s}
    <div class="col">
      <h2>{t('settings.title')}</h2>

      <section>
        <h3>{t('appearance.title')}</h3>
        <div class="appearance">
          <div class="current">
            <ThemeCard
              theme={appearance.theme}
              name={hasKey(`theme.${appearance.theme.id}`) ? t(`theme.${appearance.theme.id}`) : appearance.theme.name}
              active={false}
              onpick={() => go({ name: 'appearance' })}
            />
          </div>
          <div class="ap-text">
            <p>{t('appearance.summary')}</p>
            <button class="btn primary" onclick={() => go({ name: 'appearance' })}><Icon name="layout" size={16} />{t('appearance.open')}</button>
          </div>
        </div>
      </section>

      <section>
        <h3>{t('settings.language')}</h3>
        <Segmented
          label={t('settings.language')}
          value={s.language}
          options={[
            { value: 'auto', label: t('settings.language.auto') },
            { value: 'en', label: 'English' },
            { value: 'zh', label: '简体中文' },
          ]}
          onchange={(v) => updateSettings({ language: v })}
        />
      </section>

      <section>
        <h3>{t('settings.hotkeys')}</h3>
        <div class="row">
          <span>{t('settings.hotkeyOverlay')}</span>
          <HotkeyInput value={s.hotkeyOverlay} label={t('settings.hotkeyOverlay')} onchange={(v) => updateSettings({ hotkeyOverlay: v })} />
        </div>
        <div class="row">
          <span>{t('settings.hotkeySpells')}</span>
          <HotkeyInput value={s.hotkeySpells} label={t('settings.hotkeySpells')} onchange={(v) => updateSettings({ hotkeySpells: v })} />
        </div>
        <p class="note">{t('settings.hotkeyNote')}</p>
      </section>

      <section>
        <h3>{t('settings.behavior')}</h3>
        {@render toggleRow(t('settings.openInChampSelect'), 'openInChampSelect')}
        {@render toggleRow(t('settings.showDuringLoading'), 'showDuringLoading')}
        {@render toggleRow(t('settings.spellsOnStart'), 'spellsOnStart')}
        {@render toggleRow(t('settings.startWithWindows'), 'startWithWindows')}
      </section>

      <section>
        <h3>{t('settings.imports')}</h3>
        {@render toggleRow(t('settings.autoRunes'), 'autoImportRunes')}
        {@render toggleRow(t('settings.autoItems'), 'autoImportItems')}
        <p class="note">{t('settings.autoNote')}</p>
      </section>

      <section>
        <h3>{t('capture.title')}</h3>
        {@render toggleRow(t('capture.toggle'), 'captureAugments')}
        <p class="note">{t('capture.note')}</p>
        <div class="how-actions">
          <span class="note">{t('capture.count', captures)}</span>
          <button type="button" class="btn sm" onclick={() => api.openCaptures()}><Icon name="folder" size={15} />{t('capture.open')}</button>
          <button type="button" class="btn sm ghost" disabled={captures === 0} onclick={async () => { await api.deleteCaptures(); refreshCaptures(); }}>{t('capture.delete')}</button>
        </div>
      </section>

      <section>
        <h3>{t('riot.title')}</h3>
        <p class="note">{t('riot.why')}</p>
        <div class="riot-status"><i style="background:{riotColor}"></i>{t(`riot.status.${app.riotStatus}`)}</div>
        <form class="key-row" onsubmit={(e) => { e.preventDefault(); if (riotKey.trim()) saveKey(riotKey); }}>
          <input class="field" type="password" bind:value={riotKey} placeholder={t('riot.placeholder')} autocomplete="off" spellcheck="false" aria-label={t('riot.title')} />
          <button class="btn primary" disabled={savingKey || !riotKey.trim()}>{savingKey ? t('riot.saving') : t('riot.save')}</button>
          {#if app.riotStatus !== 'none'}<button type="button" class="btn ghost" disabled={savingKey} onclick={() => saveKey('')}>{t('riot.remove')}</button>{/if}
        </form>
        <div class="how">
          <div class="how-title">{t('riot.how')}</div>
          <ol>
            <li>{t('riot.step1')}</li>
            <li>{t('riot.step2')}</li>
            <li>{t('riot.step3')}</li>
          </ol>
          <div class="how-actions">
            <button type="button" class="btn sm primary" onclick={() => go({ name: 'riotGuide' })}><Icon name="image" size={15} />{t('guide.open')}</button>
            <button type="button" class="btn sm" onclick={() => openUrl('https://developer.riotgames.com/')}>{t('riot.open')}</button>
          </div>
        </div>
        <p class="note">{t('riot.stored')}</p>
      </section>

      <section>
        <h3>{t('update.title')}</h3>
        <div class="riot-status"><i style="background:{updateLine.color}"></i>{updateLine.text}</div>
        {#if u.error && u.phase !== 'checking'}<p class="note">{u.error}</p>{/if}
        <div class="how-actions">
          {#if u.version}
            <button class="btn primary" disabled={inGame() || u.phase === 'downloading' || u.phase === 'installing'} onclick={installUpdate}>
              <Icon name="download" size={16} />{t('update.now')}
            </button>
            <button class="btn" onclick={() => openUrl(releaseUrl(u.version!))}>{t('update.whatsNew')}</button>
          {:else}
            <button class="btn" disabled={u.phase === 'checking'} onclick={checkUpdate}><Icon name="refresh" size={16} />{t('update.check')}</button>
          {/if}
        </div>
        {@render toggleRow(t('update.auto'), 'checkUpdates')}
        <p class="note">{t('update.note')}</p>
      </section>

      <section>
        <p class="note">{t('settings.about', app.version)}</p>
        <p class="note">{t('settings.legal')}</p>
        <button class="btn danger" onclick={() => api.quitApp()}>{t('settings.quit')}</button>
      </section>
    </div>
  {/if}
</div>

<style>
  .page {
    height: 100%;
    overflow-y: auto;
  }
  .col {
    max-width: 680px;
    padding: 22px 28px 40px;
    display: grid;
    gap: 22px;
  }
  h2 {
    font-size: 22px;
  }
  section {
    display: grid;
    gap: 8px;
    justify-items: start;
  }
  section > :global(.seg) {
    justify-self: stretch;
    max-width: 420px;
  }
  h3 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
  }
  .row {
    justify-self: stretch;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 36px;
    padding: 4px 0;
    border-bottom: 1px solid var(--seam);
  }
  .note {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .appearance {
    justify-self: stretch;
    display: grid;
    grid-template-columns: 170px minmax(0, 1fr);
    gap: 16px;
    align-items: center;
  }
  .ap-text {
    display: grid;
    gap: 10px;
    justify-items: start;
    color: var(--text-2);
    font-size: 13px;
  }
  .riot-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-2);
  }
  .riot-status i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .key-row {
    justify-self: stretch;
    display: flex;
    gap: 8px;
  }
  .key-row .field {
    flex: 1;
    font-family: Consolas, ui-monospace, monospace;
  }
  .how {
    justify-self: stretch;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 12px 14px;
    display: grid;
    gap: 8px;
    justify-items: start;
  }
  .how-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .how-title {
    font-weight: 600;
    font-size: 13px;
  }
  .how ol {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 5px;
    color: var(--text-2);
    font-size: 12.5px;
  }
</style>
