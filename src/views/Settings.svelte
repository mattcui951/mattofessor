<script lang="ts">
  import { LOCALES, locale } from '../lib/locales';
  import Select from '../components/Select.svelte';
  import HotkeyInput from '../components/HotkeyInput.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { isMac } from '../lib/platform';
  import { appearance } from '../lib/appearance.svelte';
  import { app, checkUpdate, errorText, go, inGame, installUpdate, releaseUrl, toast, updateSettings } from '../lib/state.svelte';
  import { ago } from '../lib/format';
  import { hasKey } from '../lib/i18n.svelte';
  import Icon from '../components/Icon.svelte';
  import ThemeCard from '../components/ThemeCard.svelte';
  import FullscreenNotice from '../components/FullscreenNotice.svelte';
  import type { AutoAccept, OverlaySize, Settings } from '../lib/types';
  import { MIN_OVERLAY_OPACITY, OVERLAY_SIZES, overlayOpacity } from '../lib/appearance.svelte';
  import { content, loadContent } from '../lib/content.svelte';

  loadContent();

  const s = $derived(app.settings);
  /** The in-game windows' opacity setting (null: the theme's, `themeOpacity`). */
  const opacity = $derived(overlayOpacity(s?.overlayOpacity));
  const themeOpacity = $derived(appearance.theme.overlayOpacity);

  // The section list beside the page: one entry per titled section, the admin one only for admins.
  let pageEl = $state<HTMLElement>();
  let inView = $state('update');
  const toc = $derived(
    [
      { id: 'update', label: t('update.title') },
      { id: 'appearance', label: t('appearance.title') },
      { id: 'language', label: t('settings.language') },
      { id: 'hotkeys', label: t('settings.hotkeys') },
      { id: 'panel', label: t('panel.title') },
      { id: 'behavior', label: t('settings.behavior') },
      { id: 'admin', label: t('settings.adminTitle'), on: content.status.canEdit },
      { id: 'imports', label: t('settings.imports') },
      { id: 'capture', label: t('capture.title') },
      { id: 'riot', label: t('riot.title') },
    ].filter((x) => x.on !== false),
  );
  function jumpTo(id: string) {
    inView = id;
    pageEl?.querySelector('#s-' + id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
  // The section nearest the top of the page is the one in view.
  $effect(() => {
    const root = pageEl;
    if (!root || !s) return;
    const io = new IntersectionObserver(
      (entries) => {
        const top = entries.filter((e) => e.isIntersecting).sort((x, y) => x.boundingClientRect.top - y.boundingClientRect.top)[0];
        if (top) inView = top.target.id.slice(2);
      },
      { root, rootMargin: '0px 0px -70% 0px' },
    );
    root.querySelectorAll('section[id^="s-"]').forEach((el) => io.observe(el));
    return () => io.disconnect();
  });

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
  // Whether the system can read the augment cards (the game's language, the screen).
  let reader = $state<Awaited<ReturnType<typeof api.cardReaderStatus>> | null>(null);
  $effect(() => {
    app.connected;
    api.cardReaderStatus().then((r) => (reader = r), () => (reader = null));
  });
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

<div class="page" bind:this={pageEl}>
  {#if s}
    <!-- The sections, to jump to; the one in view is marked. -->
    <nav class="toc" aria-label={t('settings.title')}>
      {#each toc as x (x.id)}
        <button class:on={inView === x.id} aria-current={inView === x.id ? 'true' : undefined} onclick={() => jumpTo(x.id)}>{x.label}</button>
      {/each}
    </nav>
    <div class="col">
      <h2>{t('settings.title')}</h2>

      <section id="s-update">
        <h3>{t('update.title')}</h3>
        <div class="riot-status"><i style="background:{updateLine.color}"></i>{updateLine.text}<span class="ver">{t('update.version', app.version)}</span></div>
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

      <section id="s-appearance">
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

      <section id="s-language">
        <h3>{t('settings.language')}</h3>
        <div class="lang">
          <Select
            label={t('settings.language')}
            value={s.language}
            options={[{ value: 'auto', label: t('settings.language.auto') }, ...LOCALES.map((l) => ({ value: l.id, label: l.name }))]}
            onchange={(v) => updateSettings({ language: v })}
          />
        </div>
      </section>

      <section id="s-hotkeys">
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

      <section id="s-panel">
        <h3>{t('panel.title')}</h3>
        <div class="row">
          <span>{t('panel.size')}</span>
          <Segmented
            label={t('panel.size')}
            value={s.overlaySize ?? 'regular'}
            options={OVERLAY_SIZES.map((v) => ({ value: v, label: t(`panel.size.${v}`) }))}
            onchange={(v) => updateSettings({ overlaySize: v as OverlaySize })}
          />
        </div>
        <p class="note">{t('panel.sizeNote')}</p>
        <div class="row">
          <span>{t('panel.opacity')}</span>
          <span class="opacity">
            <input
              type="range"
              min={MIN_OVERLAY_OPACITY * 100}
              max="100"
              step="5"
              value={Math.round((opacity ?? themeOpacity) * 100)}
              aria-label={t('panel.opacity')}
              oninput={(e) => updateSettings({ overlayOpacity: Number(e.currentTarget.value) / 100 }, { debounce: true })}
            />
            <span class="num val">{Math.round((opacity ?? themeOpacity) * 100)}%</span>
            <button class="btn sm ghost" disabled={opacity === null} onclick={() => updateSettings({ overlayOpacity: 0 })}>{t('panel.opacityTheme')}</button>
          </span>
        </div>
        <p class="note">{t('panel.opacityNote')}</p>
        {@render toggleRow(t('panel.lockSpells'), 'lockSpells')}
        <div class="how-actions">
          <button class="btn sm" disabled={!s.spellsPos} onclick={() => updateSettings({ spellsPos: null })}>{t('panel.resetSpells')}</button>
        </div>
        {@render toggleRow(t('panel.goldDiff'), 'goldDiff')}
        <p class="note">{t('panel.goldDiffNote')}</p>
        <FullscreenNotice always />
      </section>

      <section id="s-behavior">
        <h3>{t('settings.behavior')}</h3>
        {@render toggleRow(t('settings.openInChampSelect'), 'openInChampSelect')}
        {@render toggleRow(t('settings.showDuringLoading'), 'showDuringLoading')}
        {@render toggleRow(t('settings.spellsOnStart'), 'spellsOnStart')}
        {@render toggleRow(t(isMac ? 'settings.startAtLogin' : 'settings.startWithWindows'), 'startWithWindows')}
        <div class="row">
          <span>{t('settings.autoAccept')}</span>
          <Segmented
            label={t('settings.autoAccept')}
            value={s.autoAccept ?? 'off'}
            options={(['off', 'hextech', 'all'] as const).map((v) => ({ value: v, label: t(`settings.autoAccept.${v}`) }))}
            onchange={(v) => updateSettings({ autoAccept: v as AutoAccept })}
          />
        </div>
        <p class="note">{t('settings.autoAcceptNote')}</p>
      </section>

      {#if content.status.canEdit}
        <section id="s-admin">
          <h3>{t('settings.adminTitle')}</h3>
          {@render toggleRow(t('settings.admin'), 'admin')}
          <p class="note">{t('settings.adminNote')}</p>
        </section>
      {/if}

      <section id="s-imports">
        <h3>{t('settings.imports')}</h3>
        {@render toggleRow(t('settings.autoRunes'), 'autoImportRunes')}
        {@render toggleRow(t('settings.autoItems'), 'autoImportItems')}
        <p class="note">{t('settings.autoNote')}</p>
      </section>

      <section id="s-capture">
        <h3>{t('capture.title')}</h3>
        {@render toggleRow(t('cards.toggle'), 'cardReader')}
        <p class="note">{t(isMac ? 'cards.noteMac' : 'cards.note')}</p>
        {#if s.cardReader && reader?.screenBlocked}
          <div class="ocr-warn">
            <b>{t('cards.noScreen')}</b>
            <span>{t('cards.screenHow')}</span>
            <button type="button" class="btn sm" onclick={() => api.allowScreenCapture()}>{t('cards.screenOpen')}</button>
          </div>
        {/if}
        {#if s.cardReader && reader?.missing}
          {@const game = locale(reader.gameLang ?? 'en')}
          <div class="ocr-warn">
            <b>{t('cards.noOcr', game.name)}</b>
            <span>{t('cards.ocrHow', game.name)}</span>
            <code>{`Add-WindowsCapability -Online -Name "Language.OCR~~~${game.tag}~0.0.1.0"`}</code>
          </div>
        {/if}
        {@render toggleRow(t('capture.toggle'), 'captureAugments')}
        <p class="note">{t('capture.note')}</p>
        <div class="how-actions">
          <span class="note">{t('capture.count', captures)}</span>
          <button type="button" class="btn sm" onclick={() => api.openCaptures()}><Icon name="folder" size={15} />{t('capture.open')}</button>
          <button type="button" class="btn sm ghost" disabled={captures === 0} onclick={async () => { await api.deleteCaptures(); refreshCaptures(); }}>{t('capture.delete')}</button>
        </div>
      </section>

      <section id="s-riot">
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
        <p class="note">{t(isMac ? 'riot.storedMac' : 'riot.stored')}</p>
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
  .ocr-warn {
    display: grid;
    gap: 4px;
    margin: 6px 0 10px;
    padding: 10px 12px;
    border-radius: var(--r-md);
    background: color-mix(in oklab, var(--warn) 10%, var(--panel));
    border: 1px solid color-mix(in oklab, var(--warn) 40%, var(--seam));
    font-size: 12.5px;
  }
  .ocr-warn span {
    color: var(--text-2);
  }
  .ocr-warn .btn {
    justify-self: start;
    margin-top: 4px;
  }
  .ocr-warn code {
    font-size: 11.5px;
    user-select: all;
    color: var(--text-2);
    background: var(--bg-sunk);
    border-radius: var(--r-sm);
    padding: 4px 6px;
    word-break: break-all;
  }
  .page {
    height: 100%;
    overflow-y: auto;
    display: grid;
    grid-template-columns: 190px minmax(0, 680px);
    align-items: start;
  }
  .toc {
    position: sticky;
    top: 0;
    display: grid;
    gap: 2px;
    padding: 64px 10px 20px 16px;
  }
  .toc button {
    text-align: left;
    padding: 6px 10px;
    border-radius: var(--r-md);
    color: var(--text-3);
    font-size: 12.5px;
    transition: background 120ms, color 120ms;
  }
  .toc button:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .toc button.on {
    color: var(--text);
    background: var(--panel-2);
    box-shadow: inset 2px 0 0 var(--you);
  }
  section[id] {
    scroll-margin-top: 16px;
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
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
  }
  .ap-text {
    flex: 1 1 220px;
    display: grid;
    gap: 10px;
    justify-items: start;
    color: var(--text-2);
    font-size: 13px;
  }
  .current {
    flex: 0 0 170px;
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
  .ver {
    margin-left: 4px;
    color: var(--text-3);
  }
  .lang {
    max-width: 260px;
  }
  .opacity {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .opacity input {
    width: 160px;
  }
  .val {
    min-width: 3em;
    text-align: right;
    color: var(--text-2);
    font-size: 12.5px;
  }
</style>
