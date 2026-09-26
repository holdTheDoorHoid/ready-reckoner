<!--
  Rare but severe (REVIEW §2.4; hazard-expansion Deliverable C): the nine rare families, one line
  each, sorted by how likely they are where the household lives and never by expected loss.
  Five columns: what, how likely for you (a range only, with one comparison against the
  household's own list), if it reaches you, why here, and what it changes in the plan (usually
  nothing beyond the basics, marked with a check). Each row opens to its sub-rows (the named causes
  inside the family) and a "How this number is made" drawer: the range a year, why it is a range,
  what goes into it and where the household lives, each with its sources.

  Below the table, what the rare-event allowance bought, or that it is off. The allowance itself is
  chosen in Your settings (lib/dials.ts); `onchoose` opens them.

  On a phone each row is a block with its labels; a table's roles are set explicitly, so screen
  readers keep the table when the layout changes. The packet prints only the collapsed rows.
-->
<script lang="ts">
  import type { HazardProfile } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import { addMonths, formatMonth, rangeOnly, usd } from '../lib/format';
  import { NUCLEAR_NOTE } from '../lib/labels';
  import { lowerFirst } from '../lib/lookup';
  import { allowance, changesNothing, locationChain, whatItChanges, whyHereShort } from '../lib/rare';
  import Chance from './Chance.svelte';
  import ExplainButton from './ExplainButton.svelte';
  import Icon from './Icon.svelte';
  import Sources from './Sources.svelte';

  let {
    hazards,
    years,
    backToTable = false,
    onchoose,
  }: {
    hazards: HazardProfile[];
    years: number;
    backToTable?: boolean;
    /** Opens the settings where the household chooses the rare-event allowance. */
    onchoose?: () => void;
  } = $props();
  const app = useApp();
  const uid = $props.id();

  let open = $state<Record<string, boolean>>({});

  const span = $derived(years === 1 ? 'the next year' : `the next ${years} years`);
  const output = $derived(app.result.output);
  const dials = $derived(app.plan?.input.dials);
  const monthly = $derived(app.plan?.input.finances.monthly_budget_usd ?? 0);
  const planningDate = $derived(app.plan?.input.planning_date);
  const spend = $derived(output && dials ? allowance(output, app.catalogue, dials, monthly) : null);
  /** The v0.1 worldwide nuclear row had no location term; its note goes once the row is local. */
  const worldwideNuclear = $derived(hazards.some((h) => h.id === 'nuclear_attack' && !h.location_factor));

  function familyName(id: string): string {
    return lowerFirst(hazards.find((h) => h.id === id)?.name ?? app.catalogue?.hazards.find((h) => h.id === id)?.name ?? id);
  }

  function yearly(h: HazardProfile): string {
    return rangeOnly(h.rate_range[0], h.rate_range[1], 1);
  }

  function subRange(lo: number, hi: number): string {
    return rangeOnly(lo, hi, 1);
  }

  function sourcesOf(h: HazardProfile): string[] {
    return [...h.sources, ...(h.location_factor?.sources ?? []), ...(h.sub_causes ?? []).flatMap((s) => s.sources)];
  }

  function factor(x: number): string {
    return `×${Number(x.toPrecision(2))}`;
  }

  function toggle(id: string) {
    open[id] = !open[id];
  }
</script>

