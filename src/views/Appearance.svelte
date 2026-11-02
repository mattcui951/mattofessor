<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import TagChip from '../components/TagChip.svelte';
  import ThemeCard from '../components/ThemeCard.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { appearance, applySettings } from '../lib/appearance.svelte';
  import { champIcon, tierColor, tierText } from '../lib/dd.svelte';
  import { hasKey, t } from '../lib/i18n.svelte';
  import { app, back, toast, updateSettings } from '../lib/state.svelte';
  import {
    ACCENT_KEYS,
    BUILT_IN,
    SCORE_FONTS,
    UI_FONTS,
    contrast,
    copyOf,
    parseHex,
    sanitize,
    type Theme,
    type ThemeColors,
  } from '../lib/themes';

  const active = $derived(appearance.theme);
  const editable = $derived(active.group === 'custom');

  const GROUPS = ['dark', 'light', 'contrast'] as const;
  const SURFACES: (keyof ThemeColors)[] = ['bg', 'bgSunk', 'panel', 'panel2', 'panel3', 'seam', 'seamStrong'];
  const TEXTS: (keyof ThemeColors)[] = ['text', 'text2', 'text3'];

  function themeName(th: Theme): string {
    return hasKey(`theme.${th.id}`) ? t(`theme.${th.id}`) : th.name;
  }

  function saveCustom(list: Theme[], select?: string, debounce = false) {
    updateSettings({ customThemes: list, ...(select ? { theme: select } : {}) }, { debounce });
  }

  function pick(id: string) {
    updateSettings({ theme: id });
  }

  function customize() {
    const copy = copyOf(active, t('appearance.copyName', themeName(active)), appearance.custom.map((x) => x.id));
    saveCustom([...appearance.custom, copy], copy.id);
  }

  /** Live edit of the active custom theme: applied at once, saved after a short pause. */
  function edit(patch: Partial<Theme>) {
    if (!editable) return;
    const next: Theme = { ...active, ...patch };
    saveCustom(appearance.custom.map((x) => (x.id === next.id ? next : x)), undefined, true);
  }

  function setColor(k: keyof ThemeColors, v: string) {
    const rgb = parseHex(v);
    if (!rgb) return;
    edit({ colors: { ...active.colors, [k]: v.startsWith('#') ? v.toLowerCase() : `#${v.toLowerCase()}` } });
  }

  let confirmDelete = $state(false);
  function remove() {
    const list = appearance.custom.filter((x) => x.id !== active.id);
    confirmDelete = false;
    saveCustom(list, 'arcade');
  }

  async function exportTheme() {
    const { id: _id, group: _group, ...rest } = active;
    try {
      await navigator.clipboard.writeText(JSON.stringify(rest, null, 2));
      toast(t('appearance.exported'), 'good');
    } catch {
      toast(t('appearance.exportFailed'), 'error');
    }
  }

  let importText = $state('');
  function importTheme() {
    let parsed: unknown;
    try {
      parsed = JSON.parse(importText);
    } catch {
      toast(t('appearance.importBad'), 'error');
      return;
    }
    const th = sanitize(parsed);
    if (!th) {
      toast(t('appearance.importBad'), 'error');
      return;
    }
    const fresh = copyOf(th, th.name, appearance.custom.map((x) => x.id));
    saveCustom([...appearance.custom, fresh], fresh.id);
    importText = '';
    toast(t('appearance.imported', fresh.name), 'good');
  }

  function fontChoice(value: string, list: { id: string }[]) {
    return list.some((f) => f.id === value) ? value : '__custom';
  }

  function ratio(fg: string): number {
    return contrast(fg, active.colors.bg);
  }

  function resetAll() {
    updateSettings({ theme: 'arcade', uiScale: 1 });
  }

  // Reapply on leaving in case a debounced save is still pending.
  $effect(() => () => {
    if (app.settings) applySettings(app.settings);
  });
</script>

