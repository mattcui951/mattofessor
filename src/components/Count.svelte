<script lang="ts">
  // A figure that counts to its new value instead of jumping (average score, LP).
  import { Tween } from 'svelte/motion';
  import { dur, ease } from '../lib/motion';

  let { value, digits = 0 }: { value: number; digits?: number } = $props();
  const shown = new Tween(0, { duration: dur(600), easing: ease });
  $effect(() => {
    shown.set(value, { duration: dur(600) });
  });
</script>

{shown.current.toFixed(digits)}
