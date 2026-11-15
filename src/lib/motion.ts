// One motion vocabulary for the whole app: the same curve as `--ease-out` in app.css and two
// durations. Svelte transitions run through the Web Animations API, which the CSS
// reduced-motion rule cannot reach, so JS motion asks `dur()` for its length.
import { expoOut } from 'svelte/easing';

export const ease = expoOut;
export const FAST = 120;
export const ENTER = 220;

const query = typeof matchMedia === 'function' ? matchMedia('(prefers-reduced-motion: reduce)') : null;

/** `ms`, or 0 when the system asks for reduced motion. */
export function dur(ms: number): number {
  return query?.matches ? 0 : ms;
}
