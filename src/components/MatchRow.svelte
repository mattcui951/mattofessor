<script lang="ts">
  import AugIcon from './AugIcon.svelte';
  import Img from './Img.svelte';
  import ScoreBadge from './ScoreBadge.svelte';
  import { api } from '../lib/api';
  import { champIcon, champName, itemIcon, itemName, runeIcon, spellIcon } from '../lib/dd.svelte';
  import { ago, compact, duration, kdaRatio, queueName } from '../lib/format';
  import { t } from '../lib/i18n.svelte';
  import { augment, loadAugments } from '../lib/meta.svelte';
  import { errorText } from '../lib/state.svelte';
  import type { FullGame, GameRow, MyScore, PlayerScore } from '../lib/types';

  let { g, puuid, perf, onprofile }: { g: GameRow; puuid: string; perf?: MyScore; onprofile: (puuid: string) => void } = $props();
  let open = $state(false);
  let full = $state<FullGame | null>(null);
  let error = $state<string | null>(null);
  let scores = $state<PlayerScore[]>([]);
  let ranked = $state(false);

  const arena = $derived(g.placement > 0);
  const result = $derived(g.remake ? t('common.remake') : arena ? t('common.place', g.placement) : g.win ? t('common.victory') : t('common.defeat'));
  const tone = $derived(g.remake ? 'remake' : g.win ? 'win' : 'loss');

  // Hextech ARAM and Arena: the augments picked, in pick order.
  $effect(() => {
    if (g.augments.length) loadAugments();
  });

  async function toggle() {
    open = !open;
    if (open && !full) {
      error = null;
      try {
        full = await api.game(g.gameId);
        scores = await api.gameScores(g.gameId).catch(() => []);
      } catch (e) {
        error = errorText(e);
      }
    }
  }

  type P = { pid: number; puuid: string; name: string; tag: string; champ: number; k: number; d: number; a: number; dmg: number; cs: number; gold: number; items: number[]; spells: number[]; augs: number[]; win: boolean; team: number; sub: number };
  const players = $derived.by((): P[] => {
    if (!full) return [];
    return full.participants.map((p) => {
      const id = full!.participantIdentities.find((x) => x.participantId === p.participantId)?.player;
      const s = p.stats;
      const n = (k: string) => Number(s[k] ?? 0);
      return {
        pid: p.participantId,
        puuid: id?.puuid ?? '',
        name: id?.gameName ?? '',
        tag: id?.tagLine ?? '',
        champ: p.championId,
        k: n('kills'),
        d: n('deaths'),
        a: n('assists'),
        dmg: n('totalDamageDealtToChampions'),
        cs: n('totalMinionsKilled') + n('neutralMinionsKilled'),
        gold: n('goldEarned'),
        items: [0, 1, 2, 3, 4, 5, 6].map((i) => n(`item${i}`)),
        spells: [p.spell1Id, p.spell2Id],
        augs: [1, 2, 3, 4, 5, 6].map((i) => n(`playerAugment${i}`)).filter((a) => a > 0),
        win: Boolean(s.win),
        team: p.teamId,
        sub: n('playerSubteamId'),
      };
    });
  });
  const scoreOf = (pid: number) => scores.find((s) => s.participantId === pid);
  const groups = $derived.by(() => {
    if (ranked && scores.length) {
      return [[...players].sort((a, b) => (scoreOf(a.pid)?.place ?? 99) - (scoreOf(b.pid)?.place ?? 99))];
    }
    const key = (p: P) => (arena ? p.sub : p.team);
    const ids = [...new Set(players.map(key))].sort((a, b) => a - b);
    return ids.map((id) => players.filter((p) => key(p) === id));
  });
  const maxDmg = $derived(Math.max(1, ...players.map((p) => p.dmg)));
  const withAugs = $derived(players.some((p) => p.augs.length > 0));
</script>

