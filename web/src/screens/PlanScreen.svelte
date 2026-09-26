<!--
  Screen 7, Your plan: phased purchases and actions by month, this month first and free steps
  first. Each item can be checked off with what was paid. Progress per consequence, guardrail
  warnings (never blocks), money being saved toward bigger items, and the whole schedule.
-->
<script lang="ts">
  import BucketGauge from '../components/BucketGauge.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import ItemCard from '../components/ItemCard.svelte';
  import PlanGate from '../components/PlanGate.svelte';
  import SavingsTrack from '../components/SavingsTrack.svelte';
  import Warning from '../components/Warning.svelte';
  import type { PlanItem, PlanOutput } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { addMonths, formatDate, formatMonth, monthsBetween, quantity, usd } from '../lib/format';
  import { CONFIDENCE_QUESTION, CONFIDENCE_SCALE, stageLine } from '../lib/labels';
  import { allPlanItems, bucketName, catalogueItem, tierName } from '../lib/lookup';
  import { href } from '../lib/router.svelte';

  const app = useApp();
  let confirmRestart = $state(false);

  const planningDate = $derived(app.plan?.input.planning_date ?? app.today());
  const currentMonth = $derived(Math.max(0, monthsBetween(planningDate, app.today())));

  const GROUPS: { name: string; categories: string[] }[] = [
    { name: 'Safety at home', categories: ['fire', 'home', 'security'] },
    { name: 'Plans and papers', categories: ['plan', 'documents', 'comms', 'money'] },
    { name: 'Water, food and medicine', categories: ['water', 'food', 'medical', 'thermal', 'animals'] },
    { name: 'Getting around', categories: ['transport'] },
    { name: 'Neighbours', categories: ['community'] },
  ];

  function groupFree(items: PlanItem[]): { name: string; items: PlanItem[] }[] {
    const out = GROUPS.map((g) => ({
      name: g.name,
      items: items.filter((i) => g.categories.includes(catalogueItem(app.catalogue, i.item_id)?.category ?? '')),
    }));
    const placed = new Set(out.flatMap((g) => g.items));
    const rest = items.filter((i) => !placed.has(i));
    if (rest.length) out.push({ name: 'Other', items: rest });
    return out.filter((g) => g.items.length);
  }

  function monthLabel(index: number): string {
    if (index === currentMonth) return 'This month';
    if (index === currentMonth + 1) return 'Next month';
    return `Month ${index + 1}: ${formatMonth(addMonths(planningDate, index))}`;
  }

  function spend(items: PlanItem[]): number {
    return items.filter((i) => !i.done).reduce((s, i) => s + i.est_cost_usd, 0);
  }

  function counts(output: PlanOutput) {
    const all = allPlanItems(output).map((x) => x.item);
    return { done: all.filter((i) => i.done).length, total: all.length };
  }

  function restart() {
    if (app.plan) app.plan.input.planning_date = app.today();
  }
</script>

