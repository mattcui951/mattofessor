import { describe, expect, it } from 'vitest';
import { LOCALES, type LangId } from '../locales';
import en from '../../../README.md?raw';
import zh from '../../../README.zh-CN.md?raw';
import zhTW from '../../../README.zh-TW.md?raw';
import ja from '../../../README.ja.md?raw';
import ko from '../../../README.ko.md?raw';
import es from '../../../README.es.md?raw';
import fr from '../../../README.fr.md?raw';

/** Each language's README at the repository root ("README.zh-CN.md" for Simplified Chinese). */
const readme = (id: LangId) => (id === 'en' ? 'README.md' : `README.${id === 'zh' ? 'zh-CN' : id}.md`);

/** Imported by name: a new language's README goes here too (the first test says so). */
const texts: Record<string, string> = { en, zh, 'zh-TW': zhTW, ja, ko, es, fr };

/** The language bar every README starts with: its own language in bold, the others linked. */
const bar = (own: LangId) =>
  LOCALES.map((l) => (l.id === own ? `**${l.name}**` : `[${l.name}](${readme(l.id)})`)).join(' · ');

describe('READMEs', () => {
  it('one per language', () => {
    expect(Object.keys(texts).sort()).toEqual(LOCALES.map((l) => l.id).sort());
  });

  for (const l of LOCALES) {
    it(`${readme(l.id)} starts with the language bar`, () => {
      const text = texts[l.id] ?? '';
      expect(text.split(/\r?\n/)[2]).toBe(bar(l.id));
      expect(text).not.toContain('—');
    });
  }
});
