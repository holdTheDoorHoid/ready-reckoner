<!--
  The money buckets, kept visibly apart from the supplies budget: the emergency-savings goal for
  lost income, and the home-loss checklist (insurance and papers). A long goal gets a nearer one
  first (one month of expenses, or three once one is saved), dated from the same money the engine
  suggests (the supplies budget, once the supplies plan is done); the full goal stays beside it.
-->
<script lang="ts">
  import type { BucketAssessment, IsoDate, PlanItem, SavingsTrack } from '../engine/types';
  import { nextMilestone } from '../lib/savings';
  import { formatMonth, monthsPhrase, targetMonths, usd } from '../lib/format';
  import ExplainButton from './ExplainButton.svelte';
  import ReadinessCard from './ReadinessCard.svelte';
  import Sources from './Sources.svelte';

  let {
    income,
    track,
    homeLoss,
    homeItems,
    planningDate = undefined,
    doneMonth = undefined,
  }: {
    income: BucketAssessment | undefined;
    track: SavingsTrack | undefined;
    homeLoss: BucketAssessment | undefined;
    homeItems: PlanItem[];
    /** The plan's first day, to date the nearer goal. */
    planningDate?: IsoDate;
    /** The month the supplies plan is done (`Plan.done_month`), when the saving can start. */
    doneMonth?: number;
  } = $props();

  const upperFirst = (t: string) => t.charAt(0).toUpperCase() + t.slice(1);
  const pct = $derived(track && track.target_months > 0 ? Math.min(100, (track.current_months / track.target_months) * 100) : 0);
  const milestone = $derived(track ? nextMilestone(track, planningDate, doneMonth) : null);
</script>

<section class="savings" aria-labelledby="savings-title">
  <div class="savings__head">
    <h2 id="savings-title">Savings track</h2>
    <p class="savings__badge">Separate from your supplies budget</p>
  </div>
  <p class="section-intro">
    Losing income is the most likely long disruption for most households. It is met with savings over time, not with things you buy, so it
    never takes money from your monthly supplies budget.
  </p>
  <div class="grid">
    {#if income && income.target.kind === 'months' && track}
      <article class="card" aria-labelledby="income-title" data-bucket={income.id} data-target={JSON.stringify(income.target)}>
        <h3 id="income-title">{income.name}</h3>
        {#if milestone}
          <p class="milestone">
            <strong>{milestone.first ? 'First goal' : 'Next goal'}:</strong>
            {milestone.months === 1 ? 'one month' : 'three months'} of expenses{milestone.usd !== undefined ? `, about ${usd(milestone.usd)}` : ''}{milestone.by
              ? `, by ${formatMonth(milestone.by)}`
              : ''}.
          </p>
        {/if}
        <p>
          {#if milestone}<strong>Full goal:</strong>{/if}
          <span class="big">{targetMonths(income.target.value, income.target.low, income.target.high)}</span> of expenses{track.target_usd > 0 ? `, about ${usd(track.target_usd)}` : ''}.
        </p>
        <div
          class="meter"
          role="meter"
          aria-label="Savings so far"
          aria-valuemin="0"
          aria-valuemax={track.target_months}
          aria-valuenow={Math.min(track.current_months, track.target_months)}
          aria-valuetext="{monthsPhrase(track.current_months)} saved of {monthsPhrase(track.target_months)}"
        >
          <div class="meter__fill" style:width="{pct}%"></div>
        </div>
        <p class="small"><strong>{upperFirst(monthsPhrase(track.current_months))}</strong> saved so far.</p>
        <p class="small">{track.why}</p>
        <footer class="foot">
          <ExplainButton kind="bucket" id="income" />
          <Sources ids={income.sources} />
        </footer>
      </article>
    {/if}
    {#if homeLoss}
      <ReadinessCard bucket={homeLoss} items={homeItems} />
    {/if}
  </div>
</section>

<style>
  .savings {
    margin-top: var(--s7);
    padding: var(--s5) var(--s4);
    border: 2px dashed var(--border-strong);
    border-radius: var(--r3);
    background: var(--surface-2);
  }
  .savings__head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2) var(--s4);
  }
  .savings__head h2 {
    margin: 0;
  }
  .savings__badge {
    margin: 0;
    font-weight: 650;
    font-size: var(--text-sm);
    padding: 0.15em 0.7em;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }
  .big {
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .milestone {
    margin: 0 0 var(--s2);
    padding: var(--s2) var(--s3);
    border-left: 4px solid var(--accent);
    background: var(--surface);
    border-radius: var(--r1);
  }
  .meter {
    height: 0.75rem;
    border-radius: 999px;
    background: var(--gauge-track);
    overflow: hidden;
    border: 1px solid var(--border);
    margin-bottom: var(--s2);
  }
  .meter__fill {
    height: 100%;
    background: var(--gauge-fill);
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
  }
</style>
