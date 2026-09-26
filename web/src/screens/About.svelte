<!--
  Screen 11, About and method: which engine and data versions are running, the credit lines and
  disclaimers the data sources require (shown exactly as the engine gives them), every source,
  the licences, what is stored and where, and how to report a wrong number.
-->
<script lang="ts">
  import { useApp } from '../lib/app.svelte';
  import { formatDate } from '../lib/format';
  import { href } from '../lib/router.svelte';

  const app = useApp();
  const info = $derived(app.info);
  const citations = $derived([...(app.catalogue?.citations ?? [])].sort((a, b) => a.title.localeCompare(b.title)));
  /** The real engine is answering without the national data packs (engine_info lists none). */
  const sampleCounties = $derived(app.source?.kind === 'wasm' && info !== null && info.packs_loaded.length === 0);
</script>

<div class="page page--narrow">
  <h1 id="page-title" tabindex="-1">About and method</h1>
  <p class="lead">
    Ready Reckoner is a free, open planner. It works out your risks and plan on this device from bundled public data, and it shows its work.
  </p>

  <section aria-labelledby="engine-title">
    <h2 id="engine-title">What is running</h2>
    {#if app.source?.kind === 'mock'}
      <div class="mock card">
        <p>
          <strong>This preview uses a stand-in engine.</strong> The real planning engine is still being built, so the risks, targets, prices and
          sources you see are placeholders that behave like the real thing. Every source name starts with "Mock". Please don't use these
          numbers for real decisions yet.
        </p>
        {#if app.source.fallback}<p class="small">The full engine was expected but did not load: {app.source.fallback}</p>{/if}
      </div>
    {:else if sampleCounties}
      <div class="mock card">
        <p>
          <strong>The national data is not loaded.</strong> The real planning engine is running on seven hand-built sample counties
          instead: the places the example households live. Any other ZIP code or county will not be found. Reload the page to try
          loading the data again.
        </p>
      </div>
    {/if}
    <!-- Wide tables scroll sideways on phones; a focusable, labelled region lets keyboard users scroll it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="table-wrap" tabindex="0" role="region" aria-label="Versions">
      <table>
        <tbody>
          <tr><th scope="row">This app</th><td>{__RR_APP_VERSION__}</td></tr>
          <tr><th scope="row">Planning engine</th><td>{info?.engine_version ?? 'loading'}{app.source ? ` (${app.source.kind === 'mock' ? 'stand-in' : 'WebAssembly'})` : ''}</td></tr>
          <tr><th scope="row">Engine contract</th><td>version {info?.api_version ?? '?'}</td></tr>
          <tr><th scope="row">Data packs</th><td>{info?.data_pack_version ?? 'none loaded'}{info?.packs_loaded.length ? ` (${info.packs_loaded.join(', ')})` : ''}</td></tr>
          <tr><th scope="row">Content</th><td>{info?.content_version ?? 'loading'}</td></tr>
        </tbody>
      </table>
    </div>
  </section>

  <section aria-labelledby="method-title">
    <h2 id="method-title">How it works</h2>
    <p>
      Your county gives each hazard a yearly chance; your household changes what those hazards would do. Each hazard becomes a few plain
      consequences, like no power or no safe water, that last some number of days. The plan sizes supplies for those days and buys the
      cheapest protection first with the budget you set. <a href={href('learn', 'numbers')}>How the numbers are made</a>
    </p>
  </section>

  {#if info?.attributions.length}
    <section aria-labelledby="credits-title">
      <h2 id="credits-title">Credits and disclaimers</h2>
      <ul class="credits">
        {#each info.attributions as a (a.source)}
          <li class="card">
            <p class="credit__source">{a.source}{a.version ? ` (${a.version})` : ''}</p>
            <p>{a.text}</p>
            <p class="small muted">Accessed {formatDate(a.accessed)}. <a href={a.url} target="_blank" rel="noopener noreferrer">{a.url}<span class="visually-hidden"> (opens in a new tab)</span></a></p>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section aria-labelledby="sources-title">
    <h2 id="sources-title">Every source</h2>
    <p class="section-intro">Every quantity, rate, duration and price in the app points to one of these. Expert estimates are marked.</p>
    <ol class="source-list">
      {#each citations as c (c.id)}
        <li>
          <a href={c.url} target="_blank" rel="noopener noreferrer">{c.title}<span class="visually-hidden"> (opens in a new tab)</span></a>.
          <span class="muted">{c.publisher}{c.year ? `, ${c.year}` : ''}. Checked {formatDate(c.retrieved)}. {c.license}.</span>
          {#if c.prior}<span class="chip">Expert estimate</span>{/if}
        </li>
      {/each}
    </ol>
  </section>

  <section aria-labelledby="licence-title">
    <h2 id="licence-title">Licences</h2>
    <ul>
      <li>The code is free software under the GNU General Public License, version 3.</li>
      <li>Guidance text and tables are under Creative Commons Attribution-ShareAlike 4.0.</li>
      <li>Data packs are built from public federal sources; each one's terms are recorded with the data.</li>
    </ul>
  </section>

  <section aria-labelledby="privacy-title">
    <h2 id="privacy-title">Privacy</h2>
    <ul>
      <li>Your answers, plan and check-offs are stored only in this browser (under the name <code>rr.plan.v1</code>), and in files you choose to save.</li>
      <li>The planning engine runs on this device and makes no network requests. The app loads only its own files and data.</li>
      <li>There are no accounts, cookies for tracking, analytics or third-party scripts.</li>
      <li>The site's host serves the app's files, like any website host, and never sees your answers.</li>
      <li>"Forget everything" on the <a href={href('maintain')}>Keep it up</a> screen deletes it all from this browser.</li>
    </ul>
  </section>

  <section aria-labelledby="report-title">
    <h2 id="report-title">Found a wrong number?</h2>
    <p>Please tell us. Say which number, on which screen, what you expected, and your source if you have one.</p>
    <p>
      <a class="button" href="https://github.com/holdTheDoorHoid/ready-reckoner/issues/new" target="_blank" rel="noopener noreferrer">Report it on GitHub<span class="visually-hidden"> (opens in a new tab)</span></a>
    </p>
    <p class="small muted">Please don't include your address or details about your household.</p>
  </section>
</div>

<style>
  .mock {
    border-left: 6px solid var(--mock-edge);
    background: var(--mock-bg);
    color: var(--mock-ink);
    margin-bottom: var(--s4);
  }
  .credits {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--s3);
  }
  .credits li + li {
    margin-top: 0;
  }
  .credit__source {
    font-weight: 650;
    margin-bottom: var(--s1);
  }
  .source-list {
    font-size: var(--text-sm);
  }
  .source-list li + li {
    margin-top: var(--s2);
  }
  th[scope='row'] {
    width: 11rem;
  }
</style>
