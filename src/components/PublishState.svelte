<script lang="ts">
  /** Admin: whether the latest builds and tier lists have reached everyone. */
  import { api } from '../lib/api';
  import { content, loadContent } from '../lib/content.svelte';
  import { t } from '../lib/i18n.svelte';

  loadContent();
  const s = $derived(content.status);
</script>

<span class="state {s.publish}" title={s.publish === 'failed' ? s.error : t('publish.hint')}>
  <span class="dot"></span>{t(`publish.${s.publish}`)}
  {#if s.publish === 'failed'}<button class="retry" onclick={() => api.publishContent()}>{t('publish.retry')}</button>{/if}
</span>

<style>
  .state {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--win);
  }
  .waiting .dot,
  .publishing .dot {
    background: var(--objective);
  }
  .publishing .dot {
    animation: pulse 0.9s ease-in-out infinite alternate;
  }
  .failed {
    color: var(--death);
  }
  .failed .dot {
    background: var(--death);
  }
  .retry {
    color: var(--you);
    font-weight: 600;
  }
  @keyframes pulse {
    to {
      opacity: 0.3;
    }
  }
</style>
