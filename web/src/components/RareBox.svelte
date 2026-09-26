<!--
  Rare catastrophic hazards in their own box, with how likely and how bad as two separate columns,
  never ranked by expected loss, and with the calm, concrete fact that ordinary supplies cover the
  first days of sheltering. How likely is a range only (H-02): these are expert estimates, and a
  single number would claim a precision nobody has. The nuclear row is a worldwide figure, and the
  note under the table says so.
-->
<script lang="ts">
  import type { HazardProfile } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { rangeOnly } from '../lib/format';
  import { NUCLEAR_NOTE } from '../lib/labels';
  import ExplainButton from './ExplainButton.svelte';
  import SeveritySwatch from './SeveritySwatch.svelte';
  import Sources from './Sources.svelte';

  let { hazards, years, backToTable = false }: { hazards: HazardProfile[]; years: number; backToTable?: boolean } = $props();
</script>

{#if hazards.length}
  <section class="rare" aria-labelledby="rare-title">
    <h2 id="rare-title">Rare but severe</h2>
    <p class="section-intro">
      These are very unlikely. They sit apart from the list above so that a tiny chance of a huge loss does not crowd out what is likely.
      The plan never spends on them by default. Your three-day supplies already cover the first days of sheltering: get inside, stay inside,
      stay tuned.
    </p>
    <!-- Wide tables scroll sideways on phones; a focusable, labelled region lets keyboard users scroll it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="table-wrap" tabindex="0" role="region" aria-label="Rare but severe hazards">
      <table>
        <caption class="visually-hidden">Rare but severe hazards: how likely and how bad</caption>
        <thead>
          <tr>
            <th scope="col">What</th>
            <th scope="col">How likely <span class="th-note">(in the next {years === 1 ? 'year' : `${years} years`})</span></th>
            <th scope="col">How bad</th>
          </tr>
        </thead>
        <tbody>
          {#each hazards as h (h.id)}
            <tr>
              <th scope="row">{h.name}</th>
              <td>{rangeOnly(h.rate_range[0], h.rate_range[1], years)}</td>
              <td><SeveritySwatch severity={h.severity} /></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if hazards.some((h) => h.id === 'nuclear_attack')}
      <p class="small note">{NUCLEAR_NOTE}</p>
    {/if}
    <ul class="what-to-do">
      {#each hazards as h (h.id)}
        <li><ExplainButton kind="hazard" id={h.id} label="{h.name}: what to do" /></li>
      {/each}
    </ul>
    <div class="foot">
      <Sources ids={hazards.flatMap((h) => h.sources)} />
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
  th[scope='row'] {
    background: transparent;
  }
  .th-note {
    font-weight: 400;
  }
  .what-to-do {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s2);
  }
  .what-to-do li + li {
    margin-top: 0;
  }
  .note {
    margin: 0 0 var(--s3);
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
</style>
