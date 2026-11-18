<script lang="ts">
  /**
   * The Hextech ARAM pages' shared filters: the patch (AramHex has the latest few) and the skill
   * bracket. Both carry over between the champion board, augments, pairs and champion pages.
   */
  import Segmented from './Segmented.svelte';
  import Select from './Select.svelte';
  import { t } from '../lib/i18n.svelte';
  import { BRACKETS, bracket, hex, patchInfo, setBracket, setPatch } from '../lib/meta.svelte';

  const info = $derived(patchInfo());
  /** A patch without skill brackets offers all players only. */
  const brackets = $derived(info && !info.bands ? BRACKETS.filter((b) => b === 'all') : BRACKETS);
</script>

<div class="filter">
  {#if hex.patches.length}
    <div class="patch">
      <Select
        label={t('hex.patch')}
        value={info?.patch ?? ''}
        options={hex.patches.map((p, i) => ({ value: p.patch, label: i === 0 ? t('hex.patchNow', p.patch) : p.patch }))}
        onchange={(v) => setPatch(v)}
      />
    </div>
  {/if}
  <Segmented
    label={t('meta.bracket')}
    value={brackets.includes(bracket()) ? bracket() : 'all'}
    options={brackets.map((b) => ({ value: b, label: t(`bracket.${b}`), hint: t(`bracket.${b}Hint`) }))}
    onchange={(v) => setBracket(v)}
  />
  {#if info && !info.bands}<span class="note">{t('hex.noBands', info.patch)}</span>{/if}
</div>

<style>
  .filter {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .patch {
    width: 132px;
  }
  .filter :global(.seg) {
    width: auto;
  }
  .note {
    color: var(--text-3);
    font-size: 12px;
  }
</style>
