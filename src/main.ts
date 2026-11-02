import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import Overlay from './overlay/Overlay.svelte';
import Spells from './overlay/Spells.svelte';
import { init } from './lib/state.svelte';

async function start() {
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    await import('./dev/browser-preview');
  }
  const view = location.hash === '#overlay' ? Overlay : location.hash === '#spells' ? Spells : App;
  if (view !== App) document.documentElement.classList.add('overlay');
  await init();
  mount(view, { target: document.getElementById('app')! });
}

start();
