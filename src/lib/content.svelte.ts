import { listen } from '@tauri-apps/api/event';
import { api } from './api';
import { app } from './state.svelte';
import type { Content, ContentEdit, ContentStatus } from './types';

/**
 * The admin's Hextech ARAM builds and tier lists (published on GitHub, followed by every copy of
 * the app): loaded once per window and kept current by the "content" event.
 */
export const content = $state({
  data: { updated: '', builds: {}, itemTiers: {}, championTiersByBracket: {} } as Content,
  status: { publish: 'done', error: '', canEdit: false } as ContentStatus,
  loaded: false,
});

export function loadContent() {
  if (content.loaded) return;
  content.loaded = true;
  listen<Content>('content', (e) => {
    content.data = e.payload;
  });
  listen<ContentStatus>('content-status', (e) => {
    content.status = e.payload;
  });
  api.content().then(
    (c) => {
      content.data = c;
    },
    () => {
      content.loaded = false;
    },
  );
  api.contentStatus().then((s) => {
    content.status = s;
  });
}

/** Editing is on: admin mode, in the admin's copy (anyone else never sees the switch). */
export function adminMode() {
  return content.status.canEdit && Boolean(app.settings?.admin);
}

/** Admin: applies an edit now; it is published a few seconds later. */
export async function editContent(edit: ContentEdit) {
  content.data = await api.editContent(edit);
}
