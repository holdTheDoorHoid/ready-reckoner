<!--
  Rare catastrophic hazards in their own box, with how likely and how bad as two separate columns,
  never ranked by expected loss, and with the calm, concrete fact that ordinary supplies cover the
  first days of sheltering.
-->
<script lang="ts">
  import type { HazardProfile } from '../engine/types';
  import { chanceWithin, naturalFrequency } from '../lib/format';
  import ExplainButton from './ExplainButton.svelte';
  import SeveritySwatch from './SeveritySwatch.svelte';
  import Sources from './Sources.svelte';

  let { hazards, years }: { hazards: HazardProfile[]; years: number } = $props();
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
            <th scope="col">How likely <span class="th-note">(households like yours, {years} {years === 1 ? 'year' : 'years'})</span></th>
            <th scope="col">How bad</th>
          </tr>
        </thead>
        <tbody>
          {#each hazards as h (h.id)}
            <tr>
              <th scope="row">{h.name}</th>
              <td>{naturalFrequency(chanceWithin(h.rate_per_year, years))}</td>
              <td><SeveritySwatch severity={h.severity} /></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <ul class="what-to-do">
      {#each hazards as h (h.id)}
        <li><ExplainButton kind="hazard" id={h.id} label="{h.name}: what to do" /></li>
      {/each}
    </ul>
    <Sources ids={hazards.flatMap((h) => h.sources)} />
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
</style>
