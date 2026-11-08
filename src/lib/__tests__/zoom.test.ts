import { describe, expect, it } from 'vitest';
import { stepScale, zoomKey } from '../zoom.svelte';

const key = (key: string, extra: Partial<KeyboardEvent> = {}) => ({ key, code: '', ctrlKey: true, altKey: false, metaKey: false, ...extra });

describe('zoom', () => {
  it('steps by 10% and stays within 80-140%', () => {
    expect(stepScale(1, 1)).toBe(1.1);
    expect(stepScale(1, -1)).toBe(0.9);
    expect(stepScale(1.3, 1)).toBe(1.4);
    expect(stepScale(1.4, 1)).toBe(1.4);
    expect(stepScale(0.8, -1)).toBe(0.8);
  });

  it('snaps a slider value between steps to the next step', () => {
    expect(stepScale(1.05, 1)).toBe(1.1);
    expect(stepScale(1.05, -1)).toBe(1);
    expect(stepScale(1.1 + 1e-12, 1)).toBe(1.2);
  });

  it('reads the zoom keys', () => {
    expect(zoomKey(key('='))).toBe(1);
    expect(zoomKey(key('+', { shiftKey: true }))).toBe(1);
    expect(zoomKey(key('-'))).toBe(-1);
    expect(zoomKey(key('0'))).toBe(0);
    expect(zoomKey(key('', { code: 'NumpadAdd' }))).toBe(1);
    expect(zoomKey(key('', { code: 'NumpadSubtract' }))).toBe(-1);
    expect(zoomKey(key('=', { ctrlKey: false }))).toBeNull();
    expect(zoomKey(key('=', { altKey: true }))).toBeNull();
    expect(zoomKey(key('a'))).toBeNull();
  });
});
