<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import Icon from '../components/Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import { app, back, go, toast } from '../lib/state.svelte';
  import dashboard from '../assets/riot-guide/1-dashboard.webp';
  import personal from '../assets/riot-guide/2-personal.webp';
  import form from '../assets/riot-guide/3-form.webp';
  import approved from '../assets/riot-guide/4-approved.webp';

  /** What Riot's form asks for; Riot reviews it in English. */
  const DESCRIPTION = `Mattofessor is a personal desktop tool (Windows) for my own use only. It is not published, sold or shared, and has no website or other users.

What it does: while I play League of Legends, it shows a small in-game panel with the ranked standing of the other 9 players in my match (rank, LP, wins, losses and win rate), so I can understand my teammates and opponents. Outside of games I use it to look at my own ranked progress and match history.

APIs used:
- ACCOUNT-V1 (accounts/by-riot-id): convert each player's Riot ID to a PUUID.
- LEAGUE-V4 (entries/by-puuid): read their solo/duo and flex ranked entries, including wins and losses.

Usage: about 2 requests per player, so about 20 requests per game, only when I am in a game or look up a player myself. Requests are throttled to stay within the personal key limits (20 per second, 100 per 2 minutes), Retry-After is respected on 429 responses, and PUUIDs are cached so repeated lookups are not re-requested. All other data (champion names and icons) comes from Riot's public Data Dragon.

Data and key handling: results are shown only on my own PC and are not stored on any server or shared. The API key is stored encrypted with Windows' per-user encryption (DPAPI) and is sent only to Riot's API servers.`;

  let zoomed = $state<string | null>(null);

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast(t('guide.copied'), 'good');
    } catch {
      toast(t('appearance.exportFailed'), 'error');
    }
  }

  const status = $derived(app.riotStatus);
</script>