<div class="page">
  <h1 id="page-title" tabindex="-1">Your plan</h1>
  <PlanGate>
    {#snippet children(output)}
      {@const months = output.plan.months}
      {@const earlier = months.filter((m) => m.index < currentMonth).flatMap((m) => m.items.filter((i) => !i.done))}
      {@const thisMonth = months.find((m) => m.index === currentMonth)}
      {@const thisItems = (thisMonth?.items ?? []).filter((i) => !i.done)}
      {@const nextMonth = months.find((m) => m.index > currentMonth)}
      {@const done = allPlanItems(output).filter((x) => x.item.done).map((x) => x.item)}
      {@const c = counts(output)}
      {@const monthly = app.plan?.input.finances.monthly_budget_usd ?? 0}
      <p class="lead">{stageLine(app.plan?.input.stage)} Month by month: free steps first, then the cheapest protection for your risks, within your budget.</p>

      <ul class="stats" aria-label="Summary">
        <li class="card">
          <span class="stat__label">This month</span>
          <span class="stat__value">{monthly > 0 || (thisMonth?.budget_usd ?? 0) > 0 ? `${usd(thisMonth?.budget_usd ?? monthly)} to spend` : 'Free steps only'}</span>
        </li>
        <li class="card">
          <span class="stat__label">Done so far</span>
          <span class="stat__value">{c.done} of {c.total} steps</span>
        </li>
        <li class="card">
          <span class="stat__label">Every target covered</span>
          <span class="stat__value">
            {#if output.plan.done_month !== undefined}
              by {formatMonth(addMonths(planningDate, output.plan.done_month))}
            {:else if monthly <= 0 && (app.plan?.input.finances.one_off_budget_usd ?? 0) <= 0}
              once you add a budget
            {:else}
              in more than three years
            {/if}
          </span>
        </li>
      </ul>

      {#if output.warnings.length}
        <section aria-label="Things to look at">
          {#each output.warnings as w (w.id)}<Warning warning={w} />{/each}
        </section>
      {/if}

      {#if earlier.length}
        <section aria-labelledby="earlier-title">
          <h2 id="earlier-title">Still to do from earlier months</h2>
          <p class="section-intro">Plans slip; that is normal. These still help, in this order.</p>
          {#each earlier as item (item.item_id + item.tier)}<ItemCard {item} />{/each}
        </section>
      {/if}

      <section aria-labelledby="this-title">
        <h2 id="this-title">This month <span class="muted h-note">from {formatDate(addMonths(planningDate, currentMonth))}</span></h2>
        {#if thisItems.length === 0}
          <p class="card">Nothing new to do this month{output.plan.envelopes.length ? '; your budget is saving toward a bigger item below' : ''}. {nextMonth ? 'The next step is shown below.' : ''}</p>
        {:else}
          {@const free = thisItems.filter((i) => i.kind === 'free_action')}
          {@const buy = thisItems.filter((i) => i.kind !== 'free_action')}
          {#if free.length}
            <h3>Free steps ({free.length})</h3>
            <p class="section-intro">These cost nothing and cover more than you might expect. Do them in any order.</p>
            {#each groupFree(free) as group (group.name)}
              <h4 class="group">{group.name}</h4>
              {#each group.items as item (item.item_id + item.tier)}<ItemCard {item} compact />{/each}
            {/each}
          {/if}
          {#if buy.length}
            <h3>To get <span class="muted h-note">about {usd(spend(buy))} of {usd(thisMonth?.budget_usd ?? 0)}</span></h3>
            {#each buy as item (item.item_id + item.tier)}<ItemCard {item} />{/each}
          {/if}
        {/if}
      </section>

      {#if nextMonth && nextMonth.index !== currentMonth}
        <section aria-labelledby="next-title">
          <h2 id="next-title">{monthLabel(nextMonth.index)} <span class="muted h-note">from {formatDate(addMonths(planningDate, nextMonth.index))}</span></h2>
          <ul class="preview">
            {#each nextMonth.items.filter((i) => !i.done) as item (item.item_id + item.tier)}
              <li>{item.name}: {item.kind === 'free_action' ? 'free' : `${quantity(item.quantity, item.unit)}, about ${usd(item.est_cost_usd)}`}</li>
            {/each}
          </ul>
        </section>
      {/if}

      <section aria-labelledby="progress-title">
        <h2 id="progress-title">Progress by consequence</h2>
        <p class="section-intro">What you have now against each target. Checking items off moves these.</p>
        <div class="progress card">
          {#each output.buckets.filter((b) => b.target.kind === 'days') as b (b.id)}<BucketGauge bucket={b} compact />{/each}
        </div>
        <p class="small muted">
          Your risks point to <strong>{tierName(app.catalogue, output.tier_recommended).toLowerCase()}</strong> of supplies.
          {#if output.tier_reached !== 'now'}You have reached <strong>{tierName(app.catalogue, output.tier_reached).toLowerCase()}</strong>.{/if}
        </p>
      </section>

      {#if output.plan.envelopes.length}
        <section aria-labelledby="saving-title">
          <h2 id="saving-title">Saving up for</h2>
          <p class="section-intro">These cost more than a month's budget, so the plan sets money aside for them over a few months.</p>
          <ul class="envelopes">
            {#each output.plan.envelopes as e, i (i)}
              {@const month = months.find((m) => m.items.some((x) => x.item_id === e.item_id && !x.done && Math.abs(x.est_cost_usd - e.needed_usd) < 0.01))}
              <li class="card">
                <strong>{catalogueItem(app.catalogue, e.item_id)?.name ?? e.item_id}</strong>: about {usd(e.needed_usd)}
                {#if month}; ready to buy around {formatMonth(addMonths(planningDate, month.index))}{/if}.
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <section aria-labelledby="all-title">
        <h2 id="all-title">The whole plan</h2>
        {#each months as m (m.index)}
          {@const open = m.items.filter((i) => !i.done)}
          {#if open.length}
            <details class="month">
              <summary>
                <span>{monthLabel(m.index)}</span>
                <span class="muted small">{open.length} {open.length === 1 ? 'step' : 'steps'}{spend(open) > 0 ? `, about ${usd(spend(open))}` : ''}</span>
              </summary>
              <ul class="month__list">
                {#each open as item (item.item_id + item.tier)}
                  <li>
                    <span>{item.name}</span>
                    <span class="muted">{item.kind === 'free_action' ? 'free' : `${quantity(item.quantity, item.unit)}, about ${usd(item.est_cost_usd)}`}</span>
                    <span class="chip">{bucketName(app.catalogue, item.buckets[0] ?? '')}</span>
                  </li>
                {/each}
              </ul>
            </details>
          {/if}
        {/each}
        {#if output.plan.done_month !== undefined}
          <p class="done-note">
            <Icon name="check" /> After {formatMonth(addMonths(planningDate, output.plan.done_month))} you are done for your risk. From then on, keeping it up takes a few minutes a month.
            <a href={href('maintain')}>See what to keep up</a>
          </p>
        {/if}
      </section>

      {#if done.length}
        <section aria-labelledby="done-title">
          <h2 id="done-title">Done <span class="muted h-note">{done.length}</span></h2>
          <details>
            <summary>Show what you've done</summary>
            {#each done as item (item.item_id + item.tier)}<ItemCard {item} />{/each}
          </details>
        </section>
      {/if}

      <SavingsTrack
        income={output.buckets.find((b) => b.id === 'income')}
        track={output.plan.savings_track}
        homeLoss={undefined}
        homeItems={[]}
      />

      {#if app.plan}
        <section class="confidence card" aria-labelledby="conf-title">
          <h2 id="conf-title">How sure do you feel now?</h2>
          <fieldset>
            <legend>{CONFIDENCE_QUESTION}</legend>
            {#if app.plan.confidence.before !== undefined}
              <p class="help">When you started you said {app.plan.confidence.before} out of 5.</p>
            {/if}
            <div class="scale">
              {#each [1, 2, 3, 4, 5] as n (n)}
                <label class="choice">
                  <input
                    type="radio"
                    name="confidence-after"
                    checked={app.plan.confidence.after === n}
                    onchange={() => {
                      if (app.plan) app.plan.confidence.after = n;
                    }}
                  />
                  <span class="choice__text"><strong>{n}</strong><span class="choice__help">{CONFIDENCE_SCALE[n]}</span></span>
                </label>
              {/each}
            </div>
          </fieldset>
          {#if app.plan.confidence.after !== undefined && app.plan.confidence.before !== undefined && app.plan.confidence.after > app.plan.confidence.before}
            <p class="good"><Icon name="check" /> Up from {app.plan.confidence.before}. Each step you take counts.</p>
          {/if}
        </section>
      {/if}

      {#if app.prefs.expert}
        <section aria-labelledby="req-title">
          <h2 id="req-title">What the targets turn into (expert view)</h2>
          <!-- Wide tables scroll sideways on phones; a focusable, labelled region lets keyboard users scroll it. -->
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <div class="table-wrap" tabindex="0" role="region" aria-label="What the targets turn into">
            <table>
              <thead><tr><th scope="col">For</th><th scope="col">Amount</th><th scope="col">How it was worked out</th><th scope="col">Rule</th></tr></thead>
              <tbody>
                {#each output.requirements as r (r.id)}
                  <tr><td>{bucketName(app.catalogue, r.bucket)}</td><td class="num">{quantity(r.quantity, r.unit)}</td><td>{r.plain}</td><td><code>{r.rule}</code></td></tr>
                {/each}
              </tbody>
            </table>
          </div>
        </section>
      {/if}

      <section class="schedule small muted" aria-label="Schedule">
        <p>
          Your plan started on {formatDate(planningDate)}.
          <button type="button" class="button button--quiet button--small" onclick={() => (confirmRestart = true)}>Restart the schedule from today</button>
        </p>
      </section>
    {/snippet}
  </PlanGate>
</div>

<ConfirmDialog bind:open={confirmRestart} title="Restart the schedule from today?" confirmLabel="Restart from today" cancelLabel="Keep the schedule" onconfirm={restart}>
  <p>Month 1 becomes this month. Nothing you have checked off is lost.</p>
  <p>If you already spent the one-off amount, set it to $0 on the Money screen so it isn't counted again.</p>
</ConfirmDialog>

<style>
  .stats {
    list-style: none;
    padding: 0;
    margin: var(--s4) 0 var(--s5);
    display: grid;
    gap: var(--s3);
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 14rem), 1fr));
  }
  .stats li + li {
    margin-top: 0;
  }
  .stats .card {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    padding: var(--s3) var(--s4);
  }
  .stat__label {
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-weight: 600;
  }
  .stat__value {
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .h-note {
    font-size: var(--text-base);
    font-weight: 500;
  }
  .group {
    margin: var(--s4) 0 0;
    font-size: var(--text-sm);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .preview li + li {
    margin-top: var(--s1);
  }
  .progress {
    padding-top: var(--s2);
    padding-bottom: var(--s2);
  }
  .envelopes {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--s2);
  }
  .envelopes li + li {
    margin-top: 0;
  }
  .month summary {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s3);
    border-bottom: 1px solid var(--border);
  }
  .month__list {
    list-style: none;
    padding: var(--s2) 0 var(--s3);
    margin: 0;
  }
  .month__list li {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s3);
    align-items: center;
    padding: var(--s1) 0;
  }
  .done-note,
  .good {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    color: var(--good);
    font-weight: 600;
  }
  .done-note a {
    font-weight: 600;
  }
  .confidence {
    margin-top: var(--s6);
  }
  .confidence h2 {
    margin-top: 0;
  }
  .scale {
    display: grid;
    gap: var(--s2);
    grid-template-columns: repeat(auto-fit, minmax(7rem, 1fr));
  }
  .schedule {
    margin-top: var(--s6);
  }
</style>