<div class="match {tone}" class:open>
  <button class="line" onclick={toggle} aria-expanded={open}>
    <i class="bar"></i>
    <div class="res">
      <b>{result}</b>
      <span>{queueName(g.queueId)}</span>
      <span>{ago(g.created)} · {duration(g.duration)}</span>
    </div>
    <div class="loadout">
      <Img src={champIcon(g.championId)} size={44} alt={champName(g.championId)} title={champName(g.championId)} />
      <div class="ss">
        <Img src={spellIcon(g.spells[0])} size={21} />
        <Img src={spellIcon(g.spells[1])} size={21} />
      </div>
      {#if g.augments.length}
        <div class="augs">
          {#each g.augments as a, i (i)}<AugIcon id={a} size={19} />{/each}
        </div>
      {:else if g.perks[0] > 0}
        <div class="ss">
          <Img src={runeIcon(g.perks[0])} size={21} round />
          <Img src={runeIcon(g.perkSub)} size={21} round />
        </div>
      {/if}
    </div>
    <div class="kda">
      <div class="num big"><span class="k">{g.kills}</span> / <span class="d">{g.deaths}</span> / <span class="a">{g.assists}</span></div>
      <span class="num">{kdaRatio(g.kills, g.deaths, g.assists)} {t('profile.kda')}</span>
    </div>
    <div class="perf">
      {#if perf}<ScoreBadge score={perf.score} badge={perf.badge} place={perf.place} players={perf.players} />{/if}
    </div>
    <div class="stats">
      <span class="num">{t('match.cs', g.cs)} <em>({(g.cs / Math.max(1, g.duration / 60)).toFixed(1)})</em></span>
      <span class="num">{compact(g.damage)} {t('match.dmg')}</span>
    </div>
    <div class="items">
      {#each g.items as it, i (i)}<Img src={itemIcon(it)} size={26} title={itemName(it)} />{/each}
    </div>
  </button>

  {#if open}
    <div class="detail">
      {#if error}
        <p class="err">{t('match.loadFailed')} <span>{error}</span></p>
      {:else if !full}
        <p class="dim">{t('common.loading')}</p>
      {:else}
        {#if g.augments.length}
          <div class="picked">
            <span class="lbl">{t('match.augments')}</span>
            {#each g.augments as a, i (i)}
              <span class="chip"><AugIcon id={a} size={18} /><span>{augment(a)?.name ?? `#${a}`}</span></span>
            {/each}
          </div>
        {/if}
        {#if scores.length}
          <div class="view" role="tablist">
            <button role="tab" aria-selected={!ranked} class:on={!ranked} onclick={() => (ranked = false)}>{t('score.byTeam')}</button>
            <button role="tab" aria-selected={ranked} class:on={ranked} onclick={() => (ranked = true)}>{t('score.byRank')}</button>
          </div>
        {/if}
        {#each groups as team, ti (ti)}
          <div class="tm">
            <div class="tm-h" class:win={!ranked && team[0]?.win} class:neutral={ranked}>
              {#if ranked}{t('score.ranking')}{:else if arena}{t('common.place', ti + 1)}{:else}{team[0]?.team === 100 ? t('match.team.blue') : t('match.team.red')} · {team[0]?.win ? t('common.victory') : t('common.defeat')}{/if}
            </div>
            {#each team as p (p.puuid + p.champ)}
              <div class="pl" class:self={p.puuid === puuid} class:won={ranked && p.win} class:lost={ranked && !p.win} class:with-augs={withAugs}>
                <Img src={champIcon(p.champ)} size={28} title={champName(p.champ)} />
                <div class="ss2"><Img src={spellIcon(p.spells[0])} size={13} /><Img src={spellIcon(p.spells[1])} size={13} /></div>
                <button class="pname" onclick={() => p.puuid && onprofile(p.puuid)} title={t('row.openProfile')}>{p.name}<span>#{p.tag}</span></button>
                <span class="sc">{#if scoreOf(p.pid)}{@const s = scoreOf(p.pid)!}<ScoreBadge size="sm" score={s.score} badge={s.badge} place={s.place} players={scores.length} />{/if}</span>
                <span class="num">{p.k}/{p.d}/{p.a}</span>
                <span class="dmg"><i style="width:{(p.dmg / maxDmg) * 100}%"></i><span class="num">{compact(p.dmg)}</span></span>
                <span class="num dim">{p.cs}</span>
                <span class="num dim">{compact(p.gold)}</span>
                {#if withAugs}<span class="pa">{#each p.augs as a, i (i)}<AugIcon id={a} size={16} />{/each}</span>{/if}
                <span class="its">{#each p.items as it, i (i)}<Img src={itemIcon(it)} size={20} title={itemName(it)} />{/each}</span>
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .match {
    container-type: inline-size;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    overflow: hidden;
  }
  .line {
    width: 100%;
    display: grid;
    grid-template-columns: 4px 120px 136px 110px 82px 120px minmax(0, 1fr);
    align-items: center;
    gap: 14px;
    padding: 8px 12px 8px 0;
    text-align: left;
    color: var(--text);
  }
  .line:hover {
    background: var(--panel-2);
  }
  .bar {
    align-self: stretch;
    margin: -8px 0;
    background: var(--death);
  }
  .win .bar {
    background: var(--win);
  }
  .remake .bar {
    background: var(--text-3);
  }
  .res {
    display: grid;
    gap: 1px;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    min-width: 0;
  }
  .res b {
    font-family: var(--font-score);
    font-size: 14px;
    color: var(--death);
  }
  .win .res b {
    color: var(--win);
  }
  .remake .res b {
    color: var(--text-3);
  }
  .loadout {
    display: flex;
    gap: 3px;
    align-items: center;
  }
  .ss {
    display: grid;
    gap: 2px;
  }
  /* Up to six augments: two rows of three, as tall as the champion icon. */
  .augs {
    display: grid;
    grid-template-columns: repeat(3, 19px);
    grid-auto-rows: 19px;
    gap: 5px 4px;
    align-content: center;
    margin-left: 3px;
  }
  .picked {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
  }
  .picked .lbl {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-2);
    margin-right: 2px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 9px 0 5px;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    font-size: 12px;
    color: var(--text);
  }
  .kda {
    display: grid;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .big {
    font-size: 16px;
    color: var(--text);
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
  .stats {
    display: grid;
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .stats em {
    font-style: normal;
    color: var(--text-3);
  }
  /* Narrow windows: drop the CS / damage column and let items wrap. */
  @container (max-width: 860px) {
    .line {
      grid-template-columns: 4px 108px 136px 100px 82px minmax(0, 1fr);
      gap: 10px;
    }
    .stats {
      display: none;
    }
    .items {
      flex-wrap: wrap;
      justify-content: flex-start;
    }
  }
  .perf {
    display: flex;
    justify-content: flex-start;
  }
  .sc {
    display: flex;
    justify-content: flex-end;
  }
  .items {
    display: flex;
    gap: 3px;
    justify-content: flex-end;
  }
  .detail {
    border-top: 1px solid var(--seam);
    padding: 10px 12px 12px;
    display: grid;
    gap: 10px;
    background: var(--bg-sunk);
  }
  .tm {
    display: grid;
    gap: 2px;
  }
  .tm-h {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--death);
    padding-bottom: 3px;
  }
  .tm-h.neutral {
    color: var(--text-2);
  }
  .view {
    display: flex;
    gap: 2px;
    justify-self: start;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    padding: 2px;
  }
  .view button {
    height: 24px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
  }
  .view button.on {
    background: var(--panel-3);
    color: var(--you);
  }
  /* Ranking view: which side each player was on. */
  .pl.won {
    box-shadow: inset 3px 0 0 var(--win);
  }
  .pl.lost {
    box-shadow: inset 3px 0 0 var(--death);
  }
  .tm-h.win {
    color: var(--win);
  }
  .pl {
    display: grid;
    grid-template-columns: 28px 13px minmax(0, 1.4fr) 76px 70px minmax(0, 1fr) 40px 44px auto;
    gap: 8px;
    align-items: center;
    font-size: 12.5px;
    padding: 2px 4px;
    border-radius: var(--r-sm);
  }
  .pl.with-augs {
    grid-template-columns: 28px 13px minmax(0, 1.4fr) 76px 70px minmax(0, 1fr) 40px 44px 112px auto;
  }
  .pa {
    display: flex;
    gap: 3px;
  }
  .pl.self {
    background: color-mix(in oklab, var(--you) 10%, transparent);
  }
  .ss2 {
    display: grid;
    gap: 2px;
  }
  .pname {
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .pname:hover {
    color: var(--you);
    text-decoration: underline;
  }
  .pname span {
    color: var(--text-3);
    font-size: 11px;
  }
  .dmg {
    position: relative;
    height: 16px;
    background: var(--panel-2);
    border-radius: 2px;
    overflow: hidden;
  }
  .dmg i {
    position: absolute;
    inset: 0 auto 0 0;
    background: color-mix(in oklab, var(--death) 45%, transparent);
  }
  .dmg span {
    position: relative;
    padding-left: 5px;
    font-size: 11px;
    line-height: 16px;
  }
  .its {
    display: flex;
    gap: 2px;
  }
  .dim {
    color: var(--text-3);
  }
  .err {
    color: var(--death);
  }
  .err span {
    color: var(--text-3);
    font-size: 12px;
  }
</style>
