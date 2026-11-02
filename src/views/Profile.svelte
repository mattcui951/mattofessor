<script lang="ts">
  import Img from '../components/Img.svelte';
  import Icon from '../components/Icon.svelte';
  import MatchRow from '../components/MatchRow.svelte';
  import ScoreBadge from '../components/ScoreBadge.svelte';
  import TagChip from '../components/TagChip.svelte';
  import { api } from '../lib/api';
  import { champIcon, champName, profileIcon, tierColor, tierText } from '../lib/dd.svelte';
  import { compact, gamesText, one, pct, roleName } from '../lib/format';
  import { i18n, t } from '../lib/i18n.svelte';
  import { app, errorText, go } from '../lib/state.svelte';
  import type { GameRow, MyScore, Profile, Rank, RankPoint } from '../lib/types';

  let { puuid }: { puuid?: string } = $props();

  let profile = $state<Profile | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let query = $state('');
  let searching = $state(false);
  let filter = $state<'all' | 'solo' | 'flex' | 'normal' | 'aram' | 'arena'>('all');

  // One stable state object, updated in place as batches arrive.
  const perfState = $state({ scores: {} as Record<number, MyScore>, scoring: false, run: 0 });

  /** Scores need each full game (cached after the first time): fetched a few at a time, newest first. */
  async function loadScores(p: Profile) {
    const run = ++perfState.run;
    for (const k of Object.keys(perfState.scores)) delete perfState.scores[Number(k)];
    const ids = p.history.filter((g) => !g.remake).map((g) => g.gameId);
    const puuid = p.card.summoner.puuid;
    perfState.scoring = true;
    for (let i = 0; i < ids.length && run === perfState.run; i += 5) {
      try {
        const got = await api.playerScores(puuid, ids.slice(i, i + 5));
        if (run !== perfState.run) return;
        for (const s of got) perfState.scores[s.gameId] = s;
      } catch {
        break;
      }
    }
    if (run === perfState.run) perfState.scoring = false;
  }

  async function load(id: string | undefined) {
    loading = true;
    error = null;
    try {
      profile = await api.profile(id);
      loadScores(profile);
    } catch (e) {
      error = errorText(e);
      profile = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Reload when the player changes, the client (re)connects, or a game ends.
    const id = puuid;
    app.profileEpoch;
    if (app.connected) load(id);
  });

  async function search(e: Event) {
    e.preventDefault();
    if (!query.trim()) return;
    searching = true;
    try {
      const id = await api.search(query.trim());
      query = '';
      go({ name: 'profile', puuid: id });
    } catch (err) {
      error = errorText(err);
    } finally {
      searching = false;
    }
  }

  const FILTERS: Record<string, (g: GameRow) => boolean> = {
    all: () => true,
    solo: (g) => g.queueId === 420,
    flex: (g) => g.queueId === 440,
    normal: (g) => [400, 430, 480, 490].includes(g.queueId),
    aram: (g) => g.mode === 'ARAM' || g.queueId === 450 || g.queueId === 2400,
    arena: (g) => g.placement > 0,
  };
  const games = $derived((profile?.history ?? []).filter(FILTERS[filter]));
  const played = $derived(games.filter((g) => !g.remake));
  const fWins = $derived(played.filter((g) => g.win).length);
  const card = $derived(profile?.card);
  const perfStats = $derived.by(() => {
    const all = Object.values(perfState.scores);
    const mvps = all.filter((s) => s.badge === 'mvp');
    const svps = all.filter((s) => s.badge === 'svp');
    return {
      games: all.length,
      avg: all.length ? all.reduce((a, s) => a + s.score, 0) / all.length : 0,
      mvp: mvps.length,
      svp: svps.length,
    };
  });

  /** Last recorded rank of each month, newest first (solo preferred). */
  const months = $derived.by(() => {
    const pts = (profile?.rankHistory ?? []) as RankPoint[];
    const byMonth = new Map<string, RankPoint>();
    for (const p of pts) {
      const m = p.day.slice(0, 7);
      const cur = byMonth.get(m);
      if (!cur || p.queue === 'solo' || cur.queue !== 'solo') byMonth.set(m, p);
    }
    return [...byMonth.entries()].sort((a, b) => b[0].localeCompare(a[0])).slice(0, 8);
  });
  function monthLabel(m: string) {
    const [y, mo] = m.split('-').map(Number);
    return new Date(y, mo - 1, 1).toLocaleDateString(i18n.lang === 'zh' ? 'zh-CN' : 'en-US', { year: 'numeric', month: 'short' });
  }
</script>

