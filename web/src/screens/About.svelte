<!--
  Screen 11, About and method: which engine and data versions are running, the credit lines and
  disclaimers the data sources require (shown exactly as the engine gives them, the National Risk
  Index statement first), what data is on this device and where it comes from, every source,
  the licences, what is stored and where, and how to report a wrong number.
-->
<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { MAP_FILE, ZIP_FILES } from '../engine/data-files';
  import type { Phase } from '../engine/loader';
  import { useApp } from '../lib/app.svelte';
  import { formatDate } from '../lib/format';
  import { href } from '../lib/router.svelte';

  const app = useApp();
  const info = $derived(app.info);
  const manifest = $derived(app.manifest);
  const citations = $derived([...(app.catalogue?.citations ?? [])].sort((a, b) => a.title.localeCompare(b.title)));
  /** The real engine is answering without the national data packs: the site has none. */
  const sampleCounties = $derived(
    app.source?.kind === 'wasm' && (app.data ? app.data.core.phase === 'none' : info !== null && info.packs_loaded.length === 0),
  );

  const REPORT_URL = 'https://github.com/holdTheDoorHoid/ready-reckoner/issues/new?template=wrong-number.md';

  function megabytes(bytes: number): string {
    return `${(bytes / 1_000_000).toFixed(1)} MB`;
  }

  function phaseWords(phase: Phase | undefined, lazy: string): string {
    if (phase === 'ready') return 'On this device';
    if (phase === 'loading') return 'Loading now';
    if (phase === 'failed') return 'Did not load (try again from the line at the top)';
    return lazy;
  }

  /** One row per part of the data, from the manifest. */
  const parts = $derived.by(() => {
    if (!manifest) return [];
    const core = manifest.packs.core?.files ?? [];
    const size = (files: { bytes?: number }[]) => files.reduce((sum, f) => sum + (f.bytes ?? 0), 0);
    const zips = core.filter((f) => ZIP_FILES.includes(f.path));
    const counties = core.filter((f) => !ZIP_FILES.includes(f.path));
    const map = (manifest.packs.geo?.files ?? []).filter((f) => f.path === MAP_FILE);
    return [
      { name: 'County data', what: 'For every county: natural hazards, power outages, storms and other events, climate projections, floods, earthquakes, nearby facilities and how resilient the community is.', files: counties.length, bytes: size(counties), status: phaseWords(app.data?.core.phase, 'Loads when the app opens') },
      { name: 'ZIP code list', what: 'Which county each ZIP code is in, and how far it is from nuclear plants and chemical sites.', files: zips.length, bytes: size(zips), status: phaseWords(app.data?.zip.phase, 'Loads when you type a ZIP code') },
      { name: 'County map', what: 'The outline of every county, for the small maps.', files: map.length, bytes: size(map), status: phaseWords(app.data?.map.phase, 'Loads when a map is shown') },
    ].filter((p) => p.files > 0);
  });

  interface JobSource {
    name: string;
    url?: string;
  }
  /** Where each part of the data comes from (the manifest's jobs), newest refresh first. */
  const jobs = $derived.by(() => {
    const raw = (manifest as { jobs?: Record<string, { title?: string; finished?: string; sources?: JobSource[] }> } | null)?.jobs ?? {};
    return Object.entries(raw)
      .map(([id, j]) => ({
        id,
        title: j.title ?? id,
        finished: j.finished?.slice(0, 10) ?? '',
        sources: (j.sources ?? []).map((src) => ({ name: src.name, url: src.url && /^https?:\/\/\S+$/.test(src.url) ? src.url : undefined })),
      }))
      .sort((a, b) => a.title.localeCompare(b.title));
  });

  /** Licences of the data sources, from the manifest (the engine's credit lines omit them). */
  const dataLicences = $derived(
    ((manifest?.attributions ?? []) as { source?: string; license?: string }[]).filter((a) => a.source && a.license).sort((a, b) => a.source!.localeCompare(b.source!)),
  );

  /** The versions a report needs; nothing about the household. */
  const versionLine = $derived(
    `App ${__RR_APP_VERSION__}; engine ${info?.engine_version ?? '?'} (contract ${info?.api_version ?? '?'}); data ${info?.data_pack_version ?? 'none'}; content ${info?.content_version ?? '?'}`,
  );
  let copied = $state('');
  async function copyVersions() {
    try {
      await navigator.clipboard.writeText(versionLine);
      copied = 'Copied. Paste it into the report.';
    } catch {
      copied = 'Your browser would not copy it; select the line above and copy it instead.';
    }
  }
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
          <tr>
            <th scope="row">Data</th>
            <td>
              {#if info?.data_pack_version}
                version {info.data_pack_version}{manifest?.generated ? `, built ${formatDate(manifest.generated.slice(0, 10))}` : ''}
              {:else if app.data?.core.phase === 'loading'}
                loading
              {:else}
                none loaded
              {/if}
            </td>
          </tr>
          <tr><th scope="row">Content</th><td>{info?.content_version ?? 'loading'}</td></tr>
        </tbody>
      </table>
    </div>
  </section>

  {#if parts.length}
    <section aria-labelledby="data-title">
      <h2 id="data-title">The data on this device</h2>
      <p>
        The app downloads its data once, from this site only, and keeps it in this browser so it also works offline. The ZIP code list and the
        map come only when they are needed. Sizes are as stored; the download is smaller.
      </p>
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="table-wrap" tabindex="0" role="region" aria-label="The data on this device">
        <table>
          <thead><tr><th scope="col">Part</th><th scope="col">What it holds</th><th scope="col">Size</th><th scope="col">Status</th></tr></thead>
          <tbody>
            {#each parts as p (p.name)}
              <tr>
                <th scope="row">{p.name}</th>
                <td>{p.what}</td>
                <td class="num">{megabytes(p.bytes)} <span class="muted">({p.files} {p.files === 1 ? 'file' : 'files'})</span></td>
                <td>{p.status}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if jobs.length}
        <details class="jobs">
          <summary>Where the data comes from ({jobs.length} parts{jobs.some((j) => j.finished) ? `, last refreshed ${formatDate(jobs.map((j) => j.finished).sort().at(-1)!)}` : ''})</summary>
          <ul>
            {#each jobs as job (job.id)}
              <li>
                <strong>{job.title}</strong>{#if job.finished}<span class="muted">, refreshed {formatDate(job.finished)}</span>{/if}
                <ul class="job-sources">
                  {#each job.sources as src, i (i)}
                    <li>
                      {#if src.url}<a href={src.url} target="_blank" rel="noopener noreferrer">{src.name}<span class="visually-hidden"> (opens in a new tab)</span></a>{:else}{src.name}{/if}
                    </li>
                  {/each}
                </ul>
              </li>
            {/each}
          </ul>
        </details>
      {/if}
    </section>
  {/if}

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
    {#if dataLicences.length}
      <p class="small">The data's own terms:</p>
      <ul class="small">
        {#each dataLicences as a (a.source)}<li><strong>{a.source}:</strong> {a.license}</li>{/each}
      </ul>
    {/if}
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
    <p>
      Please tell us. The report form asks what the app showed, what you believe is right, and your source. Say which screen, and which
      setting you had on.
    </p>
    <p>
      <a class="button" href={REPORT_URL} target="_blank" rel="noopener noreferrer"><Icon name="alert" /> Report a wrong number on GitHub<span class="visually-hidden"> (opens in a new tab)</span></a>
    </p>
    <p class="small">Add this line so we can see exactly what was running (it says nothing about you):</p>
    <p class="version-line"><code>{versionLine}</code></p>
    <p class="button-row">
      <button type="button" class="button button--small" onclick={copyVersions}>Copy this line</button>
      <span class="small" role="status">{copied}</span>
    </p>
    <p class="small muted">
      GitHub reports are public. Please don't include your address. A saved plan file holds your household's details, so attach one only if you
      are happy for anyone to read it; a made-up household that shows the same number works just as well.
    </p>
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
  .jobs {
    margin-top: var(--s4);
    font-size: var(--text-sm);
  }
  .jobs summary {
    color: var(--accent);
    min-height: 36px;
  }
  .jobs > ul > li + li {
    margin-top: var(--s3);
  }
  .job-sources {
    margin: var(--s1) 0 0;
  }
  .job-sources a {
    word-break: break-word;
  }
  .version-line code {
    display: block;
    padding: var(--s2) var(--s3);
    background: var(--surface-2);
    border-radius: var(--r1);
    font-size: 0.8125rem;
    overflow-wrap: anywhere;
  }
</style>
