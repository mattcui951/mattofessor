<script lang="ts">
  import { ordinal } from '../lib/format';
  import { t } from '../lib/i18n.svelte';

  let {
    score,
    badge = '',
    place = 0,
    players = 0,
    size = 'md',
  }: { score?: number | null; badge?: '' | 'mvp' | 'svp'; place?: number; players?: number; size?: 'sm' | 'md' } = $props();

  const tone = $derived(typeof score !== 'number' ? '' : score >= 7.5 ? 'high' : score < 4 ? 'low' : '');
  const hint = $derived(
    [badge ? t(`score.${badge}Hint`) : '', typeof score === 'number' && place && players ? t('score.hint', score.toFixed(1), place, players) : ''].filter(Boolean).join('\n'),
  );
</script>

<span class="score {size}" title={hint}>
  {#if badge}<span class="badge {badge}">{t(`score.${badge}`)}</span>{:else if place > 0}<span class="badge place" class:top={place <= 3}>{ordinal(place)}</span>{/if}
  {#if typeof score === 'number'}<span class="num value {tone}">{score.toFixed(1)}</span>{/if}
</span>

<style>
  .score {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }
  .value {
    font-size: 15px;
    color: var(--text);
  }
  .sm .value {
    font-size: 12.5px;
  }
  .value.high {
    color: var(--win);
  }
  .value.low {
    color: var(--death);
  }
  .badge {
    font-family: var(--font-score);
    font-weight: 700;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    padding: 1px 6px;
    border-radius: var(--r-sm);
  }
  .badge.mvp {
    background: var(--objective);
    color: var(--bg);
  }
  .badge.svp {
    background: var(--assist);
    color: var(--bg);
  }
  .badge.place {
    background: var(--panel-3);
    color: var(--text-2);
    min-width: 34px;
    text-align: center;
  }
  .badge.place.top {
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--seam-strong);
  }
  .sm .badge {
    font-size: 9.5px;
    padding: 0 5px;
  }
</style>
