<script lang="ts">
  import Img from './Img.svelte';
  import TagChip from './TagChip.svelte';
  import { champIcon, champName, spellIcon, tierColor, tierScore, tierText } from '../lib/dd.svelte';
  import { ago, compact, gamesText, one, pct, queueName, roleName } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import type { Slot, TierDiv } from '../lib/types';

  let { slot, onprofile }: { slot: Slot; onprofile?: (puuid: string) => void } = $props();
  let open = $state(false);

  const card = $derived(slot.card);
  const rank = $derived(card?.ranked.solo ?? card?.ranked.flex ?? null);
  const peak = $derived.by(() => {
    const r = card?.ranked;
    if (!r) return null;
    const cands = [r.seasonPeak, r.prevPeak].filter(Boolean) as TierDiv[];
    const best = cands.sort((a, b) => tierScore(b) - tierScore(a))[0];
    // Only worth a line when it differs from the current rank.
    return best && (!rank || tierScore(best) > tierScore(rank)) ? best : null;
  });
  const recent = $derived((card?.recent ?? []).filter((g) => !g.remake).slice(0, 5));
  const champ = $derived(card?.champ);
  const wr = $derived(card ? pct(card.wins, card.games) : 0);
  const champWr = $derived(champ ? pct(champ.wins, champ.games) : 0);
  const [gameName, tagLine] = $derived((slot.name || '').split('#'));
</script>

