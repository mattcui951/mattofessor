import { describe, expect, it } from 'vitest';
import { parseRich } from '../richtext';

const text = (lines: ReturnType<typeof parseRich>) => lines.map((l) => l.map((s) => s.text).join(''));

describe('parseRich', () => {
  it('reads a Data Dragon item description', () => {
    const d =
      '<mainText><stats><attention>65</attention> 攻击力<br><attention>25%</attention> 暴击几率</stats><br><br><passive>完美</passive><br>你的暴击伤害提升<attention>40%</attention>。</mainText><br>';
    const lines = parseRich(d);
    expect(text(lines)).toEqual(['65 攻击力', '25% 暴击几率', '', '完美', '你的暴击伤害提升40%。']);
    expect(lines[0][0]).toEqual({ text: '65', tone: 'key', dim: false });
    expect(lines[0][1].tone).toBe('stat');
    expect(lines[3][0].tone).toBe('title');
  });

  it('keeps tones of augment tooltips and dims the rules', () => {
    const lines = parseRich('Deals <magicDamage>50 - 175 magic damage</magicDamage>.<br><br><rules>Stacks <keyword>fall</keyword> off.');
    expect(text(lines)).toEqual(['Deals 50 - 175 magic damage.', '', 'Stacks fall off.']);
    expect(lines[0][1]).toEqual({ text: '50 - 175 magic damage', tone: 'magic', dim: false });
    expect(lines[2].every((s) => s.dim)).toBe(true);
    expect(lines[2][1].tone).toBe('key');
  });

  it('never keeps markup, entities or empty lines', () => {
    const lines = parseRich("<br><font color='#F0C200'>任务：</font>&nbsp;击杀<br><br><br><li>一<li>二<br>");
    expect(text(lines)).toEqual(['任务： 击杀', '', '一', '二']);
    expect(text(parseRich(''))).toEqual([]);
    expect(text(parseRich('a <script>x</script> b'))).toEqual(['a x b']);
  });
});
