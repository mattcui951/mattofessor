<script lang="ts">
  /**
   * A champion's Hextech ARAM adjustments: its own modifiers (from regular ARAM and Hextech
   * ARAM's layer on top), what the mode gives every melee or ranged champion, and its ability
   * changes. In Chinese the ability lines show their translation; a line not translated yet is
   * left out (and counted) rather than shown in English.
   */
  import { bi, gameData, loadGameData, MOD_KEYS, modGood, modText, type ModKey } from '../lib/gamedata.svelte';
  import { ago } from '../lib/format';
  import { i18n, t } from '../lib/i18n.svelte';

  let { championId }: { championId: number } = $props();
  loadGameData();

  const m = $derived(gameData.data?.mayhem);
  const mods = $derived(m?.champions[String(championId)]);
  const stats = $derived(MOD_KEYS.filter((k) => mods?.stats[k] !== undefined).map((k) => ({ key: k, v: mods!.stats[k]!, aram: mods!.aram[k] })));
  const attack = $derived(m?.attackTypes[String(championId)]);
  /** The mode's own rules for this champion's attack type, and for everyone. */
  const env = $derived.by(() => {
    const items = m?.sections.find((s) => s.id === 'champions')?.items ?? [];
    const want = [attack === 'ranged' ? 'Ranged' : attack === 'melee' ? 'Melee' : '', 'Regen', 'Rune'];
    return items.filter((i) => want.includes(i.label.en));
  });
  /** The ability changes as shown: in Chinese only the translated lines. */
  const abilities = $derived(
    (mods?.abilities ?? [])
      .map((a) => ({ ...a, shown: a.lines.map((l) => ({ mode: l.mode, text: i18n.lang === 'zh' ? l.zh : l.text })).filter((l) => l.text) }))
      .filter((a) => a.shown.length),
  );
  const pending = $derived(i18n.lang === 'zh' ? (mods?.abilities ?? []).reduce((n, a) => n + a.lines.filter((l) => !l.zh).length, 0) : 0);
  const checked = $derived(gameData.data?.checkedAt ? ago(Date.parse(gameData.data.checkedAt)) : '');
  const label = (k: ModKey) => t(`mods.${k}`);
</script>

{#if m}
  <section class="card mods">
    <div class="label-row">
      <div class="label">{t('mods.title')}</div>
      <span class="src">{t('mods.source', m.patch, checked)}</span>
    </div>

    <div class="chips">
      {#each stats as s (s.key)}
        <span class="chip" class:good={s.v !== 0 && modGood(s.key, s.v)} class:bad={s.v !== 0 && !modGood(s.key, s.v)}>
          {label(s.key)} <b class="num">{modText(s.key, s.v)}</b>
          {#if s.aram !== undefined}<span class="was" title={t('mods.aramHint')}>{t('mods.aram', modText(s.key, s.aram))}</span>{/if}
        </span>
      {:else}
        <span class="none">{t('mods.none')}</span>
      {/each}
    </div>

    {#if env.length}
      <div class="chips env">
        {#each env as e (e.label.en)}
          <span class="chip" title={bi(e.note)}><span class="k">{bi(e.label)}</span> {bi(e.text)}{#if e.note}<span class="warn" aria-label={bi(e.note)}>!</span>{/if}</span>
        {/each}
      </div>
    {/if}

    {#if abilities.length}
      <div class="abilities">
        {#each abilities as a (a.name.en)}
          <span class="key num">{a.key || '·'}</span>
          <div class="ab">
            <div class="ab-name">{bi(a.name)}</div>
            <ul>
              {#each a.shown as l, i (i)}
                <li class:mayhem={l.mode === 'mayhem'}>{l.text}</li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    {/if}
    {#if abilities.length || pending}
      <p class="note">{t('mods.linesNote')}{#if pending} {t('mods.linesPending', pending)}{/if}</p>
    {/if}
  </section>
{/if}

<style>
  /* The builds page's card look. */
  .card {
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 14px 16px;
  }
  .label {
    font-family: var(--font-score);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-3);
  }
  .mods .label-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .src {
    color: var(--text-3);
    font-size: 12px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips.env {
    margin-top: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    padding: 4px 10px;
    border-radius: var(--r-sm);
    background: var(--panel-2);
    color: var(--text-2);
    font-size: 12.5px;
  }
  .chip b {
    color: var(--text);
    font-weight: 700;
  }
  .chip.good b {
    color: var(--win);
  }
  .chip.bad b {
    color: var(--death);
  }
  .was {
    color: var(--text-3);
    font-size: 11.5px;
  }
  .env .chip {
    background: color-mix(in oklab, var(--assist) 14%, var(--panel));
  }
  .env .k {
    color: var(--assist);
    font-weight: 700;
  }
  .warn {
    display: inline-grid;
    place-items: center;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: var(--warn);
    color: var(--bg);
    font: 800 10px/1 var(--font-ui);
    align-self: center;
  }
  .none {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .abilities {
    display: grid;
    grid-template-columns: 26px minmax(0, 1fr);
    gap: 8px 10px;
    margin-top: 12px;
  }
  .key {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    background: var(--panel-3);
    color: var(--text-2);
    font: 700 12px var(--font-score);
  }
  .ab {
    min-width: 0;
  }
  .ab-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    line-height: 26px;
  }
  .ab ul {
    margin: 0;
    padding-left: 16px;
    color: var(--text-2);
    font-size: 12.5px;
  }
  .ab li {
    margin: 2px 0;
  }
  .ab li.mayhem {
    color: var(--text);
  }
  .ab li.mayhem::marker {
    color: var(--you);
  }
  .note {
    margin: 10px 0 0;
    color: var(--text-3);
    font-size: 12px;
  }
</style>
