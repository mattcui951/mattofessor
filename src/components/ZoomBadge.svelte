<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { setBadgeHovered, setScale, zoomBadge } from '../lib/zoom.svelte';
</script>

{#if zoomBadge.visible}
  <div class="badge" role="status" aria-live="polite" onpointerenter={() => setBadgeHovered(true)} onpointerleave={() => setBadgeHovered(false)}>
    <span class="label">{t('zoom.label')}</span>
    <span class="num">{Math.round(zoomBadge.scale * 100)}%</span>
    <button class="btn sm" disabled={zoomBadge.scale === 1} onclick={() => setScale(1)}>{t('zoom.reset')}</button>
  </div>
{/if}

<style>
  .badge {
    position: fixed;
    top: 52px;
    right: 20px;
    z-index: 101;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 6px 6px 14px;
    background: var(--panel);
    border: 1px solid var(--seam-strong);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-pop);
    animation: drop 160ms var(--ease-out);
  }
  .label {
    color: var(--text-2);
  }
  .num {
    min-width: 4ch;
    text-align: right;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  @keyframes drop {
    from {
      transform: translateY(-6px);
      opacity: 0;
    }
  }
</style>