{#snippet shot(src: string, alt: string)}
  <button class="shot" onclick={() => (zoomed = src)} title={t('guide.zoom')}>
    <img {src} {alt} loading="lazy" />
  </button>
{/snippet}

<div class="page">
  <div class="col">
    <div class="top">
      <button class="btn ghost sm" onclick={back}><Icon name="chevron-left" size={15} />{t('nav.settings')}</button>
      <h2>{t('guide.title')}</h2>
    </div>
    <p class="lead">{t('guide.intro')}</p>

    <ol class="steps">
      <li>
        <h3>{t('guide.s1.title')}</h3>
        <p>{t('guide.s1.body')}</p>
        <button class="btn sm" onclick={() => openUrl('https://developer.riotgames.com/')}><Icon name="open" size={14} />developer.riotgames.com</button>
      </li>

      <li>
        <h3>{t('guide.s2.title')}</h3>
        <p>{t('guide.s2.body')}</p>
        <ul class="callouts">
          <li><span class="n">1</span>{t('guide.s2.c1')}</li>
          <li><span class="n">2</span>{t('guide.s2.c2')}</li>
        </ul>
        {@render shot(dashboard, t('guide.s2.title'))}
      </li>

      <li>
        <h3>{t('guide.s3.title')}</h3>
        <p>{t('guide.s3.body')}</p>
        <ul class="callouts">
          <li><span class="n">3</span>{t('guide.s3.c3')}</li>
        </ul>
        {@render shot(personal, t('guide.s3.title'))}
      </li>

      <li>
        <h3>{t('guide.s4.title')}</h3>
        <div class="split">
          <div class="fields">
            <ul class="callouts">
              <li><span class="n">1</span><span>{t('guide.s4.c1')} <code>Mattofessor</code></span></li>
              <li><span class="n">2</span><span>{t('guide.s4.c2')}</span></li>
              <li><span class="n">3</span><span>{t('guide.s4.c3')}</span></li>
              <li><span class="n">4</span><span>{t('guide.s4.c4')}</span></li>
              <li><span class="n">5</span><span>{t('guide.s4.c5')}</span></li>
              <li><span class="n">6</span><span>{t('guide.s4.c6')}</span></li>
            </ul>
            <div class="desc">
              <div class="desc-head">
                <span>{t('guide.s4.descLabel')}</span>
                <button class="btn sm primary" onclick={() => copy(DESCRIPTION)}><Icon name="duplicate" size={14} />{t('guide.copyDesc')}</button>
              </div>
              <pre>{DESCRIPTION}</pre>
            </div>
          </div>
          <div class="form-shot">{@render shot(form, t('guide.s4.title'))}</div>
        </div>
      </li>

      <li>
        <h3>{t('guide.s5.title')}</h3>
        <p>{t('guide.s5.body')}</p>
        <ul class="callouts">
          <li><span class="n">1</span>{t('guide.s5.c1')}</li>
        </ul>
        {@render shot(approved, t('guide.s5.title'))}
      </li>

      <li>
        <h3>{t('guide.s6.title')}</h3>
        <p>{t('guide.s6.body')}</p>
        <div class="finish">
          <button class="btn primary" onclick={() => go({ name: 'settings' })}><Icon name="settings" size={15} />{t('guide.toSettings')}</button>
          <span class="state {status}"><i></i>{t(`riot.status.${status}`)}</span>
        </div>
      </li>
    </ol>

    <p class="note">{t('guide.privacy')}</p>
  </div>
</div>

{#if zoomed}
  <button class="lightbox" onclick={() => (zoomed = null)} aria-label={t('common.close')}>
    <img src={zoomed} alt="" />
  </button>
{/if}

<style>
  .page {
    height: 100%;
    overflow-y: auto;
  }
  .col {
    max-width: 980px;
    padding: 18px 28px 48px;
    display: grid;
    gap: 14px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  h2 {
    font-size: 22px;
  }
  .lead {
    color: var(--text-2);
    max-width: 720px;
  }
  .steps {
    list-style: none;
    counter-reset: step;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 18px;
  }
  .steps > li {
    counter-increment: step;
    position: relative;
    display: grid;
    gap: 10px;
    justify-items: start;
    background: var(--panel);
    border: 1px solid var(--seam);
    border-radius: var(--r-lg);
    padding: 16px 18px 18px 58px;
  }
  .steps > li::before {
    content: counter(step);
    position: absolute;
    left: 16px;
    top: 14px;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--you);
    color: var(--you-ink);
    font-family: var(--font-score);
    font-weight: 700;
  }
  h3 {
    font-size: 16px;
  }
  p {
    color: var(--text-2);
    max-width: 760px;
  }
  .callouts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 7px;
    color: var(--text-2);
    font-size: 13px;
  }
  .callouts li {
    display: flex;
    gap: 9px;
    align-items: flex-start;
  }
  /* Matches the numbered badges drawn on the screenshots. */
  .n {
    flex: none;
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: #00a8d6;
    color: #fff;
    font-family: var(--font-score);
    font-weight: 700;
    font-size: 12px;
    margin-top: 1px;
  }
  code {
    font-family: Consolas, ui-monospace, monospace;
    background: var(--bg-sunk);
    border: 1px solid var(--seam);
    border-radius: var(--r-sm);
    padding: 0 5px;
    color: var(--text);
  }
  .shot {
    display: block;
    max-width: 100%;
    border: 1px solid var(--seam-strong);
    border-radius: var(--r-md);
    overflow: hidden;
    cursor: zoom-in;
    background: #fff;
  }
  .shot img {
    display: block;
    max-width: 100%;
    height: auto;
  }
  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 360px);
    gap: 18px;
    align-items: start;
    width: 100%;
  }
  @media (max-width: 1100px) {
    .split {
      grid-template-columns: 1fr;
    }
  }
  .fields {
    display: grid;
    gap: 14px;
  }
  .desc {
    background: var(--bg-sunk);
    border: 1px solid var(--seam);
    border-radius: var(--r-md);
    padding: 10px 12px;
    display: grid;
    gap: 8px;
  }
  .desc-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-3);
  }
  pre {
    margin: 0;
    white-space: pre-wrap;
    font-family: Consolas, ui-monospace, monospace;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--text-2);
    max-height: 220px;
    overflow-y: auto;
    user-select: text;
  }
  .finish {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .state {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: var(--text-2);
  }
  .state i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .state.ok i {
    background: var(--win);
  }
  .state.invalid i {
    background: var(--death);
  }
  .state.unchecked i,
  .state.unreachable i {
    background: var(--warn);
  }
  .note {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(0 0 0 / 0.75);
    cursor: zoom-out;
  }
  .lightbox img {
    max-width: 100%;
    max-height: 100%;
    border-radius: var(--r-md);
    box-shadow: var(--shadow-pop);
  }
</style>
