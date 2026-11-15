import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import Overlay from './overlay/Overlay.svelte';
import Spells from './overlay/Spells.svelte';
import Cards from './overlay/Cards.svelte';
import { logError } from './lib/errors';
import { init } from './lib/state.svelte';

// Errors no code caught go to app.log.
window.addEventListener('error', (e) => logError(e.error ?? e.message));
window.addEventListener('unhandledrejection', (e) => logError(e.reason));

async function start() {
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    await import('./dev/browser-preview');
  }
  const views: Record<string, typeof App> = { '#overlay': Overlay, '#spells': Spells, '#cards': Cards };
  const view = views[location.hash] ?? App;
  if (view !== App) document.documentElement.classList.add('overlay');
  await init();
  mount(view, { target: document.getElementById('app')! });
}

start();
