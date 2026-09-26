<!--
  How well do these numbers hold up? (`#/validation`; model review Part 3.1): the planner checked
  against 22 real disasters, the tally first, then one row per event with the household used, what
  happened, what the planner tells that household today, a plain verdict, whether the event is in
  the records the model learned from, and what we changed because of it. The misses stay on the
  page. The tally comes from the engine (`EngineInfo.validation`); the rows are the web's copy of
  the frozen test (lib/validation.ts, docs/VALIDATION.md). Linked from every "Why?" drawer and from
  About.
-->
<script lang="ts">
  import Icon, { type IconName } from '../components/Icon.svelte';
  import Sources from '../components/Sources.svelte';
  import { useApp } from '../lib/app.svelte';
  import { formatDate } from '../lib/format';
  import { href } from '../lib/router.svelte';
  import type { Tally, Verdict } from '../lib/validation';
  import { agrees, tally, VALIDATION_DOC_URL, VALIDATION_EVENTS, VALIDATION_RUN, VERDICT_WORDS, VERDICTS } from '../lib/validation';

  const app = useApp();
  const summary = $derived(app.info?.validation);
  const rows = VALIDATION_EVENTS;
  const fromRows = tally(rows);
  /** The engine's own count when it carries one; the rows' count otherwise. */
  const counts = $derived<Tally>(
    summary ? { covered: summary.covered, partial: summary.partial, short: summary.short, not_modelled: summary.not_modelled } : fromRows,
  );
  const tested = $derived(summary?.events_tested ?? rows.length);
  const matches = $derived(!summary || agrees(summary, rows));
  const docUrl = $derived(summary?.url_anchor ? `${VALIDATION_DOC_URL}#${summary.url_anchor}` : VALIDATION_DOC_URL);

  const ICON: Record<Verdict, IconName> = { covered: 'check', partial: 'circle', short: 'minus', not_modelled: 'info' };

  /** "We checked 22 real disasters. This version covered 6, partly covered 9 and fell short on 6. We cannot model one yet." */
  const sentence = $derived(
    `We checked ${tested} real disasters. This version covered ${counts.covered}, partly covered ${counts.partial} and fell short on ${counts.short}.${
      counts.not_modelled ? ` We cannot model ${counts.not_modelled === 1 ? 'one' : counts.not_modelled} yet.` : ''
    } Here is why, event by event.`,
  );
</script>

