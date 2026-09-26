<!--
  Screen 4, Money: the monthly budget (slider with a suggested starting point, labelled as an
  estimate), a one-off amount, months of savings, monthly expenses, how steady income is, and
  insurance. $0 is a fine answer.
-->
<script lang="ts">
  import ChoiceGroup from '../components/ChoiceGroup.svelte';
  import CheckRow from '../components/CheckRow.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import NumberField from '../components/NumberField.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import Sources from '../components/Sources.svelte';
  import { INCOME_STABILITIES } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { usd } from '../lib/format';
  import { STABILITY } from '../lib/labels';
  import { href } from '../lib/router.svelte';

  const app = useApp();
  const input = $derived(app.plan?.input);

  // A suggested starting point, not a survey figure: no survey measures what households spend on
  // preparing (FEMA's 2024 National Household Survey has no spending question), so the marker is
  // labelled an estimate and never presented as what most people do. The one relevant survey
  // finding, that about a quarter name cost as their main barrier, is cited beneath it.
  const STARTING_POINT = 50;
  const COST_BARRIER_SOURCE = 'fema_nhs_2024';
  const SLIDER_MAX = 300;

  const earners = $derived(input?.people.filter((p) => p.earner).length ?? 0);
  const monthly = $derived(input?.finances.monthly_budget_usd ?? 0);
</script>

<div class="page page--narrow">
  <ProgressSteps current="money" />
  <h1 id="page-title" tabindex="-1">Money</h1>
  <p class="lead">Your plan spends only what you set here, month by month, cheapest protection first.</p>

  {#if input}
    <p class="reassure"><strong>$0 is a fine answer.</strong> Your plan then starts with free steps, and they still cover a lot.</p>

    <section aria-labelledby="budget-title">
      <h2 id="budget-title">Budget for preparing</h2>
      <NumberField
        id="monthly-budget"
        label="Each month"
        help="What you could set aside each month for supplies. You can change it any time."
        value={input.finances.monthly_budget_usd}
        prefix="$"
        example="40"
        onchange={(v) => (input.finances.monthly_budget_usd = v ?? 0)}
      />
      <div class="slider">
        <input
          type="range"
          min="0"
          max={SLIDER_MAX}
          step="5"
          aria-label="Monthly budget"
          aria-valuetext="{usd(monthly)} a month"
          value={Math.min(monthly, SLIDER_MAX)}
          oninput={(e) => (input.finances.monthly_budget_usd = Number((e.currentTarget as HTMLInputElement).value))}
        />
        <div class="slider__scale" aria-hidden="true">
          <span>$0</span>
          <span class="slider__typical" style:left="{(STARTING_POINT / SLIDER_MAX) * 100}%">{usd(STARTING_POINT)} (estimate)</span>
          <span>{usd(SLIDER_MAX)}+</span>
        </div>
      </div>
      <p class="small muted typical-note">
        The {usd(STARTING_POINT)} mark is a starting point we suggest. It is an estimate, not a survey figure. Any amount works; a smaller one
        just takes longer.
      </p>
      <p class="small typical-note">
        Cost is a real barrier: in FEMA's 2024 National Household Survey, about 1 in 4 people (26%) said cost was the main thing stopping them
        from preparing. That is why $0 still gets you a plan.
      </p>
      <Sources ids={[COST_BARRIER_SOURCE]} label="Where the survey finding comes from" />
      <NumberField
        id="one-off"
        label="A one-off amount at the start"
        help="Money you could spend once, now. It is added to the first month."
        value={input.finances.one_off_budget_usd}
        prefix="$"
        example="0"
        onchange={(v) => (input.finances.one_off_budget_usd = v ?? 0)}
      />
    </section>

    <section aria-labelledby="savings-title">
      <h2 id="savings-title">Savings and income</h2>
      <NumberField
        id="fund-months"
        label="Months of expenses already saved"
        help="Money you could live on if income stopped, counted in months of normal spending. Use 0 if none."
        value={input.finances.emergency_fund_months}
        suffix="months"
        example="1.5"
        onchange={(v) => (input.finances.emergency_fund_months = v ?? 0)}
      />
      <NumberField
        id="expenses"
        label="Normal monthly expenses"
        help="Used only to size your savings goal. Leave it blank if you'd rather not say."
        value={input.finances.monthly_expenses_usd}
        optional
        prefix="$"
        example="3000"
        width="medium"
        onchange={(v) => {
          if (v === undefined) delete input.finances.monthly_expenses_usd;
          else input.finances.monthly_expenses_usd = v;
        }}
      />
      <p class="earners">
        {#if earners === 0}
          Nobody in the household is marked as earning income, so job loss is not part of your plan.
        {:else}
          {earners === 1 ? '1 person earns' : `${earners} people earn`} income for the household.
        {/if}
        <a href={href('who')}>Change this</a>
      </p>
      {#if earners > 0}
        <ChoiceGroup
          legend="How steady is the income?"
          name="stability"
          help="Less steady income makes a savings buffer matter more."
          options={INCOME_STABILITIES.map((v) => ({ value: v, label: STABILITY[v].label, help: STABILITY[v].help }))}
          bind:value={input.finances.income.stability}
          columns={2}
        />
      {/if}
    </section>

    <section aria-labelledby="insurance-title">
      <h2 id="insurance-title">Insurance</h2>
      <p class="section-intro">Not sure? Leave it unticked. The plan includes checking your policy, which costs nothing.</p>
      <CheckRow label={input.housing.tenure === 'rent' ? 'Renters insurance' : 'Homeowners insurance'} help="Usually pays for repairs or belongings, and often a place to stay." bind:checked={input.finances.insurance.home_or_renters} />
      <CheckRow label="Flood insurance" help="Usually a separate policy." bind:checked={input.finances.insurance.flood} />
      <CheckRow label="Earthquake insurance" help="Usually a separate policy." bind:checked={input.finances.insurance.earthquake} />
    </section>

    <InterviewNav step="money" />
  {/if}
</div>

<style>
  .reassure {
    padding: var(--s3) var(--s4);
    background: var(--good-soft);
    border-radius: var(--r2);
  }
  .slider {
    margin: calc(-1 * var(--s3)) 0 var(--s2);
    max-width: 30rem;
  }
  .slider__scale {
    position: relative;
    display: flex;
    justify-content: space-between;
    font-size: var(--text-sm);
    color: var(--text-muted);
    height: 1.5rem;
  }
  .slider__typical {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    font-weight: 650;
    color: var(--text);
    padding-top: 0.35rem;
    border-top: 2px solid var(--text);
  }
  .typical-note {
    margin-bottom: var(--s1);
  }
  .earners {
    padding: var(--s3) var(--s4);
    background: var(--surface-2);
    border-radius: var(--r2);
  }
</style>
