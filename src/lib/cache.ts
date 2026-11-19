/**
 * Backend answers kept while the app runs, so a page opened again draws at once instead of
 * showing "loading" while it asks again (pages are rebuilt on every visit).
 *
 * An answer younger than its time to live is used as it is. An older one is still shown at once,
 * and replaced when the new answer comes. The backend keeps its own copies on disk with their own
 * lifetimes (`store::Ttl`, AramHex data sets, Data Dragon per patch); these times only say how
 * often a page asks the backend again. Failed requests are not kept. One request runs per key.
 */

/** AramHex numbers: the backend looks for a new data set every 6 hours. */
export const HEX_TTL = 10 * 60_000;
/** OP.GG tier lists and builds: the backend renews them every 12 and 24 hours. */
export const OPGG_TTL = 30 * 60_000;
/** A finished game and its scores never change. */
export const FOREVER = Infinity;

type Entry = { at: number; value: unknown };
const entries = new Map<string, Entry>();
const pending = new Map<string, Promise<unknown>>();
/** The oldest-used answers go first past this many. */
const MAX = 300;

/** The kept answer for `key`, whatever its age. */
export function peek<T>(key: string): T | undefined {
  const e = entries.get(key);
  if (!e) return undefined;
  entries.delete(key);
  entries.set(key, e);
  return e.value as T;
}

function keep(key: string, value: unknown) {
  entries.delete(key);
  entries.set(key, { at: Date.now(), value });
  while (entries.size > MAX) entries.delete(entries.keys().next().value!);
}

function ask<T>(key: string, fetch: () => Promise<T>): Promise<T> {
  let p = pending.get(key) as Promise<T> | undefined;
  if (!p) {
    p = fetch().then(
      (v) => {
        pending.delete(key);
        keep(key, v);
        return v;
      },
      (e) => {
        pending.delete(key);
        throw e;
      },
    );
    pending.set(key, p);
  }
  return p;
}

/** The answer for `key`: the kept one while younger than `ttl`, else a new one. */
export function cached<T>(key: string, ttl: number, fetch: () => Promise<T>): Promise<T> {
  const e = entries.get(key);
  if (e && Date.now() - e.at < ttl) return Promise.resolve(peek<T>(key)!);
  return ask(key, fetch);
}

/**
 * `set` gets the kept answer at once (when there is one), then a new one when the kept one is
 * older than `ttl`. `fail` is called only when nothing could be shown. Returns whether an answer
 * was shown at once.
 */
export function show<T>(key: string, ttl: number, fetch: () => Promise<T>, set: (v: T) => void, fail?: (e: unknown) => void): boolean {
  const e = entries.get(key);
  const had = e !== undefined;
  if (had) set(peek<T>(key)!);
  if (had && Date.now() - e.at < ttl) return true;
  ask(key, fetch).then(
    (v) => {
      if (!had || v !== e.value) set(v);
    },
    (err) => {
      if (!had) fail?.(err);
    },
  );
  return had;
}

/** Drops the kept answers whose key starts with `prefix` (all of them without one). */
export function forget(prefix = '') {
  for (const k of [...entries.keys()]) if (k.startsWith(prefix)) entries.delete(k);
}
