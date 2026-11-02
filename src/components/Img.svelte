<script lang="ts">
  /** A square game image (champion, item, spell, rune) with an empty frame while missing. */
  let { src, size = 32, alt = '', round = false, title }: { src: string | null; size?: number; alt?: string; round?: boolean; title?: string } = $props();
  let failed = $state(false);
  $effect(() => {
    src;
    failed = false;
  });
</script>

<span class="img" class:round style="width:{size}px;height:{size}px" {title}>
  {#if src && !failed}
    <img {src} {alt} width={size} height={size} loading="lazy" draggable="false" onerror={() => (failed = true)} />
  {/if}
</span>

<style>
  .img {
    display: inline-block;
    flex: none;
    border-radius: var(--r-md);
    background: var(--panel-3);
    overflow: hidden;
    vertical-align: middle;
  }
  .img.round {
    border-radius: 50%;
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
