<script lang="ts">
  /**
   * The latest patch notes' Hextech ARAM changes that touch this champion: the champion itself,
   * and the augments and items its players take most, each with the champion's numbers for it.
   */
  import Img from './Img.svelte';
  import { champIcon, champName, dd, itemIcon, itemName } from '../lib/dd.svelte';
  import { gameData, loadGameData, notesFor, noteVerdict, type NoteEntry } from '../lib/gamedata.svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { augment, pct1, share } from '../lib/meta.svelte';
  import type { ChampionPage } from '../lib/types';

  let { championId, page }: { championId: number; page: ChampionPage } = $props();
  loadGameData();

  /** Taken by at least this share of the champion's players to count as its own. */
  const AUG_SHARE = 0.01;
  const ITEM_SHARE = 0.03;

  const notes = $derived(notesFor(gameData.data?.patchNotes));

  type Change = { e: NoteEntry; patch: string; icon: string | null; name: string; stat: string };

  const changes = $derived.by((): Change[] => {
    const out: Change[] = [];
    const me = champName(championId);
    for (const p of (notes?.patches ?? []).slice(0, 2)) {
      for (const e of p.mayhem.flatMap((s) => s.entries)) {
        if (e.championId === championId) {
          const c = page.champion;
          out.push({ e, patch: p.patch, icon: champIcon(championId), name: me, stat: c ? t('hex.chChampion', pct1(c.winRate), c.rank, page.champions) : '' });
        } else if (e.augmentId) {
          const a = page.augments.find((x) => x.id === e.augmentId);
          if (!a || a.pickRate < AUG_SHARE) continue;
          out.push({ e, patch: p.patch, icon: augment(e.augmentId)?.icon ?? null, name: augment(e.augmentId)?.name ?? e.name, stat: t('hex.chWith', me, pct1(a.winRate), share(a.pickRate)) });
        } else if (e.itemId) {
          const i = page.items.find((x) => x.id === e.itemId) ?? page.boots.find((x) => x.id === e.itemId);
          if (!i || i.pickRate < ITEM_SHARE || !dd.data?.items[String(e.itemId)]) continue;
          out.push({ e, patch: p.patch, icon: itemIcon(e.itemId), name: itemName(e.itemId), stat: t('hex.chWith', me, pct1(i.winRate), share(i.pickRate)) });
        }
      }
    }
    return out;
  });

  const lines = (e: NoteEntry) => e.parts.flatMap((p) => p.lines.map((l) => (l.attr && l.after ? `${l.attr} ${l.before ?? ''} ⇒ ${l.after}` : l.text))).filter(Boolean);
</script>

{#if changes.length}
  <div class="list">
    {#each changes as c, i (i)}
      {@const v = noteVerdict(c.e)}
      <div class="ch">
        <div class="hd">
          <Img src={c.icon} size={30} round={Boolean(c.e.augmentId)} />
          <b>{c.name}</b>
          <span class="tag {v || (c.e.isNew ? 'new' : 'changed')}">{v ? t(`patches.${v}`) : c.e.isNew ? t('patches.new') : t('hex.changed')} {c.patch}</span>
        </div>
        {#if c.stat}<p class="stat">{c.stat}</p>{/if}
        <ul>
          {#each lines(c.e).slice(0, 3) as l, li (li)}<li>{l}</li>{/each}
        </ul>
      </div>
    {/each}
  </div>
{:else}
  <p class="none">{t('hex.noChanges')}</p>
{/if}

<style>
  .list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 8px;
  }
  .ch {
    padding: 10px 12px;
    border-radius: var(--r-md);
    background: var(--panel-2);
    min-width: 0;
  }
  .hd {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hd b {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    flex: none;
    padding: 1px 7px;
    border-radius: var(--r-sm);
    font: 600 11px var(--font-score);
    letter-spacing: 0.04em;
    color: var(--text-2);
    background: var(--panel-3);
  }
  .tag.buff {
    color: var(--win);
    background: color-mix(in oklab, var(--win) 14%, transparent);
  }
  .tag.nerf {
    color: var(--death);
    background: color-mix(in oklab, var(--death) 14%, transparent);
  }
  .tag.new {
    color: var(--you);
    background: color-mix(in oklab, var(--you) 14%, transparent);
  }
  .stat {
    margin: 6px 0 0;
    color: var(--text-2);
    font-size: 12px;
  }
  ul {
    margin: 6px 0 0;
    padding-left: 16px;
    font-size: 12.5px;
    line-height: 1.45;
  }
  .none {
    margin: 0;
    color: var(--text-3);
    font-size: 12.5px;
  }
</style>
