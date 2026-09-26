<!--
  The risk matrix (owner request, 2026-09-26): every hazard in one plain table, most likely first,
  so the whole register reads at a glance before the cards. One row each: rank and name (a link to
  the hazard's card, which opens "All N risks" first when the card is folded away there), how likely
  (the chance over the chosen years, worded as the card words it, with how often a year below), how
  bad and how sure. The rare-but-severe rows follow under a divider: a range only, never ranked,
  each linking to the rare box. Four columns at every width; long names wrap.
-->
<script lang="ts">
  import type { HazardProfile } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { chanceShort, chanceWithin, CONFIDENCE_LABELS, perYearWords, rangeOnly } from '../lib/format';
  import { NUCLEAR_NOTE } from '../lib/labels';
  import Chance from './Chance.svelte';
  import SeveritySwatch from './SeveritySwatch.svelte';

  let { ranked, rare, years }: { ranked: HazardProfile[]; rare: HazardProfile[]; years: number } = $props();

  const span = $derived(years === 1 ? '1 year' : `${years} years`);

  /** Expert estimates carry their range, as the card's sentence does (CONTENT_STANDARDS §6). */
  function likely(h: HazardProfile): string {
    const p = chanceWithin(h.rate_per_year, years);
    if (h.confidence !== 'prior') return chanceShort(p);
    return chanceShort(p, chanceWithin(h.rate_range[0], years), chanceWithin(h.rate_range[1], years));
  }

  function go(e: MouseEvent, id: string, focus?: string) {
    if (jumpTo(id, { focus })) e.preventDefault();
  }
</script>

<section class="matrix" aria-labelledby="matrix-title">
  <h2 id="matrix-title">Your risks at a glance</h2>
  <p class="section-intro">
    Everything checked for your household, most likely first. Chances are for households like yours in your county, in the next {span}. Pick a
    name to see what it can do and what helps.
  </p>
  <table>
    <caption class="visually-hidden">Your risks, most likely first: how likely in the next {span}, how bad and how sure</caption>
    <thead>
      <tr>
        <th scope="col">Risk</th>
        <th scope="col">How likely <span class="th-note">({span})</span></th>
        <th scope="col">How bad</th>
        <th scope="col">How sure</th>
      </tr>
    </thead>
    <tbody>
      {#each ranked as h, i (h.id)}
        <tr>
          <th scope="row">
            <span class="rank">{i + 1}.</span>
            <a id="matrix-{h.id}" href="#hazard-{h.id}" onclick={(e) => go(e, `hazard-${h.id}`, 'h3')}>{h.name}</a>
          </th>
          <td>
            <Chance text={likely(h)} /><span class="per-year"
              ><span class="visually-hidden">{'; '}</span><Chance text={perYearWords(h.rate_per_year, h.annual_probability)} /></span
            >
          </td>
          <td><SeveritySwatch severity={h.severity} /></td>
          <td>{CONFIDENCE_LABELS[h.confidence]}</td>
        </tr>
      {/each}
    </tbody>
    {#if rare.length}
      <tbody class="rare">
        <tr>
          <th scope="rowgroup" colspan="4" class="divider">Rare but severe: expert estimates, shown as a range and never ranked</th>
        </tr>
        {#each rare as h (h.id)}
          <tr>
            <th scope="row"><a id="matrix-{h.id}" href="#rare-title" onclick={(e) => go(e, 'rare-title')}>{h.name}</a></th>
            <td><Chance text={rangeOnly(h.rate_range[0], h.rate_range[1], years)} /></td>
            <td><SeveritySwatch severity={h.severity} /></td>
            <td>{CONFIDENCE_LABELS[h.confidence]}</td>
          </tr>
        {/each}
      </tbody>
    {/if}
  </table>
  {#if rare.some((h) => h.id === 'nuclear_attack')}
    <p class="small muted note">{NUCLEAR_NOTE}</p>
  {/if}
</section>

<style>
  .matrix {
    margin-top: var(--s5);
  }
  .matrix h2 {
    margin: 0 0 var(--s2);
    font-size: var(--text-lg);
  }
  table {
    table-layout: auto;
  }
  /* Words wrap whole; a word breaks only if it cannot fit its column at all. */
  th,
  td {
    overflow-wrap: break-word;
  }
  thead th {
    white-space: normal;
  }
  .th-note {
    font-weight: 400;
  }
  tbody th[scope='row'] {
    font-weight: 600;
  }
  .rank {
    display: inline-block;
    min-width: 1.6em;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }
  .per-year {
    display: block;
    color: var(--text-muted);
  }
  /* The swatch's word may wrap under it in a narrow column. */
  td :global(.severity) {
    white-space: normal;
  }
  .divider {
    background: var(--surface-2);
    font-weight: 650;
    border-top: 2px solid var(--border-strong);
  }
  .note {
    margin-top: var(--s2);
  }
  /* Phones: four columns still, with tighter cells and the severity swatch above its word. */
  @media (max-width: 30rem) {
    th,
    td {
      padding: 0.4rem 0.25rem;
    }
    .rank {
      min-width: 1.4em;
    }
    td :global(.severity) {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.2em;
    }
  }
  /* The smallest phones (360 px and less): slightly smaller type keeps every word whole. */
  @media (max-width: 23rem) {
    table {
      font-size: 0.8125rem;
    }
  }
  @media print {
    .matrix a {
      text-decoration: none;
    }
    tr {
      break-inside: avoid;
    }
  }
</style>
