<script lang="ts">
  import Rail from './components/Rail.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Toasts from './components/Toasts.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import Builds from './views/Builds.svelte';
  import Live from './views/Live.svelte';
  import Profile from './views/Profile.svelte';
  import Appearance from './views/Appearance.svelte';
  import RiotGuide from './views/RiotGuide.svelte';
  import Settings from './views/Settings.svelte';
  import { app } from './lib/state.svelte';
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
          <Builds championId={app.route.championId} role={app.route.role} />
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