<div class="page">
  <h1 id="page-title" tabindex="-1">How well do these numbers hold up?</h1>
  <p class="lead">
    No model of rare events is exact, so we test this one against real disasters and show what we find. For each past disaster we planned for a
    household that lived through it, then compared its targets with what really happened. The misses stay on this page.
  </p>

  <section class="tally card" aria-labelledby="tally-title">
    <h2 id="tally-title">The results</h2>
    <p class="tally__sentence">{sentence}</p>
    <ul class="tally__boxes">
      {#each VERDICTS as v (v)}
        <li class="box box--{v}">
          <span class="box__icon" aria-hidden="true"><Icon name={ICON[v]} /></span>
          <span class="box__count">{counts[v]}</span>
          <span class="box__label">{VERDICT_WORDS[v].label}</span>
        </li>
      {/each}
    </ul>
    <p class="small muted">
      The first version, checked the same way: covered {VALIDATION_RUN.first.covered}, partly covered {VALIDATION_RUN.first.partial}, short on
      {VALIDATION_RUN.first.short}, not modelled {VALIDATION_RUN.first.not_modelled}.
    </p>
    <p class="small muted">
      Recorded {formatDate(VALIDATION_RUN.recorded)} for {VALIDATION_RUN.label}{summary ? `; data ${summary.data_pack}` : ''}{app.info
        ? `; engine ${app.info.engine_version}`
        : ''}.
    </p>
    {#if !matches && summary}
      <p class="small mismatch" role="note">
        The engine you are using counts {summary.events_tested} events differently from the table below, which was recorded with an earlier
        run. The counts above are the engine's own.
      </p>
    {/if}
  </section>

  <section aria-labelledby="words-title">
    <h2 id="words-title">What the results mean</h2>
    <dl class="words">
      {#each VERDICTS as v (v)}
        <div>
          <dt><span class="badge badge--{v}"><Icon name={ICON[v]} /> {VERDICT_WORDS[v].label}</span></dt>
          <dd>{VERDICT_WORDS[v].means}</dd>
        </div>
      {/each}
    </dl>
    <p>
      Each target was read at the usual setting (very serious, 1-in-100). An event with several needs gets the result of the worst one.
      <span class="in-sample"><Icon name="clock" /> In our records</span> means the event happened inside the records the model learned from, so
      getting it right is easier; we say so for every one.
    </p>
  </section>

  <section aria-labelledby="events-title">
    <h2 id="events-title">Every event</h2>
    <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
    <table class="events" role="table">
      <caption class="visually-hidden">
        The {rows.length} events: who we planned for, what happened, what the planner says today, the result, and what we changed
      </caption>
      <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
      <thead role="rowgroup">
        <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
        <tr role="row">
          <th scope="col" role="columnheader">Event</th>
          <th scope="col" role="columnheader">Who we planned for</th>
          <th scope="col" role="columnheader">What happened</th>
          <th scope="col" role="columnheader">What the planner says today</th>
          <th scope="col" role="columnheader">Result</th>
          <th scope="col" role="columnheader">What we changed</th>
        </tr>
      </thead>
      <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
      <tbody role="rowgroup">
        {#each rows as r (r.n)}
          <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
          <tr role="row" id="event-{r.n}">
            <th scope="row" role="rowheader">
              <span class="event__name">{r.event}</span>
              <span class="event__where">{r.place}, {r.when}</span>
              {#if r.in_sample}<span class="in-sample"><Icon name="clock" /> In our records</span>{/if}
            </th>
            <td role="cell"><span class="cell-label" aria-hidden="true">Who we planned for</span>{r.household}</td>
            <td role="cell">
              <span class="cell-label" aria-hidden="true">What happened</span>{r.happened}
              {#if r.sources.length}<Sources ids={r.sources} variant="inline" what={`${r.event}, ${r.place}`} />{/if}
            </td>
            <td role="cell"><span class="cell-label" aria-hidden="true">What the planner says today</span>{r.target}</td>
            <td role="cell">
              <span class="cell-label" aria-hidden="true">Result</span>
              <span class="badge badge--{r.verdict}"><Icon name={ICON[r.verdict]} /> {VERDICT_WORDS[r.verdict].label}</span>
              {#if r.before !== r.verdict}<span class="before">First version: {VERDICT_WORDS[r.before].label.toLowerCase()}</span>{/if}
            </td>
            <td role="cell"><span class="cell-label" aria-hidden="true">What we changed</span>{r.changed}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  <section aria-labelledby="misses-title">
    <h2 id="misses-title">What the misses need</h2>
    <ul>
      <li>
        <strong>Barrier islands</strong> (Long Beach): a city on a barrier island is averaged with its county. It needs storm-surge data for each ZIP
        code.
      </li>
      <li>
        <strong>Wildfires</strong> (Paradise, Lahaina): after a fire, people are away for months where homes burn. It needs data on homes at the edge
        of wild land.
      </li>
      <li><strong>Inland hurricanes</strong> (Utuado): an inland county needs its own share of major hurricanes.</li>
      <li>
        <strong>Very long boil-water notices</strong> (Asheville, Jackson): a notice of seven weeks is beyond the usual setting. A stove, bleach or a
        filter covers a notice of any length while gas or power runs.
      </li>
      <li><strong>Fuel</strong> (Colonial Pipeline): the planner has no fuel target yet. Keeping half a tank is the free step that helps.</li>
    </ul>
    <p>Treat each target as a floor, not a promise. If your area has lived through something longer, plan for that.</p>
  </section>

  <section aria-labelledby="honest-title">
    <h2 id="honest-title">How we keep this honest</h2>
    <ul>
      <li>The events, the households and the scoring rule are written down and frozen before each data refresh, so a change that fixes or breaks a real case shows up.</li>
      <li>Each new major event is added as a test before the data that includes it, and scored on the old data first.</li>
      <li>Every source for every event is listed in the full test.</li>
    </ul>
    <p>
      <a href={docUrl} target="_blank" rel="noopener noreferrer">Read the full test, with every source<span class="visually-hidden"> (opens in a new tab)</span></a>
    </p>
  </section>

  <p class="button-row next">
    <a class="button" href={href('risks')}>Back to your risks</a>
    <a href={href('about')}>About and method</a>
  </p>
</div>

<style>
  .tally {
    margin: var(--s4) 0 var(--s6);
  }
  .tally h2 {
    margin-top: 0;
  }
  .tally__sentence {
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .tally__boxes {
    list-style: none;
    padding: 0;
    margin: var(--s3) 0;
    display: grid;
    gap: var(--s2);
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 9rem), 1fr));
  }
  .tally__boxes li + li {
    margin-top: 0;
  }
  .box {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: var(--s2);
    align-items: center;
    padding: var(--s3);
    border-radius: var(--r2);
    border: 1px solid var(--border);
    border-left-width: 6px;
  }
  .box__icon {
    grid-row: span 2;
    display: inline-flex;
  }
  .box__count {
    font-size: var(--text-2xl);
    font-weight: 750;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
  }
  .box__label {
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .box--covered,
  .badge--covered {
    border-color: var(--good);
    background: var(--good-soft);
    color: var(--good);
  }
  .box--partial,
  .badge--partial {
    border-color: var(--warn-edge);
    background: var(--warn-soft);
    color: var(--warn);
  }
  .box--short,
  .badge--short {
    border-color: var(--danger);
    background: var(--danger-soft);
    color: var(--danger);
  }
  .box--not_modelled,
  .badge--not_modelled {
    border-color: var(--note-edge);
    background: var(--note-soft);
    color: var(--note);
  }
  .box .box__count,
  .box .box__label {
    color: var(--text);
  }
  .mismatch {
    padding: var(--s2) var(--s3);
    background: var(--note-soft);
    border-radius: var(--r1);
  }
  .words {
    display: grid;
    gap: var(--s2);
    margin: 0 0 var(--s3);
  }
  .words div {
    display: grid;
    gap: var(--s1) var(--s3);
    grid-template-columns: minmax(9rem, auto) 1fr;
    align-items: baseline;
  }
  .words dd {
    margin: 0;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25em;
    padding: 0.1em 0.55em;
    border: 1px solid;
    border-radius: 999px;
    font-size: var(--text-sm);
    font-weight: 650;
    white-space: nowrap;
  }
  .in-sample {
    display: inline-flex;
    align-items: center;
    gap: 0.25em;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .events {
    table-layout: auto;
    font-size: var(--text-sm);
  }
  .events th,
  .events td {
    vertical-align: top;
    overflow-wrap: break-word;
  }
  .events th[scope='row'] {
    min-width: 9rem;
  }
  .event__name {
    display: block;
    font-weight: 700;
  }
  .event__where {
    display: block;
    font-weight: 400;
  }
  .before {
    display: block;
    margin-top: var(--s1);
    color: var(--text-muted);
  }
  .cell-label {
    display: none;
  }
  .next {
    margin-top: var(--s6);
  }
  @media (max-width: 56rem) {
    .events,
    .events thead,
    .events tbody,
    .events tr,
    .events th,
    .events td {
      display: block;
    }
    .events thead {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
      white-space: nowrap;
    }
    .events tr {
      border: 1px solid var(--border);
      border-radius: var(--r2);
      margin-bottom: var(--s3);
      padding: var(--s2) var(--s3);
      background: var(--surface);
    }
    .events th,
    .events td {
      border: 0;
      padding: var(--s1) 0;
    }
    .cell-label {
      display: block;
      font-size: 0.8125rem;
      font-weight: 650;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.03em;
    }
    .words div {
      grid-template-columns: 1fr;
    }
  }
  @media print {
    .events tr {
      break-inside: avoid;
    }
    .next {
      display: none;
    }
  }
</style>
