import { afterEach, describe, expect, it, vi } from 'vitest';
import { cached, forget, peek, show } from '../cache';

const tick = () => new Promise((r) => setTimeout(r, 0));

afterEach(() => {
  forget();
  vi.useRealTimers();
});

describe('session cache', () => {
  it('asks once while fresh, and once for requests at the same time', async () => {
    const fetch = vi.fn(async () => 1);
    await Promise.all([cached('a', 1000, fetch), cached('a', 1000, fetch)]);
    expect(await cached('a', 1000, fetch)).toBe(1);
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('shows a kept answer at once, then the new one when it is old', async () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    let n = 0;
    const fetch = async () => ++n;
    const seen: number[] = [];
    expect(show('b', 1000, fetch, (v) => seen.push(v))).toBe(false);
    await tick();
    expect(seen).toEqual([1]);

    expect(show('b', 1000, fetch, (v) => seen.push(v))).toBe(true);
    await tick();
    expect(seen).toEqual([1, 1]);

    vi.setSystemTime(Date.now() + 5000);
    show('b', 1000, fetch, (v) => seen.push(v));
    expect(seen).toEqual([1, 1, 1]);
    await tick();
    expect(seen).toEqual([1, 1, 1, 2]);
  });

  it('keeps no failures and reports them only with nothing to show', async () => {
    const fail = vi.fn();
    show('c', 1000, () => Promise.reject(new Error('offline')), () => {}, fail);
    await tick();
    expect(fail).toHaveBeenCalledTimes(1);
    expect(peek('c')).toBeUndefined();

    await cached('c', 0, async () => 'old');
    show('c', 0, () => Promise.reject(new Error('offline')), () => {}, fail);
    await tick();
    expect(fail).toHaveBeenCalledTimes(1);
    expect(peek('c')).toBe('old');
  });

  it('forgets by prefix', async () => {
    await cached('build:1', 1000, async () => 1);
    await cached('game:1', 1000, async () => 1);
    forget('build:');
    expect(peek('build:1')).toBeUndefined();
    expect(peek('game:1')).toBe(1);
  });
});
