<script lang="ts">
  import PlayerRow from './PlayerRow.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { LiveState } from '../lib/types';

  let { live, onprofile }: { live: LiveState; onprofile?: (puuid: string) => void } = $props();

  const many = $derived(live.teams.length > 2);
  function title(i: number, ally: boolean): string {
    if (ally) return t('live.yourTeam');
    if (many) return t('live.team', i + 1);
    return t('live.enemyTeam');
  }
</script>

<div class="teams" class:many>
  {#each live.teams as team, i (team.id)}
    <section class="team">
      <header>
        <span class="label" class:ally={team.ally} class:enemy={!team.ally}>{title(i, team.ally)}</span>
      </header>
      <div class="rows">
        {#each team.players as slot, j (slot.puuid || `${team.id}-${j}`)}
          <PlayerRow {slot} {onprofile} />
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .teams {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 700px), 1fr));
    container-type: inline-size;
  }
  .teams.many {
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 440px), 1fr));
  }
  .team {
    padding: 6px 10px 10px;
    min-width: 0;
  }
  .team + .team {
    border-left: 1px solid var(--seam);
  }
  @container (max-width: 1399px) {
    .teams:not(.many) .team + .team {
      border-left: 0;
      border-top: 1px solid var(--seam);
    }
  }
  header {
    padding: 4px 8px 6px;
  }
  .label {
    font-family: var(--font-score);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-3);
  }
  .label.ally {
    color: var(--you);
  }
  .label.enemy {
    color: var(--death);
  }
  .rows {
    display: grid;
    gap: 2px;
  }
</style>
