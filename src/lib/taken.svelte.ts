/** The augments you took this game (Hextech ARAM), as the card reader saw the picks. */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export const taken = $state({ ids: [] as number[] });

let started = false;

/** Follows the picks in this window (once; later calls do nothing). */
export function followTaken() {
  if (started) return;
  started = true;
  invoke<number[]>('taken_augments').then(
    (ids) => (taken.ids = ids),
    () => {
      /* browser preview */
    },
  );
  listen<number[]>('taken-augments', (e) => (taken.ids = e.payload)).catch(() => {});
}