<div class="row" class:me={slot.me} class:open class:smurf={card?.smurf}>
  <div class="cq">
    <button class="main" class:stale={card && !card.fresh} onclick={() => card && (open = !open)} aria-expanded={open} disabled={!card}>
      <div class="pic">
        <Img src={champIcon(slot.championId)} size={44} alt={champName(slot.championId)} title={champName(slot.championId)} />
        <div class="spells">
          <Img src={slot.spells[0] ? spellIcon(slot.spells[0]) : null} size={20} />
          <Img src={slot.spells[1] ? spellIcon(slot.spells[1]) : null} size={20} />
        </div>
      </div>
  
      <div class="who">
        <div class="name">
          {#if slot.status === 'hidden'}
            <span class="dim">{t('live.hidden')}</span>
          {:else}
            <b title={slot.name}>{gameName || '...'}</b>
            {#if card}<span class="lvl num" title={t('common.level', card.summoner.level)}>{card.summoner.level}</span>{/if}
          {/if}
        </div>
        <div class="tags">
          {#if slot.status === 'loading'}
            <span class="skel" style="width:84px"></span><span class="skel" style="width:60px"></span>
          {:else if slot.status === 'error'}
            <span class="dim" title={slot.error ?? ''}>{t('live.failed')}</span>
          {:else if card}
            {#each card.tags.slice(0, 4) as tag (tag.key)}<TagChip {tag} size="sm" />{/each}
          {/if}
        </div>
      </div>
  
      <div class="champ">
        {#if champ}
          {#if champ.games > 0}
            <div class="big num"><span style="color:{champWr >= 55 ? 'var(--win)' : champWr < 45 ? 'var(--death)' : 'var(--text)'}">{champWr}%</span> <span class="unit">{gamesText(champ.games)}</span></div>
            <div class="sub num">{one(champ.kills)} / <span class="d">{one(champ.deaths)}</span> / {one(champ.assists)}</div>
          {:else if !card?.fresh}
            <span class="skel" style="width:70px"></span>
          {:else}
            <div class="sub none">{t('row.champNone', champName(slot.championId))}</div>
          {/if}
          {#if champ.masteryPoints > 0}<div class="sub num">{t('row.mastery', compact(champ.masteryPoints))}</div>{/if}
        {:else if slot.status === 'loading'}
          <span class="skel" style="width:70px"></span>
        {/if}
      </div>
  
      <div class="recent" aria-label={t('row.recent')}>
        {#if card && !card.fresh && recent.length === 0}<span class="skel" style="width:150px;height:30px"></span>{/if}
        {#each recent as g (g.gameId)}
          <span class="g" class:win={g.win} title="{champName(g.championId)} · {queueName(g.queueId)} · {ago(g.created)}">
            <Img src={champIcon(g.championId)} size={20} />
            <span class="num kda">{g.kills}/{g.deaths}/{g.assists}</span>
          </span>
        {/each}
      </div>
  
      <div class="rank">
        {#if card}
          <div class="tier num">
            {#if rank && card.ranked.solo === null}<span class="q">{t('filter.flex')}</span>{/if}
            <span style="color:{tierColor(rank?.tier)}">{tierText(rank, true)}</span>{#if rank}<span class="lp">{t('common.lp', rank.lp)}</span>{/if}
          </div>
          <div class="sub num">
            {#if rank && (rank.losses > 0 || card.ranked.exact)}
              <span class:hi={pct(rank.wins, rank.wins + rank.losses) >= 55} class:lo={pct(rank.wins, rank.wins + rank.losses) < 47}>{pct(rank.wins, rank.wins + rank.losses)}%</span> · {gamesText(rank.wins + rank.losses)}
            {:else if rank}
              {t('row.wins', rank.wins)}
            {:else if card.games > 0}
              <span class:hi={wr >= 55} class:lo={wr < 47}>{wr}%</span> · {gamesText(card.games)}
            {/if}
          </div>
          {#if peak}<div class="peak num" style="color:{tierColor(peak.tier)}">{t('row.peak', tierText(peak, true))}</div>{/if}
        {:else if slot.status === 'loading'}
          <span class="skel" style="width:64px"></span>
        {/if}
      </div>
    </button>
  </div>

  {#if open && card}
    <div class="cq">
      <div class="more">
        <div class="col">
          <div class="label">{t('row.topChamps')}</div>
          {#each card.topChamps as c (c.championId)}
            <div class="tc">
              <Img src={champIcon(c.championId)} size={22} />
              <span class="cn">{champName(c.championId)}</span>
              <span class="num">{c.games}</span>
              <span class="num" style="color:{pct(c.wins, c.games) >= 55 ? 'var(--win)' : pct(c.wins, c.games) < 45 ? 'var(--death)' : 'var(--text-2)'}">{pct(c.wins, c.games)}%</span>
            </div>
          {/each}
          {#if card.roles.length}
            <div class="roles">
              {#each card.roles.slice(0, 3) as r (r.role)}<span>{roleName(r.role)} <b class="num">{r.games}</b></span>{/each}
            </div>
          {/if}
        </div>
        <div class="col">
          <div class="label">{t('row.lastGames')}</div>
          {#each card.recent as g (g.gameId)}
            <div class="lg" class:win={g.win} class:remake={g.remake}>
              <i></i>
              <Img src={champIcon(g.championId)} size={20} />
              <span class="num">{g.kills}/{g.deaths}/{g.assists}</span>
              <span class="q">{queueName(g.queueId)}</span>
              <span class="ago">{ago(g.created)}</span>
            </div>
          {/each}
        </div>
        <div class="col">
          <div class="label">{t('profile.pastRanks')}</div>
          {#if card.ranked.solo}<div class="pr"><span>{t('profile.solo')}</span><b style="color:{tierColor(card.ranked.solo.tier)}">{tierText(card.ranked.solo)} {card.ranked.solo.lp}</b></div>{/if}
          {#if card.ranked.flex}<div class="pr"><span>{t('profile.flex')}</span><b style="color:{tierColor(card.ranked.flex.tier)}">{tierText(card.ranked.flex)} {card.ranked.flex.lp}</b></div>{/if}
          {#if card.ranked.seasonPeak}<div class="pr"><span>{t('profile.seasonPeak')}</span><b style="color:{tierColor(card.ranked.seasonPeak.tier)}">{tierText(card.ranked.seasonPeak)}</b></div>{/if}
          {#if card.ranked.prevPeak}<div class="pr"><span>{t('profile.prevPeak')}</span><b style="color:{tierColor(card.ranked.prevPeak.tier)}">{tierText(card.ranked.prevPeak)}</b></div>{/if}
          {#if card.ranked.prevEnd}<div class="pr"><span>{t('profile.prevEnd')}</span><b style="color:{tierColor(card.ranked.prevEnd.tier)}">{tierText(card.ranked.prevEnd)}</b></div>{/if}
          <div class="alltags">{#each card.tags as tag (tag.key)}<TagChip {tag} size="sm" />{/each}</div>
          {#if onprofile && slot.puuid}<button class="btn sm ghost" onclick={() => onprofile?.(slot.puuid)}>{t('row.openProfile')}</button>{/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Two tracks of the team's grid: the row, then its details (empty while closed). The parts
     are the containers the layout queries ask: a container on the row itself would turn its
     subgrid off (containment). */
  .row {
    grid-row: span 2;
    display: grid;
    grid-template-rows: subgrid;
    row-gap: 0;
    border-radius: var(--r-md);
  }
  .cq {
    container-type: inline-size;
    display: grid;
  }
  .row.me {
    background: color-mix(in oklab, var(--you) 8%, transparent);
  }
  .row.open {
    background: var(--panel-2);
  }
  .main {
    width: 100%;
    display: grid;
    grid-template-columns: 70px minmax(0, 1fr) 104px 214px 124px;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    text-align: left;
    border-radius: var(--r-md);
    color: var(--text);
  }
  .main:hover:not(:disabled) {
    background: var(--panel-2);
  }
  .main.stale .recent,
  .main.stale .champ {
    opacity: 0.6;
  }
  .main:disabled {
    cursor: default;
  }
  @container (max-width: 660px) {
    .main {
      grid-template-columns: 70px minmax(0, 1fr) 96px 116px;
    }
    .recent {
      display: none !important;
    }
  }
  .pic {
    display: flex;
    gap: 3px;
    align-items: center;
  }
  .spells {
    display: grid;
    gap: 3px;
  }
  .who {
    min-width: 0;
    display: grid;
    gap: 5px;
  }
  .name {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
  }
  .name b {
    font-weight: 600;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
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
    min-height: 18px;
    overflow: hidden;
    max-height: 40px;
  }
  .dim {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .champ,
  .rank {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .rank {
    text-align: right;
  }
  .big {
    font-size: 15px;
    white-space: nowrap;
  }
  .unit {
    font-size: 11.5px;
    color: var(--text-3);
    font-weight: 500;
  }
  .sub {
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .sub .d {
    color: var(--death);
  }
  .none {
    white-space: normal;
    line-height: 1.25;
  }
  .tier {
    font-size: 13.5px;
    white-space: nowrap;
  }
  .lp {
    margin-left: 5px;
    color: var(--text-3);
    font-size: 11.5px;
    font-weight: 500;
  }
  .q {
    margin-right: 5px;
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-3);
  }
  .hi {
    color: var(--win);
  }
  .lo {
    color: var(--death);
  }
  .peak {
    font-size: 11px;
    white-space: nowrap;
    opacity: 0.9;
  }
  .recent {
    display: flex;
    gap: 3px;
    justify-content: flex-start;
  }
  .g {
    display: grid;
    justify-items: center;
    gap: 2px;
    width: 40px;
    padding-top: 3px;
    border-top: 2px solid var(--death);
  }
  .g.win {
    border-top-color: var(--win);
  }
  .kda {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-2);
    letter-spacing: -0.02em;
    white-space: nowrap;
  }
  .skel {
    display: inline-block;
    height: 12px;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--panel-2), var(--panel-3), var(--panel-2));
    background-size: 200% 100%;
    animation: shimmer 1.2s linear infinite;
  }
  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }
  .more {
    display: grid;
    grid-template-columns: 1fr 1.2fr 1fr;
    gap: 16px;
    padding: 4px 12px 12px 86px;
  }
  @container (max-width: 860px) {
    .more {
      grid-template-columns: 1fr 1.3fr;
      padding-left: 12px;
    }
    .more .col:last-child {
      grid-column: 1 / -1;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      column-gap: 16px;
    }
    .more .col:last-child .label,
    .more .col:last-child .alltags,
    .more .col:last-child .btn {
      grid-column: 1 / -1;
      justify-self: start;
    }
  }
  .col {
    display: grid;
    gap: 4px;
    align-content: start;
    min-width: 0;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
    margin-bottom: 2px;
  }
  .tc {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) 24px 36px;
    gap: 7px;
    align-items: center;
    font-size: 12.5px;
  }
  .tc .cn {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tc .num {
    text-align: right;
  }
  .roles {
    display: flex;
    gap: 10px;
    font-size: 12px;
    color: var(--text-3);
    margin-top: 4px;
  }
  .roles b {
    color: var(--text-2);
  }
  .lg {
    display: grid;
    grid-template-columns: 3px 20px 62px minmax(0, 1fr) auto;
    gap: 7px;
    align-items: center;
    font-size: 12px;
  }
  .lg i {
    height: 18px;
    border-radius: 2px;
    background: var(--death);
  }
  .lg.win i {
    background: var(--win);
  }
  .lg.remake {
    opacity: 0.5;
  }
  .lg.remake i {
    background: var(--text-3);
  }
  .lg .q {
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lg .ago {
    color: var(--text-3);
    font-size: 11px;
  }
  .pr {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12.5px;
  }
  .pr span {
    color: var(--text-3);
  }
  .pr b {
    font-family: var(--font-score);
    font-weight: 600;
  }
  .alltags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 6px 0;
  }
</style>
