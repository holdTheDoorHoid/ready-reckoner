<!--
  One hazard: how often it reaches households like this one (natural frequency, with range), how
  bad and how sure, what it does, and what in the plan answers it. Threat and action together.
  The card's id is `hazard-<id>`, the target of its row in the risk matrix; with `backToTable` it
  links back to that row.
-->
<script lang="ts">
  import type { HazardProfile, PlanItem } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import { chanceWithin, CONFIDENCE_LABELS, percent, usd } from '../lib/format';
  import { bucketName, itemSourceIds, keyedItems, lowerFirst } from '../lib/lookup';
  import { href } from '../lib/router.svelte';
  import ExplainButton from './ExplainButton.svelte';
  import IconArray from './IconArray.svelte';
  import SeveritySwatch from './SeveritySwatch.svelte';
  import Sources from './Sources.svelte';

  let {
    hazard,
    featured = false,
    helps = [],
    years,
    backToTable = false,
  }: { hazard: HazardProfile; featured?: boolean; helps?: PlanItem[]; years: number; backToTable?: boolean } = $props();
  const app = useApp();
  const uid = $props.id();

  const chance = $derived(chanceWithin(hazard.rate_per_year, years));
  /** The hazard's own sources, then those behind the prices in "What helps". */
  const sourceIds = $derived([...hazard.sources, ...helps.slice(0, 3).flatMap((i) => (i.kind === 'free_action' ? [] : itemSourceIds(app.catalogue, app.result.output, i.item_id)))]);
  const TIER_WORDS = { natural: 'Nature', societal: 'Society', personal: 'Household' } as const;

  /** "free", "$34", or, for what the household already has or has done, "have it" / "done" rather than a price. */
  function helpNote(item: PlanItem): string {
    if (item.done) return item.kind === 'free_action' ? 'done' : 'have it';
    return item.kind === 'free_action' ? 'free' : usd(item.est_cost_usd);
  }
</script>

<article class="hazard card" class:featured id="hazard-{hazard.id}" aria-labelledby="{uid}-name">
  <header class="hazard__head">
    <h3 id="{uid}-name">{hazard.name}</h3>
    <span class="chip">{TIER_WORDS[hazard.tier]}</span>
  </header>
  <p class="hazard__freq">{hazard.frequency_sentence}</p>
  <div class="hazard__main">
    {#if featured}
      <IconArray probability={chance} label="{hazard.frequency_sentence} Each square is one household." />
    {/if}
    <dl class="facts" class:facts--stacked={featured}>
      <div><dt>How bad</dt><dd><SeveritySwatch severity={hazard.severity} /></dd></div>
      <div><dt>How sure</dt><dd>{CONFIDENCE_LABELS[hazard.confidence]}</dd></div>
      {#if hazard.climate_multiplier !== 1}
        <div>
          <dt>Around 2050</dt>
          <dd>{hazard.climate_multiplier > 1 ? `about ${Math.round((hazard.climate_multiplier - 1) * 100)}% more often` : `about ${Math.round((1 - hazard.climate_multiplier) * 100)}% less often`} (projection)</dd>
        </div>
      {/if}
    </dl>
  </div>
  {#if app.prefs.expert}
    <p class="expert small">
      Expert view: {percent(chance)} in {years} {years === 1 ? 'year' : 'years'}; {hazard.rate_per_year} events a year ({hazard.rate_range[0]}–{hazard.rate_range[1]}){#if hazard.eal_per_household_usd !== undefined}; expected loss about {usd(hazard.eal_per_household_usd)} a year{/if}.
    </p>
  {/if}
  {#if hazard.buckets.length}
    <p class="small"><strong>What it can do:</strong> {hazard.buckets.map((b) => lowerFirst(bucketName(app.catalogue, b))).join('; ')}.</p>
  {/if}
  {#if helps.length}
    <p class="small helps">
      <strong>What helps:</strong>
      {#each keyedItems(helps.slice(0, 3)) as { key, item }, i (key)}{i > 0 ? ', ' : ''}{lowerFirst(item.name)} ({helpNote(item)}){/each}.
      <a href={href('plan')}>See it in your plan</a>
    </p>
  {/if}
  <footer class="hazard__foot">
    <ExplainButton kind="hazard" id={hazard.id} />
    <Sources ids={sourceIds} what={hazard.name} />
    {#if backToTable}
      <a
        class="back no-print"
        href="#matrix-{hazard.id}"
        onclick={(e) => {
          if (jumpTo(`matrix-${hazard.id}`, { block: 'center' })) e.preventDefault();
        }}>Back to the table<span class="visually-hidden"> of risks</span></a
      >
    {/if}
  </footer>
</article>

<style>
  .hazard {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .hazard__head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--s2);
  }
  .hazard__head h3 {
    margin: 0;
  }
  .hazard__main {
    display: flex;
    gap: var(--s4);
    align-items: center;
  }
  .hazard__freq {
    margin: 0;
  }
  .featured .hazard__freq {
    font-size: var(--text-lg);
    font-weight: 560;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s5);
    margin: 0;
    font-size: var(--text-sm);
  }
  .facts div {
    display: flex;
    gap: var(--s2);
  }
  .facts--stacked {
    flex-direction: column;
    gap: var(--s2);
  }
  .facts--stacked div {
    flex-direction: column;
    gap: 0;
  }
  .facts dt {
    color: var(--text-muted);
  }
  .facts dd {
    margin: 0;
    font-weight: 600;
  }
  .helps {
    background: var(--good-soft);
    border-radius: var(--r1);
    padding: var(--s2) var(--s3);
  }
  .expert {
    color: var(--text-muted);
    margin-bottom: var(--s2);
  }
  .hazard__foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
    align-items: flex-start;
    margin-top: auto;
  }
  .back {
    display: inline-flex;
    align-items: center;
    min-height: var(--tap);
    font-size: var(--text-sm);
  }
</style>
