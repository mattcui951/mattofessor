<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import Img from '../components/Img.svelte';
  import { api } from '../lib/api';
  import { champIconByKey, spellByKey, spellIcon } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { appearance } from '../lib/appearance.svelte';
  import { app } from '../lib/state.svelte';
  import { cooldownFor } from './spells';

  /** Ready-at game time per "riotId|spellKey". */
  let timers = $state<Record<string, number>>({});
  let now = $state(Date.now());
  let base = $state({ gameTime: 0, at: Date.now() });

  $effect(() => {
    const g = app.ingame;
    if (g) base = { gameTime: g.gameTime, at: Date.now() };
  });
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(id);
  });
  // A new match starts with no timers.
  $effect(() => {
    if ((app.ingame?.gameTime ?? 0) < 5) timers = {};
  });

  const gameTime = $derived(base.gameTime + (now - base.at) / 1000);
  const enemies = $derived(app.ingame ? app.ingame.players.filter((p) => p.team !== app.ingame!.myTeam) : []);

  function start(riotId: string, key: string, items: number[]) {
    const s = spellByKey(key);
    if (!s) return;
    timers[`${riotId}|${key}`] = gameTime + cooldownFor(s.cooldown, items, app.ingame?.mode ?? '');
  }
  function reset(e: MouseEvent, riotId: string, key: string) {
    e.preventDefault();
    delete timers[`${riotId}|${key}`];
  }
  function left(riotId: string, key: string): number {
    const at = timers[`${riotId}|${key}`];
    return at ? Math.max(0, at - gameTime) : 0;
  }
  let panel: HTMLElement;
  $effect(() => {
    const ro = new ResizeObserver(() => api.fitSpells(panel.scrollHeight * appearance.scale));
    ro.observe(panel);
    return () => ro.disconnect();
  });

  function fmt(s: number) {
    const m = Math.floor(s / 60);
    const r = Math.ceil(s % 60);
    return m > 0 ? `${m}:${String(r === 60 ? 0 : r).padStart(2, '0')}` : `${Math.ceil(s)}`;
  }
</script>

<div class="panel" bind:this={panel}>
  <header data-tauri-drag-region>
    <Icon name="timer" size={15} />
    <span data-tauri-drag-region>{t('spells.title')}</span>
    <button class="x" aria-label={t('common.close')} onclick={() => api.hideSpells()}><Icon name="close" size={14} /></button>
  </header>
  {#if enemies.length === 0}
    <p class="wait">{t('spells.none')}</p>
  {:else}
    <div class="list">
      {#each enemies as p (p.riotId)}
        <div class="enemy" class:dead={p.dead}>
          <Img src={champIconByKey(p.champion)} size={36} title={p.riotId} />
          {#each p.spells as key (key)}
            {@const rem = left(p.riotId, key)}
            <button
              class="sp"
              class:cd={rem > 0}
              title={spellByKey(key)?.name}
              onclick={() => start(p.riotId, key, p.items)}
              oncontextmenu={(e) => reset(e, p.riotId, key)}
            >
              <Img src={spellIcon(key)} size={36} />
              {#if rem > 0}<span class="num">{fmt(rem)}</span>{/if}
            </button>
          {/each}
        </div>
      {/each}
    </div>
    <p class="hint">{t('spells.hint')}</p>
  {/if}
</div>

<style>
  :global(html.overlay),
  :global(html.overlay body) {
    background: transparent !important;
  }
  .panel {
    max-height: 100vh;
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
    gap: 7px;
    height: 30px;
    padding: 0 4px 0 10px;
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-2);
    border-bottom: 1px solid var(--seam);
    cursor: move;
    flex: none;
  }
  header span {
    flex: 1;
  }
  .x {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--text-3);
  }
  .x:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  .list {
    display: grid;
    gap: 6px;
    padding: 8px 10px;
  }
  .enemy {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .enemy.dead :global(.img:first-child) {
    filter: grayscale(1) brightness(0.6);
  }
  .sp {
    position: relative;
    border-radius: var(--r-md);
  }
  .sp.cd :global(.img) {
    filter: grayscale(1) brightness(0.45);
  }
  .sp span {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 13px;
    color: #fff;
    text-shadow: 0 1px 2px #000;
  }
  .sp:hover:not(.cd) {
    box-shadow: 0 0 0 2px var(--you);
  }
  .wait,
  .hint {
    padding: 8px 10px;
    color: var(--text-3);
    font-size: 11.5px;
    line-height: 1.35;
  }
</style>
