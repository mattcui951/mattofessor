<script lang="ts">
  /** One box for champions (any language, pinyin too) and players (Name#TAG). */
  import Icon from './Icon.svelte';
  import Img from './Img.svelte';
  import { api } from '../lib/api';
  import { buildIndex, riotIdOf, searchChamps } from '../lib/champSearch';
  import { champIcon, dd, loadOtherNames, otherNames } from '../lib/dd.svelte';
  import { t } from '../lib/i18n.svelte';
  import { errorText, go } from '../lib/state.svelte';

  type Row = { kind: 'player'; name: string; tag: string } | { kind: 'champ'; id: number };

  let query = $state('');
  let open = $state(false);
  let active = $state(0);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const index = $derived(buildIndex([dd.data?.champions, otherNames.champions]));
  const riotId = $derived(riotIdOf(query));
  const rows = $derived.by((): Row[] => {
    const champs: Row[] = riotId ? [] : searchChamps(index, query, 6).map((h) => ({ kind: 'champ', id: h.id }));
    return riotId ? [{ kind: 'player', ...riotId }] : champs;
  });
  const showList = $derived(open && query.trim().length > 0);

  $effect(() => {
    query;
    active = 0;
    error = null;
  });

  function names(id: number) {
    const here = dd.data?.champions[String(id)];
    const there = otherNames.champions?.[String(id)];
    return { name: here?.name ?? `#${id}`, sub: [there?.name, here?.alias].filter(Boolean).join(' · ') };
  }

  async function pick(row: Row | undefined) {
    if (!row) return;
    error = null;
    if (row.kind === 'champ') {
      reset();
      go({ name: 'builds', championId: row.id });
      return;
    }
    busy = true;
    try {
      const puuid = await api.search(`${row.name}#${row.tag}`);
      reset();
      go({ name: 'profile', puuid });
    } catch (e) {
      error = errorText(e);
      open = false;
    } finally {
      busy = false;
    }
  }

  function reset() {
    query = '';
    open = false;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      open = true;
      const n = rows.length;
      if (n) active = (active + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
    } else if (e.key === 'Escape') {
      open = false;
    } else if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault();
      pick(rows[active]);
    }
  }
</script>

<div class="box">
  <div class="input">
    <Icon name="search" size={16} />
    <input
      class="field"
      bind:value={query}
      placeholder={t('search.placeholder')}
      spellcheck="false"
      autocomplete="off"
      role="combobox"
      aria-expanded={showList}
      aria-controls="search-list"
      aria-label={t('search.placeholder')}
      onfocus={() => {
        open = true;
        loadOtherNames();
      }}
      oninput={() => (open = true)}
      onblur={() => setTimeout(() => (open = false), 120)}
      {onkeydown}
    />
    {#if showList}
      <div class="list" id="search-list" role="listbox">
        {#each rows as row, i (row.kind === 'champ' ? row.id : 'player')}
          <button
            type="button"
            class="row"
            class:active={i === active}
            role="option"
            aria-selected={i === active}
            onmouseenter={() => (active = i)}
            onmousedown={(e) => e.preventDefault()}
            onclick={() => pick(row)}
          >
            {#if row.kind === 'champ'}
              {@const n = names(row.id)}
              <Img src={champIcon(row.id)} size={30} alt={n.name} />
              <span class="text"><b>{n.name}</b>{#if n.sub}<small>{n.sub}</small>{/if}</span>
              <span class="go">{t('search.builds')}</span>
            {:else}
              <span class="avatar"><Icon name="user" size={16} /></span>
              <span class="text"><b>{row.name}<em>#{row.tag}</em></b><small>{busy ? t('common.loading') : t('search.player')}</small></span>
              <span class="go">{t('search.go')}</span>
            {/if}
          </button>
        {:else}
          <p class="hint">{t('search.noMatch')}</p>
        {/each}
        {#if !riotId}<p class="hint tip">{t('search.playerTip')}</p>{/if}
      </div>
    {/if}
  </div>
  {#if error}<p class="err">{error}</p>{/if}
</div>

<style>
  .box {
    position: relative;
    width: 100%;
    max-width: 460px;
  }
  .input {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-3);
  }
  .input .field {
    flex: 1;
    min-width: 0;
  }
  .list {
    position: absolute;
    z-index: 20;
    top: calc(100% + 6px);
    left: 24px;
    right: 0;
    padding: 4px;
    background: var(--panel);
    border: 1px solid var(--seam-strong);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-pop);
    display: grid;
    gap: 1px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: var(--r-md);
    text-align: left;
    color: var(--text);
  }
  .row.active {
    background: var(--panel-3);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  .text b {
    font-weight: 600;
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .text em {
    font-style: normal;
    color: var(--text-3);
    font-weight: 500;
  }
  .text small {
    color: var(--text-3);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .avatar {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--panel-3);
    color: var(--text-2);
  }
  .go {
    flex: none;
    font-size: 12px;
    color: var(--text-3);
  }
  .row.active .go {
    color: var(--you);
  }
  .hint {
    padding: 8px 10px;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .hint.tip {
    border-top: 1px solid var(--seam);
    margin-top: 3px;
    padding-top: 7px;
  }
  .err {
    margin-top: 6px;
    padding-left: 24px;
    color: var(--death);
    font-size: 12.5px;
  }
</style>
