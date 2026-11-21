<script lang="ts">
  /**
   * A square game image (champion, item, spell, rune) with an empty frame while missing.
   * `plate`: white artwork on a clear ground (augment icons), drawn on a dark plate in every
   * theme, as in the game; on a light theme's frame it would not show.
   */
  let { src, size = 32, alt = '', round = false, plate = false, title }: { src: string | null; size?: number; alt?: string; round?: boolean; plate?: boolean; title?: string } = $props();
  let failed = $state(false);
  $effect(() => {
    src;
    failed = false;
  });
</script>

<span class="img" class:round class:plate style="width:{size}px;height:{size}px" {title}>
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
  .img.plate {
    background: var(--art-plate);
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
