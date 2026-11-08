<script lang="ts">
  import Rail from './components/Rail.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Toasts from './components/Toasts.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import ZoomBadge from './components/ZoomBadge.svelte';
  import Builds from './views/Builds.svelte';
  import Live from './views/Live.svelte';
  import Meta from './views/Meta.svelte';
  import Profile from './views/Profile.svelte';
  import Appearance from './views/Appearance.svelte';
  import RiotGuide from './views/RiotGuide.svelte';
  import Settings from './views/Settings.svelte';
  import { app } from './lib/state.svelte';
  import { gameEnded, startAutoRefresh } from './lib/profiles.svelte';
  import { installZoom } from './lib/zoom.svelte';

  startAutoRefresh();
  $effect(() => installZoom());
  let epoch = 0;
  $effect(() => {
    // A game ended: the signed-in account has a new game to add.
    if (app.profileEpoch !== epoch) {
      epoch = app.profileEpoch;
      gameEnded();
    }
  });
</script>

<div class="shell">
  <TitleBar />
  <div class="body">
    <Rail />
    <div class="content">
      <UpdateBanner />
      <div class="view">
        {#if !app.ready}
          <div class="empty"><p>...</p></div>
        {:else if app.route.name === 'profile'}
          {#key app.route.puuid}<Profile puuid={app.route.puuid} />{/key}
        {:else if app.route.name === 'live'}
          <Live />
        {:else if app.route.name === 'builds'}
          {#key `${app.route.championId}-${app.route.mode}`}<Builds championId={app.route.championId} role={app.route.role} mode={app.route.mode} />{/key}
        {:else if app.route.name === 'meta'}
          <Meta mode={app.route.mode} />
        {:else if app.route.name === 'appearance'}
          <Appearance />
        {:else if app.route.name === 'riotGuide'}
          <RiotGuide />
        {:else}
          <Settings />
        {/if}
      </div>
    </div>
  </div>
  <Toasts />
  <ZoomBadge />
</div>

<style>
  .shell {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .view {
    flex: 1;
    min-height: 0;
    position: relative;
  }
</style>
