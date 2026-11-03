import { describe, expect, it } from 'vitest';
import { buildIndex, norm, riotIdOf, searchChamps } from '../champSearch';
import type { Champ } from '../types';

// Shaped like the app's static data: zh_CN has name and title swapped (name = 贾克斯, alias = 武器大师).
const en: Record<string, Champ> = {
  '24': { key: 'Jax', name: 'Jax', alias: 'Grandmaster at Arms', tags: [] },
  '145': { key: 'Kaisa', name: "Kai'Sa", alias: 'Daughter of the Void', tags: [] },
  '36': { key: 'DrMundo', name: 'Dr. Mundo', alias: 'The Madman of Zaun', tags: [] },
  '64': { key: 'LeeSin', name: 'Lee Sin', alias: 'The Blind Monk', tags: [] },
  '62': { key: 'MonkeyKing', name: 'Wukong', alias: 'The Monkey King', tags: [] },
};
const zh: Record<string, Champ> = {
  '24': { key: 'Jax', name: '贾克斯', alias: '武器大师', tags: [] },
  '145': { key: 'Kaisa', name: '卡莎', alias: '虚空之女', tags: [] },
  '36': { key: 'DrMundo', name: '蒙多医生', alias: '祖安狂人', tags: [] },
  '64': { key: 'LeeSin', name: '李青', alias: '盲僧', tags: [] },
  '62': { key: 'MonkeyKing', name: '孙悟空', alias: '齐天大圣', tags: [] },
};
const index = buildIndex([en, zh]);
const first = (q: string) => searchChamps(index, q)[0]?.id;

describe('champion search', () => {
  it('finds a champion by English or Chinese name and title', () => {
    expect(first('jax')).toBe(24);
    expect(first('贾克斯')).toBe(24);
    expect(first('武器大师')).toBe(24);
    expect(first('盲僧')).toBe(64);
    expect(first('blind monk')).toBe(64);
  });

  it('ignores punctuation, spaces and case', () => {
    expect(first('kaisa')).toBe(145);
    expect(first("KAI'SA")).toBe(145);
    expect(first('dr mundo')).toBe(36);
    expect(first('drmundo')).toBe(36);
  });

  it('matches the Data Dragon key', () => {
    expect(first('monkeyking')).toBe(62);
  });

  it('matches pinyin, full or initials', () => {
    expect(first('jiakesi')).toBe(24);
    expect(first('jks')).toBe(24);
    expect(first('mangseng')).toBe(64);
    expect(first('ms')).toBe(64);
    expect(first('sunwukong')).toBe(62);
  });

  it('ranks exact over prefix over contains', () => {
    const hits = searchChamps(index, 'li');
    expect(hits[0].id).toBe(64); // 李青 = "liqing" starts with "li"
    expect(searchChamps(index, 'zzz')).toEqual([]);
    // A name match beats a title match of the same kind: "mon" starts "MonkeyKing" (key) and is
    // inside "the Blind Monk" (title).
    expect(searchChamps(index, 'mon').map((h) => h.id)).toEqual([62, 64]);
    expect(searchChamps(index, '  ')).toEqual([]);
  });

  it('folds full-width letters', () => {
    expect(norm('ＪＡＸ')).toBe('jax');
  });
});

describe('riot id', () => {
  it('reads Name#TAG, including the full-width ＃ from Chinese input', () => {
    expect(riotIdOf('Faker#KR1')).toEqual({ name: 'Faker', tag: 'KR1' });
    expect(riotIdOf(' 小明＃CN1 ')).toEqual({ name: '小明', tag: 'CN1' });
    expect(riotIdOf('a#b#NA1')).toEqual({ name: 'a#b', tag: 'NA1' });
    expect(riotIdOf('Jax')).toBeNull();
    expect(riotIdOf('Name#')).toBeNull();
  });
});