{#if hazards.length}
  <section class="rare" aria-labelledby="rare-title">
    <h2 id="rare-title" tabindex="-1">Rare but severe</h2>
    <p class="section-intro">
      These are very unlikely, so they sit apart from the list above: a tiny chance of a huge loss should not crowd out what is likely.
      They are sorted by how likely they are where you live, not by how bad they would be. Each chance is a range, because it rests on expert
      estimates. The plan never spends on them unless you ask it to. Your three-day supplies already cover the first days of sheltering: get
      inside, stay inside, stay tuned.
    </p>
    <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
    <table class="rare-table" role="table">
      <caption class="visually-hidden">
        Rare but severe, sorted by how likely here: how likely for you in {span}, if it reaches you, why here, and what it changes in your plan
      </caption>
      <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
      <thead role="rowgroup">
        <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
        <tr role="row">
          <th scope="col" role="columnheader">What</th>
          <th scope="col" role="columnheader">How likely for you <span class="th-note">(in {span})</span></th>
          <th scope="col" role="columnheader">If it reaches you</th>
          <th scope="col" role="columnheader">Why here</th>
          <th scope="col" role="columnheader">What it changes in your plan</th>
        </tr>
      </thead>
      {#each hazards as h (h.id)}
        {@const isOpen = !!open[h.id]}
        <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
        <tbody class="family" class:is-open={isOpen} id="rare-{h.id}" role="rowgroup">
          <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
          <tr role="row" class="family__row">
            <th scope="row" role="rowheader">
              <button
                type="button"
                class="expander"
                aria-expanded={isOpen}
                aria-controls="{uid}-{h.id}-more"
                onclick={() => toggle(h.id)}
              >
                <span class="expander__icon" aria-hidden="true"><Icon name="chevron-right" /></span>
                <span class="expander__name">{h.name}</span>
                <span class="visually-hidden">{isOpen ? ': hide the details' : ': show the details'}</span>
              </button>
            </th>
            <td role="cell">
              <span class="cell-label" aria-hidden="true">How likely for you</span>
              <span class="likely"><Chance text={rangeOnly(h.rate_range[0], h.rate_range[1], years)} /></span>
              {#if h.anchor_sentence}<span class="anchor">{h.anchor_sentence}</span>{/if}
            </td>
            <td role="cell">
              <span class="cell-label" aria-hidden="true">If it reaches you</span>
              {h.if_it_reaches_you ?? ''}
            </td>
            <td role="cell">
              <span class="cell-label" aria-hidden="true">Why here</span>
              {whyHereShort(h)}
            </td>
            <td role="cell" class="changes" class:changes--nothing={changesNothing(h)}>
              <span class="cell-label" aria-hidden="true">What it changes in your plan</span>
              {#if changesNothing(h)}<span class="tick" aria-hidden="true"><Icon name="check" /></span>{/if}<span>{whatItChanges(h)}</span>
            </td>
          </tr>
          <!-- svelte-ignore a11y_no_redundant_roles (kept: phones restyle the table, and some browsers drop its roles) -->
          <tr role="row" class="more no-print" id="{uid}-{h.id}-more" hidden={!isOpen}>
            <td role="cell" colspan="5">
              {#if h.sub_causes?.length}
                <h3 class="more__title">What it includes</h3>
                <ul class="subrows">
                  {#each h.sub_causes as s (s.id)}
                    <li>
                      <p class="subrow__name">
                        {s.name}{#if s.rate_range}<span class="subrow__range">: <Chance text="{subRange(s.rate_range[0], s.rate_range[1])} a year" /></span>{/if}
                      </p>
                      <p class="subrow__note">{s.note}</p>
                    </li>
                  {/each}
                </ul>
              {/if}
              <details class="how">
                <summary>How this number is made</summary>
                <div class="how__body">
                  <p>
                    <strong>For a household like yours:</strong>
                    <Chance text={rangeOnly(h.rate_range[0], h.rate_range[1], years)} /> in {span}, which is
                    <Chance text={yearly(h)} /> in any one year.
                  </p>
                  {#if h.range_only || h.confidence === 'prior'}
                    <p>
                      <strong>Why a range:</strong> it rests on published forecasts and expert judgement stacked on each other, so it is shown as
                      a range, never as one number. The row is sorted by the middle of the range, which is never shown.
                    </p>
                  {/if}
                  {#if h.location_factor}
                    <p><strong>Where you live:</strong> {h.location_factor.label}</p>
                    {#if locationChain(h)}<p>{locationChain(h)}</p>{/if}
                    {#if app.prefs.expert}
                      <p class="small muted">
                        Expert view: location group {h.location_factor.class}; factor {factor(h.location_factor.multiplier[1])} ({factor(h.location_factor.multiplier[0])}
                        to {factor(h.location_factor.multiplier[2])}).
                      </p>
                    {/if}
                  {:else}
                    <p><strong>Where you live:</strong> this chance is the same everywhere; nothing in the data makes it higher or lower for your county.</p>
                  {/if}
                  {#if h.sub_causes?.some((s) => s.rate_range)}
                    <p><strong>What goes into it:</strong> the named causes above, each with its own range a year where one is known. Some are shown for everyone and never added to your chance; their notes say so.</p>
                  {/if}
                  <Sources ids={sourcesOf(h)} what="how the {lowerFirst(h.name)} range is made" />
                </div>
              </details>
              <div class="more__foot">
                <ExplainButton kind="hazard" id={h.id} label="What to do if it happens" />
                {#if backToTable}
                  <a
                    class="back"
                    href="#matrix-{h.id}"
                    onclick={(e) => {
                      if (jumpTo(`matrix-${h.id}`, { block: 'center' })) e.preventDefault();
                    }}>Back to the table<span class="visually-hidden"> of risks</span></a
                  >
                {/if}
              </div>
            </td>
          </tr>
        </tbody>
      {/each}
    </table>
    {#if worldwideNuclear}
      <p class="small note">{NUCLEAR_NOTE}</p>
    {/if}

    {#if spend && dials}
      <div class="allowance" aria-labelledby="{uid}-allowance">
        <h3 id="{uid}-allowance">What the plan spends on these</h3>
        {#if spend.families.length === 0}
          <p>
            Nothing. The plan spends on these only if you allow it: up to 10% of your monthly budget, for the rows you choose, and only after
            your three-day basics.
          </p>
        {:else if monthly <= 0}
          <p>You allowed spending on {spend.families.length === 9 ? 'all of these' : `${spend.families.length} of these`}, but your monthly budget is $0, so nothing is bought.</p>
        {:else if spend.bought.length === 0}
          <p>
            Nothing to buy for the rows you chose ({spend.families.map(familyName).join(', ')}): your basics already cover them, so the money stays
            in your main plan.
          </p>
        {:else}
          <p>
            Your rare-event allowance (up to {usd(spend.monthly_usd)} a month) buys
            {#each spend.bought as b, i (i)}{i > 0 ? (i === spend.bought.length - 1 ? ' and ' : ', ') : ''}{lowerFirst(b.item.name)} ({usd(b.item.est_cost_usd)}{planningDate
                ? `, around ${formatMonth(addMonths(planningDate, b.month))}`
                : ''}){#if b.families.length}{' '}for the {b.families.map(familyName).join(' and ')} row{b.families.length === 1 ? '' : 's'}{/if}{/each}.
            {#if spend.families.some((f) => !spend.bought.some((b) => b.families.includes(f)))}
              Your basics already cover the other rows you chose.
            {/if}
          </p>
        {/if}
        {#if onchoose}
          <button type="button" class="button button--small" onclick={onchoose}>Choose what the plan may spend on</button>
        {/if}
      </div>
    {/if}

    <div class="foot">
      <Sources ids={hazards.flatMap(sourcesOf)} />
      {#if backToTable}
        <a
          class="back no-print"
          href="#matrix-{hazards[0]!.id}"
          onclick={(e) => {
            if (jumpTo(`matrix-${hazards[0]!.id}`, { block: 'center' })) e.preventDefault();
          }}>Back to the table<span class="visually-hidden"> of risks</span></a
        >
      {/if}
    </div>
  </section>
{/if}

<style>
  .rare {
    margin-top: var(--s6);
    padding: var(--s4);
    border: 1px solid var(--border);
    border-radius: var(--r3);
    background: var(--surface);
  }
  .rare h2 {
    margin-top: 0;
  }
  .rare-table {
    table-layout: auto;
  }
  .rare-table thead th:nth-child(1) {
    width: 17%;
  }
  .rare-table thead th:nth-child(2) {
    width: 21%;
  }
  .rare-table thead th:nth-child(3) {
    width: 19%;
  }
  .rare-table thead th:nth-child(4),
  .rare-table thead th:nth-child(5) {
    width: 21.5%;
  }
  .th-note {
    font-weight: 400;
  }
  th[scope='row'] {
    background: transparent;
    vertical-align: top;
  }
  td {
    vertical-align: top;
    overflow-wrap: break-word;
  }
  .family + .family .family__row > * {
    border-top: 1px solid var(--border);
  }
  .expander {
    display: inline-flex;
    align-items: flex-start;
    gap: var(--s1);
    min-height: var(--tap);
    padding: var(--s1) 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-weight: 650;
    text-align: left;
    cursor: pointer;
  }
  .expander:hover .expander__name {
    text-decoration: underline;
  }
  .expander__icon {
    display: inline-flex;
    flex: none;
    margin-top: 0.15em;
    transition: transform 0.15s ease;
  }
  .is-open .expander__icon {
    transform: rotate(90deg);
  }
  @media (prefers-reduced-motion: reduce) {
    .expander__icon {
      transition: none;
    }
  }
  .likely {
    display: block;
    font-weight: 600;
  }
  .anchor {
    display: block;
    margin-top: var(--s1);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .tick {
    display: inline-flex;
    margin-right: 0.25em;
    color: var(--good);
    vertical-align: -0.15em;
  }
  .changes--nothing > span:last-child {
    color: var(--good);
    font-weight: 600;
  }
  .cell-label {
    display: none;
  }
  .more > td {
    background: var(--surface-2);
    padding: var(--s3) var(--s4) var(--s4);
  }
  .more__title {
    margin: 0 0 var(--s2);
    font-size: var(--text-base);
  }
  .subrows {
    list-style: none;
    margin: 0 0 var(--s3);
    padding: 0;
    display: grid;
    gap: var(--s2);
  }
  .subrows li {
    padding-left: var(--s3);
    border-left: 3px solid var(--border-strong);
  }
  .subrows li + li {
    margin-top: 0;
  }
  .subrow__name {
    margin: 0;
    font-weight: 650;
  }
  .subrow__range {
    font-weight: 400;
  }
  .subrow__note {
    margin: 0;
    font-size: var(--text-sm);
  }
  .how summary {
    color: var(--accent);
    min-height: 36px;
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .how__body {
    border-left: 3px solid var(--accent-soft);
    padding: var(--s1) 0 var(--s1) var(--s3);
    margin: var(--s1) 0 var(--s2);
    font-size: var(--text-sm);
  }
  .how__body p {
    margin: 0 0 var(--s2);
  }
  .more__foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
    align-items: flex-start;
  }
  .note {
    margin: var(--s2) 0 var(--s3);
  }
  .allowance {
    margin: var(--s4) 0;
    padding: var(--s3) var(--s4);
    border-left: 4px solid var(--accent);
    background: var(--surface-2);
    border-radius: var(--r1);
  }
  .allowance h3 {
    margin: 0 0 var(--s1);
    font-size: var(--text-base);
  }
  .allowance p {
    margin: 0 0 var(--s2);
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
    align-items: flex-start;
  }
  .back {
    display: inline-flex;
    align-items: center;
    min-height: var(--tap);
    font-size: var(--text-sm);
  }

  /* Phones and narrow windows: each family is a block, each cell labelled. */
  @media (max-width: 48rem) {
    .rare {
      padding: var(--s3);
    }
    .rare-table,
    .rare-table thead,
    .rare-table tbody,
    .rare-table tr,
    .rare-table th,
    .rare-table td {
      display: block;
    }
    .rare-table thead {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
      white-space: nowrap;
    }
    .rare-table .family {
      border: 1px solid var(--border);
      border-radius: var(--r2);
      margin-bottom: var(--s3);
      overflow: hidden;
    }
    .family + .family .family__row > * {
      border-top: 0;
    }
    .rare-table th[scope='row'],
    .rare-table td {
      border: 0;
      padding: var(--s1) var(--s3);
    }
    .rare-table th[scope='row'] {
      padding-top: var(--s2);
    }
    .rare-table td:last-child {
      padding-bottom: var(--s3);
    }
    .cell-label {
      display: block;
      font-size: 0.8125rem;
      font-weight: 650;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.03em;
    }
    .more[hidden] {
      display: none;
    }
    .more > td {
      padding: var(--s3);
    }
  }
  @media print {
    .rare {
      border: 0;
      padding: 0;
    }
    .expander {
      color: inherit;
      min-height: 0;
      padding: 0;
    }
    .expander__icon,
    .allowance button,
    .more {
      display: none !important;
    }
    tbody {
      break-inside: avoid;
    }
  }
</style>
