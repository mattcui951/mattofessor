<script lang="ts" module>
  // What the page shows, kept while the app runs: back to this page lands on the same tabs.
  let part = $state<'rift' | 'mayhem'>('rift');
  let picked = $state('');
</script>

<script lang="ts">
  /**
   * The latest patches' changes, Summoner's Rift and Hextech ARAM apart (refreshed data): Riot's
   * patch notes in the UI language (Tencent's for Simplified Chinese), never another language's.
   * Champions, items and augments are shown by their names in the UI language.
   */
  import Icon from '../components/Icon.svelte';
  import { day } from '../lib/format';
  import Img from '../components/Img.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { locale } from '../lib/locales';
  import { champIcon, champName, itemIcon, itemName } from '../lib/dd.svelte';
  import { gameData, loadGameData, noteBetter, noteMod, notesFor, noteVerdict, plain, bi, type NoteEntry } from '../lib/gamedata.svelte';
  import { hasKey, i18n, t } from '../lib/i18n.svelte';
  import { augment, loadAugments } from '../lib/meta.svelte';
  import { go } from '../lib/state.svelte';

  loadGameData();
  loadAugments();

  const notes = $derived(notesFor(gameData.data?.patchNotes));
  const patches = $derived(notes?.patches ?? []);
  const patch = $derived(patches.find((p) => p.patch === picked) ?? patches[0]);
  const sections = $derived(patch ? patch[part] : []);
  const headliners = $derived(gameData.data?.mayhem.headliners ?? []);

  /** A section or group title in the UI language when it is a known one. */
  const title = (s: string) => (hasKey(`patches.t.${plain(s)}`) ? t(`patches.t.${plain(s)}`) : s);

  function name(e: NoteEntry): string {
    if (e.championId) return champName(e.championId);
    if (e.itemId) return itemName(e.itemId);
    if (e.augmentId) return augment(e.augmentId)?.name ?? e.name;
    const band = headliners.find((h) => plain(h.name.en) === plain(e.name) || h.name.zh === e.name);
    return band ? bi(band.name) : e.name;
  }
  function icon(e: NoteEntry): string | null {
    if (e.championId) return champIcon(e.championId);
    if (e.itemId) return itemIcon(e.itemId);
    if (e.augmentId) return augment(e.augmentId)?.icon ?? null;
    return null;
  }
  const better = noteBetter;
  const verdict = noteVerdict;
  const colon = $derived(t('fmt.colon'));
  /** An ability heading ("R - Summon: Tibbers") with the ability's name in the UI language. */
  function partTitle(e: NoteEntry, title: string): string {
    if (title === 'Base Stats') return t('patches.baseStats');
    const m = /^(Passive|P|Q|W|E|R)\s*[-–]\s*(.+)$/.exec(title);
    const key = m?.[1] === 'Passive' ? 'P' : m?.[1];
    const n = key && e.championId ? gameData.data?.mayhem.abilityNames?.[String(e.championId)]?.[key] : undefined;
    return n ? `${key === 'P' ? t('patches.passive') : key} - ${bi(n)}` : title;
  }
  const attrLabel = (a: string) => (noteMod(a) ? t(`mods.${noteMod(a)}`) : a);
  function open(e: NoteEntry) {
    if (e.championId) go({ name: 'builds', championId: e.championId, mode: part === 'mayhem' ? 'aram' : 'rift' });
  }
  const date = $derived(patch?.date ? day(patch.date) : '');
</script>