{#snippet colorRow(k: keyof ThemeColors, min: number)}
  {@const value = active.colors[k]}
  {@const r = ratio(value)}
  <div class="crow">
    <label class="swatch" style="background:{value}">
      <input type="color" {value} disabled={!editable} oninput={(e) => setColor(k, e.currentTarget.value)} aria-label={t(`color.${k}`)} />
    </label>
    <span class="cname">{t(`color.${k}`)}</span>
    <input
      class="field hex num"
      value={value}
      disabled={!editable}
      spellcheck="false"
      maxlength="7"
      onchange={(e) => setColor(k, e.currentTarget.value)}
      aria-label={t(`color.${k}`)}
    />
    {#if min > 0}
      <span class="ratio num" class:low={r < min} title={t('appearance.contrastHint', min)}>{r.toFixed(1)}:1</span>
    {:else}
      <span></span>
    {/if}
  </div>
{/snippet}

{#snippet fontPicker(label: string, key: 'fontUi' | 'fontScore', list: { id: string; label: string; bundled: boolean }[])}
  {@const choice = fontChoice(active[key], list)}
  <div class="frow">
    <span class="cname">{label}</span>
    <select
      class="field"
      disabled={!editable}
      value={choice}
      onchange={(e) => {
        const v = e.currentTarget.value;
        edit({ [key]: v === '__custom' ? '' : v });
      }}
    >
      {#each list as f (f.id)}<option value={f.id}>{f.label}</option>{/each}
      <option value="__custom">{t('appearance.fontCustom')}</option>
    </select>
    {#if choice === '__custom'}
      <input
        class="field"
        disabled={!editable}
        value={active[key]}
        placeholder={t('appearance.fontCustomHint')}
        spellcheck="false"
        onchange={(e) => edit({ [key]: e.currentTarget.value })}
      />
    {/if}
    <span class="fsample" style="font-family:{`"${active[key].replace(/["';{}]/g, '')}", var(--font-ui)`}">Aa 0123 Faker 中文</span>
  </div>
{/snippet}

<div class="page">
  <div class="col">
    <div class="top">
      <button class="btn ghost sm" onclick={back}><Icon name="chevron-left" size={15} />{t('nav.settings')}</button>
      <h2>{t('appearance.title')}</h2>
      <span class="grow"></span>
      <button class="btn ghost sm" onclick={resetAll}><Icon name="restore" size={15} />{t('appearance.reset')}</button>
    </div>

    <section>
      <h3>{t('appearance.themes')}</h3>
      {#each GROUPS as g (g)}
        <div class="glabel">{t(`appearance.group.${g}`)}</div>
        <div class="grid">
          {#each BUILT_IN.filter((x) => x.group === g) as th (th.id)}
            <ThemeCard theme={th} name={themeName(th)} active={active.id === th.id} onpick={() => pick(th.id)} />
          {/each}
        </div>
      {/each}
      <div class="glabel">{t('appearance.group.custom')}</div>
      <div class="grid">
        {#each appearance.custom as th (th.id)}
          <ThemeCard theme={th} name={th.name} active={active.id === th.id} onpick={() => pick(th.id)} />
        {/each}
        <button class="newcard" onclick={customize}>
          <Icon name="plus" size={20} />
          <span>{t('appearance.newFrom', themeName(active))}</span>
        </button>
      </div>
    </section>

    <section class="editor">
      <div class="ehead">
        <h3>{t('appearance.customize')}</h3>
        {#if editable}
          <input class="field name" value={active.name} maxlength="40" aria-label={t('appearance.name')} onchange={(e) => edit({ name: e.currentTarget.value.trim() || active.name })} />
          <span class="grow"></span>
          <button class="btn sm" onclick={exportTheme}><Icon name="export" size={15} />{t('appearance.export')}</button>
          {#if confirmDelete}
            <span class="dim">{t('appearance.deleteAsk')}</span>
            <button class="btn sm danger solid" onclick={remove}>{t('appearance.delete')}</button>
            <button class="btn sm ghost" onclick={() => (confirmDelete = false)}>{t('appearance.keep')}</button>
          {:else}
            <button class="btn sm danger" onclick={() => (confirmDelete = true)}><Icon name="trash" size={15} />{t('appearance.delete')}</button>
          {/if}
        {:else}
          <span class="grow"></span>
          <span class="dim">{t('appearance.builtInNote')}</span>
          <button class="btn sm primary" onclick={customize}><Icon name="edit" size={15} />{t('appearance.customizeCopy')}</button>
        {/if}
      </div>

      <div class="preview" aria-label={t('appearance.preview')}>
        <div class="pv-row">
          <Img src={champIcon(103)} size={36} />
          <div class="pv-who">
            <b>Faker</b> <span class="lvl num">812</span>
            <div class="tags">
              <TagChip tag={{ key: 'oneTrick', tone: 'info', arg: 103 }} size="sm" />
              <TagChip tag={{ key: 'winStreak', tone: 'pos', arg: 5 }} size="sm" />
              <TagChip tag={{ key: 'premade', tone: 'duo', arg: 1 }} size="sm" />
              <TagChip tag={{ key: 'diesALot', tone: 'neg' }} size="sm" />
              <TagChip tag={{ key: 'farmer', tone: '' }} size="sm" />
            </div>
          </div>
          <div class="pv-kda num"><span class="k">11</span> / <span class="d">2</span> / <span class="a">9</span></div>
          <div class="pv-rank num">
            <span style="color:{tierColor('CHALLENGER')}">{tierText({ tier: 'CHALLENGER', division: '' })}</span>
            <span style="color:{tierColor('DIAMOND')}">{tierText({ tier: 'DIAMOND', division: 'II' }, true)}</span>
            <span style="color:{tierColor('GOLD')}">{tierText({ tier: 'GOLD', division: 'I' }, true)}</span>
          </div>
        </div>
        <div class="pv-buttons">
          <button class="btn primary sm">{t('appearance.sample.primary')}</button>
          <button class="btn sm">{t('appearance.sample.button')}</button>
          <button class="btn ghost sm">{t('appearance.sample.ghost')}</button>
          <button class="btn danger sm">{t('appearance.sample.danger')}</button>
          <Toggle checked={true} label="sample" onchange={() => {}} />
          <span class="dim">{t('appearance.sample.muted')}</span>
        </div>
      </div>

      <fieldset class="block" disabled={!editable}>
        <div class="blabel">{t('appearance.surfaces')}</div>
        <div class="cgrid">{#each SURFACES as k (k)}{@render colorRow(k, 0)}{/each}</div>
        <div class="blabel">{t('appearance.textColors')}</div>
        <div class="cgrid">{#each TEXTS as k (k)}{@render colorRow(k, 4.5)}{/each}</div>
        <div class="blabel">{t('appearance.accents')}</div>
        <div class="cgrid">{#each ACCENT_KEYS as k (k)}{@render colorRow(k, 3)}{/each}</div>

        <div class="blabel">{t('appearance.fonts')}</div>
        {@render fontPicker(t('appearance.fontUi'), 'fontUi', UI_FONTS)}
        {@render fontPicker(t('appearance.fontScore'), 'fontScore', SCORE_FONTS)}

        <div class="blabel">{t('appearance.shape')}</div>
        <div class="srow">
          <span class="cname">{t('appearance.radius')}</span>
          <input type="range" min="0" max="14" step="1" value={active.radius} oninput={(e) => edit({ radius: Number(e.currentTarget.value) })} />
          <span class="num val">{active.radius}px</span>
        </div>
        <div class="srow">
          <span class="cname">{t('appearance.overlayOpacity')}</span>
          <input type="range" min="60" max="100" step="1" value={Math.round(active.overlayOpacity * 100)} oninput={(e) => edit({ overlayOpacity: Number(e.currentTarget.value) / 100 })} />
          <span class="num val">{Math.round(active.overlayOpacity * 100)}%</span>
        </div>
        <div class="trow">
          <span>{t('appearance.dark')}</span>
          <Toggle checked={active.dark} disabled={!editable} label={t('appearance.dark')} onchange={(v) => edit({ dark: v })} />
        </div>
        <div class="trow">
          <span>{t('appearance.readable')}<small>{t('appearance.readableHint')}</small></span>
          <Toggle checked={active.readable} disabled={!editable} label={t('appearance.readable')} onchange={(v) => edit({ readable: v })} />
        </div>
      </fieldset>
    </section>

    <section>
      <h3>{t('appearance.textSize')}</h3>
      <div class="srow">
        <span class="cname">{t('appearance.textSizeHint')}</span>
        <input type="range" min="80" max="140" step="5" value={Math.round(appearance.scale * 100)} oninput={(e) => updateSettings({ uiScale: Number(e.currentTarget.value) / 100 }, { debounce: true })} />
        <span class="num val">{Math.round(appearance.scale * 100)}%</span>
      </div>
    </section>

    <section>
      <h3>{t('appearance.import')}</h3>
      <p class="note">{t('appearance.importHint')}</p>
      <textarea class="field area" rows="4" bind:value={importText} spellcheck="false" placeholder={'{ "name": "My theme", "colors": { "bg": "#232136", ... } }'}></textarea>
      <button class="btn sm" disabled={!importText.trim()} onclick={importTheme}><Icon name="download" size={15} />{t('appearance.importButton')}</button>
    </section>
  </div>
</div>

<style>
  .page {
    height: 100%;
    overflow-y: auto;
  }
  .col {
    max-width: 980px;
    padding: 18px 28px 48px;
    display: grid;
    gap: 26px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .grow {
    flex: 1;
  }
  h2 {
    font-size: 22px;
  }
  section {
    display: grid;
    gap: 10px;
  }
  h3 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
  }
  .glabel,
  .blabel {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-2);
    margin-top: 6px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }
  .newcard {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 6px;
    min-height: 112px;
    border: 1px dashed var(--seam-strong);
    border-radius: var(--r-lg);
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 600;
    padding: 8px;
    text-align: center;
  }
  .newcard:hover {
    color: var(--you);
    border-color: var(--you);
  }
  .editor {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 16px 18px;
  }
  .ehead {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .field.name {
    width: 240px;
  }
  .dim {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .preview {
    background: var(--bg);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 12px 14px;
    display: grid;
    gap: 12px;
  }
  .pv-row {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) auto auto;
    gap: 14px;
    align-items: center;
  }
  .pv-who b {
    font-weight: 600;
  }
  .lvl {
    font-size: 11px;
    color: var(--text-3);
    background: var(--panel-3);
    border-radius: var(--r-sm);
    padding: 0 5px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }
  .pv-kda {
    font-size: 16px;
  }
  .k {
    color: var(--you);
  }
  .d {
    color: var(--death);
  }
  .a {
    color: var(--assist);
  }
  .pv-rank {
    display: grid;
    text-align: right;
    font-size: 12.5px;
  }
  .pv-buttons {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .block {
    border: 0;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
    min-width: 0;
  }
  .block:disabled {
    opacity: 0.55;
  }
  .cgrid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 6px 18px;
  }
  .crow {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) 92px 52px;
    gap: 8px;
    align-items: center;
  }
  .swatch {
    position: relative;
    width: 28px;
    height: 28px;
    border-radius: var(--r-md);
    border: 1px solid var(--seam-strong);
    cursor: pointer;
    overflow: hidden;
  }
  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .cname {
    font-size: 13px;
    color: var(--text-2);
  }
  .hex {
    height: 28px;
    font-size: 12.5px;
  }
  .ratio {
    font-size: 11.5px;
    color: var(--win);
    text-align: right;
  }
  .ratio.low {
    color: var(--warn);
  }
  .frow {
    display: grid;
    grid-template-columns: 170px 220px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
  }
  .frow input {
    grid-column: 2;
  }
  .fsample {
    grid-column: 3;
    grid-row: 1;
    font-size: 15px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
  }
  .srow {
    display: grid;
    grid-template-columns: 170px minmax(0, 320px) 56px;
    gap: 12px;
    align-items: center;
  }
  input[type='range'] {
    accent-color: var(--you);
  }
  .val {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .trow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    max-width: 560px;
    padding: 4px 0;
    font-size: 13px;
  }
  .trow small {
    display: block;
    color: var(--text-3);
    font-size: 12px;
  }
  .note {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .area {
    height: auto;
    padding: 8px 10px;
    font-family: Consolas, ui-monospace, monospace;
    font-size: 12px;
    resize: vertical;
    max-width: 640px;
  }
  section > .btn {
    justify-self: start;
  }
</style>
