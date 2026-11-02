<script lang="ts">
  import { app } from '../lib/state.svelte';
</script>

<div class="stack" aria-live="polite">
  {#each app.toasts as toast (toast.id)}
    <div class="toast {toast.tone}" role="status">{toast.text}</div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    right: 20px;
    bottom: 20px;
    display: grid;
    gap: 8px;
    z-index: 100;
    width: 340px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    padding: 10px 14px;
    background: var(--panel);
    border: 1px solid var(--seam-strong);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-pop);
    animation: rise 220ms var(--ease-out);
  }
  .toast.error {
    border-color: color-mix(in oklab, var(--death) 60%, transparent);
  }
  .toast.good {
    border-color: color-mix(in oklab, var(--win) 55%, transparent);
  }
  @keyframes rise {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