{#snippet rankCard(label: string, r: Rank | null)}
  <div class="rankcard">
    <div class="label">{label}</div>
    {#if r}
      <div class="tier num" style="color:{tierColor(r.tier)}">{tierText(r)} <span class="lp">{t('common.lp', r.lp)}</span></div>
      <div class="sub num">{#if r.losses > 0 || card?.ranked.exact}{r.wins}W {r.losses}L · {t('common.wr', pct(r.wins, r.wins + r.losses))}{:else}{t('row.wins', r.wins)}{/if}</div>
    {:else}
      <div class="tier dim">{t('common.unranked')}</div>
    {/if}
  </div>
{/snippet}

<div class="page">
  {#if !app.connected}
    <div class="empty">
      <Icon name="warn" size={30} />
      <h3>{t('conn.off')}</h3>
      <p>{t('conn.offHint')}</p>
    </div>
  {:else if error && !profile}
    <div class="empty">
      <h3>{error}</h3>
      <button class="btn" onclick={() => load(puuid)}>{t('common.retry')}</button>
    </div>
  {:else if !profile || !card}
    <div class="empty"><p>{t('common.loading')}</p></div>
  {:else}
    <aside class="side">
      <div class="who">
        <Img src={profileIcon(card.summoner.icon)} size={64} />
        <div>
          <h2>{card.summoner.gameName}<span class="tagline">#{card.summoner.tagLine}</span></h2>
          <div class="sub">{t('common.level', card.summoner.level)}</div>
          {#if !profile.isMe}<button class="link" onclick={() => go({ name: 'profile' })}>{t('profile.backToMe')}</button>{/if}
        </div>
      </div>

      {@render rankCard(t('profile.solo'), card.ranked.solo)}
      {@render rankCard(t('profile.flex'), card.ranked.flex)}

      <section>
        <div class="label">{t('profile.pastRanks')}</div>
        <div class="past">
          {#if card.ranked.seasonPeak}<div class="pr"><span>{t('profile.seasonPeak')}</span><b style="color:{tierColor(card.ranked.seasonPeak.tier)}">{tierText(card.ranked.seasonPeak)}</b></div>{/if}
          {#if card.ranked.prevPeak}<div class="pr"><span>{t('profile.prevPeak')}</span><b style="color:{tierColor(card.ranked.prevPeak.tier)}">{tierText(card.ranked.prevPeak)}</b></div>{/if}
          {#if card.ranked.prevEnd}<div class="pr"><span>{t('profile.prevEnd')}</span><b style="color:{tierColor(card.ranked.prevEnd.tier)}">{tierText(card.ranked.prevEnd)}</b></div>{/if}
          {#if months.length}
            <div class="sublabel">{t('profile.recorded')}</div>
            {#each months as [m, p] (m)}
              <div class="pr"><span>{monthLabel(m)}</span><b style="color:{tierColor(p.tier)}">{tierText(p)} <em>{p.lp}</em></b></div>
            {/each}
          {/if}
          {#if !card.ranked.seasonPeak && !card.ranked.prevPeak && !card.ranked.prevEnd && !months.length}<p class="dim">{t('profile.noPast')}</p>{/if}
        </div>
      </section>

      <section>
        <div class="label">{t('score.title')}</div>
        {#if perfStats.games > 0}
          <div class="perf">
            <div class="avg">
              <span class="num big">{perfStats.avg.toFixed(1)}</span>
              <span class="dim">{t('score.avg')} · {t('score.games', perfStats.games)}</span>
            </div>
            <div class="counts">
              <span title={t('score.mvpHint')}><ScoreBadge size="sm" badge="mvp" /> <b class="num">×{perfStats.mvp}</b></span>
              <span title={t('score.svpHint')}><ScoreBadge size="sm" badge="svp" /> <b class="num">×{perfStats.svp}</b></span>
            </div>
          </div>
        {/if}
        {#if perfState.scoring}<p class="dim">{t('score.loading')}</p>{/if}
      </section>

      <section>
        <div class="label">{t('profile.tags')}</div>
        {#if card.tags.length}
          <div class="tags">{#each card.tags as tag (tag.key)}<TagChip {tag} />{/each}</div>
        {:else}
          <p class="dim">{t('profile.noTags')}</p>
        {/if}
      </section>

      <section>
        <div class="label">{t('profile.champions')}</div>
        {#each card.topChamps as c (c.championId)}
          <div class="champ">
            <Img src={champIcon(c.championId)} size={30} />
            <div class="cn">
              <b>{champName(c.championId)}</b>
              <span class="num">{one(c.kills)} / {one(c.deaths)} / {one(c.assists)}</span>
            </div>
            <div class="cw num">
              <b style="color:{pct(c.wins, c.games) >= 55 ? 'var(--win)' : pct(c.wins, c.games) < 45 ? 'var(--death)' : 'var(--text)'}">{pct(c.wins, c.games)}%</b>
              <span>{gamesText(c.games)}</span>
            </div>
          </div>
        {/each}
        {#if card.roles.length}
          <div class="roles">{#each card.roles as r (r.role)}<span>{roleName(r.role)} <b class="num">{r.games}</b></span>{/each}</div>
        {/if}
      </section>

      {#if profile.masteries.length}
        <section>
          <div class="label">{t('profile.mastery')}</div>
          <div class="mast">
            {#each profile.masteries.slice(0, 6) as m (m.championId)}
              <div title={champName(m.championId)}>
                <Img src={champIcon(m.championId)} size={34} />
                <span class="num">{compact(m.points)}</span>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </aside>

    <main class="main">
      <form class="search" onsubmit={search}>
        <Icon name="search" size={16} />
        <input class="field" bind:value={query} placeholder={t('search.placeholder')} spellcheck="false" />
        <button class="btn sm" disabled={searching || !query.trim()}>{t('search.go')}</button>
      </form>
      {#if error}<p class="err">{error}</p>{/if}

      <div class="hist-head">
        <h3>{t('profile.history')}</h3>
        <div class="filters" role="tablist">
          {#each ['all', 'solo', 'flex', 'normal', 'aram', 'arena'] as f (f)}
            <button role="tab" aria-selected={filter === f} class:on={filter === f} onclick={() => (filter = f as typeof filter)}>{t(`filter.${f}`)}</button>
          {/each}
        </div>
        {#if played.length}
          <span class="summary num">{t('profile.recent', played.length)} · {fWins}W {played.length - fWins}L · <b style="color:{pct(fWins, played.length) >= 50 ? 'var(--win)' : 'var(--death)'}">{t('common.wr', pct(fWins, played.length))}</b></span>
        {/if}
        {#if loading}<span class="dim">{t('common.loading')}</span>{/if}
      </div>
      <div class="list">
        {#each games as g (g.gameId)}
          <MatchRow {g} puuid={card.summoner.puuid} perf={perfState.scores[g.gameId]} onprofile={(id) => go({ name: 'profile', puuid: id })} />
        {:else}
          <p class="dim pad">{t('profile.empty')}</p>
        {/each}
      </div>
    </main>
  {/if}
</div>

<style>
  .page {
    height: 100%;
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    overflow: hidden;
  }
  .page > .empty {
    grid-column: 1 / -1;
  }
  .side {
    border-right: 1px solid var(--seam);
    padding: 18px 16px;
    display: grid;
    gap: 16px;
    align-content: start;
    overflow-y: auto;
  }
  .who {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  h2 {
    font-size: 19px;
    word-break: break-word;
  }
  .tagline {
    color: var(--text-3);
    font-weight: 500;
    font-size: 14px;
  }
  .link {
    color: var(--you);
    font-size: 12.5px;
    margin-top: 2px;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
    margin-bottom: 6px;
  }
  .sublabel {
    font-size: 11.5px;
    color: var(--text-3);
    margin-top: 6px;
  }
  .rankcard {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 11px 13px;
  }
  .rankcard .label {
    margin-bottom: 2px;
  }
  .tier {
    font-family: var(--font-score);
    font-weight: 700;
    font-size: 19px;
  }
  .lp {
    font-size: 13px;
    color: var(--text-2);
    font-weight: 600;
  }
  .sub {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .past {
    display: grid;
    gap: 4px;
  }
  .pr {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
  }
  .pr span {
    color: var(--text-3);
  }
  .pr b {
    font-family: var(--font-score);
    font-weight: 600;
    white-space: nowrap;
  }
  .pr em {
    font-style: normal;
    color: var(--text-3);
    font-size: 12px;
  }
  .perf {
    display: grid;
    gap: 8px;
  }
  .avg {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .avg .big {
    font-size: 24px;
  }
  .counts {
    display: flex;
    gap: 16px;
  }
  .counts span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .counts b {
    font-size: 15px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .champ {
    display: grid;
    grid-template-columns: 30px minmax(0, 1fr) auto;
    gap: 9px;
    align-items: center;
    padding: 4px 0;
  }
  .cn {
    display: grid;
    min-width: 0;
  }
  .cn b {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cn span,
  .cw span {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .cw {
    display: grid;
    text-align: right;
  }
  .roles {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    font-size: 12.5px;
    color: var(--text-3);
    margin-top: 6px;
  }
  .roles b {
    color: var(--text-2);
  }
  .mast {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .mast div {
    display: grid;
    justify-items: center;
    gap: 2px;
    font-size: 11px;
    color: var(--text-2);
  }
  .main {
    overflow-y: auto;
    padding: 16px 20px 24px;
    display: grid;
    gap: 12px;
    align-content: start;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-3);
  }
  .search .field {
    flex: 1;
    max-width: 420px;
  }
  .hist-head {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  h3 {
    font-size: 16px;
  }
  .filters {
    display: flex;
    gap: 2px;
    background: var(--bg-sunk);
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    padding: 2px;
  }
  .filters button {
    height: 26px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    color: var(--text-2);
    font-weight: 600;
    font-size: 12.5px;
  }
  .filters button:hover:not(.on) {
    background: var(--panel-2);
    color: var(--text);
  }
  .filters button.on {
    background: var(--panel-3);
    color: var(--you);
  }
  .summary {
    color: var(--text-2);
    font-size: 12.5px;
  }
  .list {
    display: grid;
    gap: 6px;
  }
  .dim {
    color: var(--text-3);
    font-size: 13px;
  }
  .pad {
    padding: 24px 0;
  }
  .err {
    color: var(--death);
  }
</style>
