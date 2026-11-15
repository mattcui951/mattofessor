import { api } from './api';

/** Errors no code caught go to app.log (once each), where a bug report can find them. */
const logged = new Set<string>();

export function logError(e: unknown) {
  const text = e instanceof Error ? (e.stack ?? `${e.name}: ${e.message}`) : String(e);
  if (logged.has(text)) return;
  logged.add(text);
  api.logError(text).catch(() => {});
}