<div class="page">
  <header class="head">
    <h2>{t('patches.title')}</h2>
    <Segmented
      label={t('meta.view')}
      value={part}
      options={[
        { value: 'rift', label: t('mode.rift') },
        { value: 'mayhem', label: t('mode.aram') },
      ]}
      onchange={(v) => (part = v as typeof part)}
    />
    {#if patches.length}
      <Segmented label={t('patches.patch')} value={patch?.patch ?? ''} options={patches.map((p) => ({ value: p.patch, label: p.patch }))} onchange={(v) => (picked = v)} />
    {/if}
    <span class="grow"></span>
    {#if patch}
      <span class="dim">{date}</span>
      <button class="btn sm" onclick={() => openUrl(patch.url)}><Icon name="open" size={14} />{t('patches.original')}</button>
    {/if}
  </header>

  <div class="scroll">
    {#if !gameData.data}
      <p class="dim">{t('common.loading')}</p>
    {:else if !notes}
      <!-- No notes in this language yet (English is Riot's, Chinese Tencent's): never another language's. -->
      <div class="none">
        <p class="dim">{t('notes.none')}</p>
        <button class="btn sm" onclick={() => openUrl(`https://www.leagueoflegends.com/${locale(i18n.lang).riot}/news/tags/patch-notes/`)}><Icon name="open" size={14} />{t('notes.official')}</button>
      </div>
    {:else if !sections.length}
      <p class="dim">{t(patch?.missing?.includes(part) ? 'patches.missing' : 'patches.nothing')}</p>
    {:else}
      {#each sections as s, si (si)}
        <section class="sec">
          {#if part === 'rift' || sections.length > 1}<h3>{title(s.title)}</h3>{/if}
          {#if s.intro}
            <details class="intro">
              <summary>{t('patches.devNote')}</summary>
              <p>{s.intro}</p>
            </details>
          {/if}
          {#each s.entries as e, ei (ei)}
            {#if ei === 0 || e.group !== s.entries[ei - 1]?.group}
              {#if e.group}<h4>{title(e.group)}</h4>{/if}
            {/if}
            {@const v = verdict(e)}
            <div class="entry" class:named={Boolean(e.name)}>
              {#if e.name}
                <button class="pic" disabled={!e.championId} onclick={() => open(e)}>
                  {#if icon(e)}<Img src={icon(e)} size={36} />{:else}<span class="blank"><Icon name="flag" size={16} /></span>{/if}
                </button>
              {/if}
              <div class="body">
                {#if e.name}
                  <div class="name">
                    <span>{name(e)}</span>
                    {#if e.isNew}<span class="tag new">{t('patches.new')}</span>{/if}
                    {#if v}<span class="tag {v}">{t(`patches.${v}`)}</span>{/if}
                  </div>
                {/if}
                {#if e.summary}<p class="summary">{e.summary}</p>{/if}
                {#each e.parts as p, pi (pi)}
                  {#if p.title}<div class="part">{partTitle(e, p.title)}</div>{/if}
                  <ul>
                    {#each p.lines as l, li (li)}
                      {@const b = better(l)}
                      <li>
                        {#if l.attr && l.after}
                          <span class="attr">{attrLabel(l.attr)}</span>{colon}<span class="before">{l.before}</span> <span class="arrow">⇒</span>
                          <b class:good={b === true} class:bad={b === false}>{l.after}</b>
                        {:else}{l.text}{/if}
                      </li>
                    {/each}
                  </ul>
                {/each}
              </div>
            </div>
          {/each}
        </section>
      {/each}
      <p class="credit">{t('patches.credit')}</p>
    {/if}
  </div>
</div>

<style>
  .page {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px 14px;
    min-height: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .head h2 {
    font-size: 22px;
  }
  .grow {
    flex: 1;
  }
  .dim {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .sec {
    flex-shrink: 0;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 12px 16px;
  }
  .sec h3 {
    font-size: 15px;
    margin: 0 0 6px;
  }
  .sec h4 {
    margin: 14px 0 4px;
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
  }
  .intro {
    margin: 0 0 6px;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .intro summary {
    cursor: pointer;
  }
  .intro p {
    margin: 6px 0 0;
    line-height: 1.55;
  }
  .entry {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    padding: 10px 0;
    border-top: 1px solid var(--seam);
  }
  .entry.named {
    grid-template-columns: 36px minmax(0, 1fr);
  }
  .pic {
    padding: 0;
    align-self: start;
  }
  .pic:not(:disabled) {
    cursor: pointer;
  }
  .blank {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: var(--r-md);
    background: var(--panel-3);
    color: var(--text-3);
  }
  .body {
    min-width: 0;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    color: var(--text);
    font-size: 14px;
  }
  .tag {
    font-size: 11px;
    font-weight: 700;
    padding: 1px 7px;
    border-radius: var(--r-sm);
  }
  .tag.new {
    background: color-mix(in oklab, var(--you) 22%, var(--panel));
    color: var(--you);
  }
  .tag.buff {
    background: color-mix(in oklab, var(--win) 18%, var(--panel));
    color: var(--win);
  }
  .tag.nerf {
    background: color-mix(in oklab, var(--death) 18%, var(--panel));
    color: var(--death);
  }
  .summary {
    margin: 4px 0 0;
    color: var(--text-3);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .part {
    margin-top: 8px;
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 600;
  }
  ul {
    margin: 4px 0 0;
    padding-left: 18px;
    color: var(--text-2);
    font-size: 13px;
  }
  li {
    margin: 2px 0;
  }
  .attr {
    color: var(--text);
  }
  .before {
    color: var(--text-3);
  }
  .arrow {
    color: var(--text-3);
  }
  b {
    color: var(--text);
  }
  b.good {
    color: var(--win);
  }
  b.bad {
    color: var(--death);
  }
  .credit {
    margin: 0;
    color: var(--text-3);
    font-size: 12px;
  }
  .none {
    display: grid;
    justify-items: start;
    gap: 10px;
  }
</style>
