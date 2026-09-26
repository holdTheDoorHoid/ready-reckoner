<!--
  One plan item: what it is and how much, the cost and price band, why it is here (which
  consequences, which hazards), what to look for and avoid, a check-off, and what was paid.
-->
<script lang="ts">
  import type { PlanItem } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { band, formatDate, quantity, usd } from '../lib/format';
  import { bucketName, catalogueItem, hazardName, itemSourceIds, requirementsFor } from '../lib/lookup';
  import { intervalLabel } from '../lib/maintenance';
  import ExplainButton from './ExplainButton.svelte';
  import Icon from './Icon.svelte';
  import NumberField from './NumberField.svelte';
  import Sources from './Sources.svelte';

  let { item, compact = false }: { item: PlanItem; compact?: boolean } = $props();
  const app = useApp();
  const uid = $props.id();

  const info = $derived(catalogueItem(app.catalogue, item.item_id));
  /** How the quantity was worked out (the requirement lines behind it), and every source behind the numbers. */
  const workings = $derived(item.kind === 'free_action' ? [] : requirementsFor(app.catalogue, app.result.output, item.item_id));
  const sourceIds = $derived(itemSourceIds(app.catalogue, app.result.output, item.item_id));
  const purchase = $derived(app.purchaseFor(item.item_id, item.tier));
  const fromInventory = $derived(!!item.done && !purchase);
  const verb = $derived(item.kind === 'free_action' ? 'Done' : item.kind === 'reserve' ? 'Set aside' : 'Bought');
  const perUnit = $derived.by(() => {
    if (!info || item.kind === 'free_action' || item.quantity <= 0) return '';
    const each = item.est_cost_usd / item.quantity;
    if (info.energy_kcal_per_unit) return `about ${usd((each / info.energy_kcal_per_unit) * 2000)} per 2,000 kcal`;
    if (info.volume_l_per_unit) return `about ${usd((each / info.volume_l_per_unit) * 3.785)} per gallon`;
    return '';
  });

  function toggle(checked: boolean) {
    if (checked) app.record(item);
    else app.unrecord(item.item_id, item.tier);
  }
</script>

<article class="item" class:is-done={item.done} class:compact aria-labelledby="{uid}-name">
  <div class="item__check">
    <input
      id="{uid}-check"
      type="checkbox"
      checked={!!item.done}
      disabled={fromInventory}
      aria-describedby="{uid}-qty"
      onchange={(e) => toggle((e.currentTarget as HTMLInputElement).checked)}
    />
    <label for="{uid}-check">{verb}<span class="visually-hidden">: {item.name}</span></label>
  </div>
  <div class="item__body">
    <h4 id="{uid}-name" class="item__name">{item.name}</h4>
    <p class="item__qty" id="{uid}-qty">
      {#if item.kind === 'free_action'}
        {#if !compact}<span class="chip chip--accent">Free</span>{/if}
      {:else}
        <strong>{quantity(item.quantity, item.unit)}</strong>
        <span>· about {usd(item.est_cost_usd)}</span>
        <span class="muted">({band(item.price_band.low, item.price_band.high)})</span>
      {/if}
      {#if item.done}
        <span class="done-mark"><Icon name="check" /> {item.kind === 'free_action' ? 'Done' : 'Have it'}</span>
      {/if}
    </p>
    <p class="item__why">{item.why}</p>
    {#if fromInventory}
      <p class="small muted">You already had this. Change it on the "What you already have" screen.</p>
    {/if}
    {#if purchase && item.kind !== 'free_action'}
      <div class="item__paid">
        <NumberField
          id="{uid}-paid"
          label="What you paid (optional)"
          help="Your real price replaces the estimate in the budget."
          value={purchase.paid_usd}
          optional
          prefix="$"
          example="25"
          onchange={(v) => app.setPaid(item.item_id, item.tier, v)}
        />
      </div>
    {/if}
    <details class="item__more">
      <summary>{compact ? 'More about this' : 'What to look for, and why'}</summary>
      {#if info}
        <p>{info.spec}</p>
        {#if info.look_for.length}
          <p class="sub">Look for</p>
          <ul>{#each info.look_for as line (line)}<li>{line}</li>{/each}</ul>
        {/if}
        {#if info.avoid.length}
          <p class="sub">Avoid</p>
          <ul>{#each info.avoid as line (line)}<li>{line}</li>{/each}</ul>
        {/if}
        {#if !info.free}
          <p class="small">
            Typical price: {band(info.price_band_usd.low, info.price_band_usd.high)} per {info.price_band_usd.per}{info.price_band_usd.note ? `; ${info.price_band_usd.note}` : ''}.
            {#if perUnit}That is {perUnit}.{/if}
            {#if info.retrieved}Prices as of {formatDate(info.retrieved)}.{/if}
          </p>
        {/if}
        {#if info.maintenance?.rotate_months}<p class="small">Use and replace: {intervalLabel(info.maintenance.rotate_months).toLowerCase()}.</p>{/if}
        {#if info.maintenance?.check_months}<p class="small">Check: {intervalLabel(info.maintenance.check_months).toLowerCase()}.</p>{/if}
      {/if}
      <p class="small"><span class="sub-inline">Helps with:</span> {item.buckets.map((b) => bucketName(app.catalogue, b)).join('; ')}.</p>
      {#if item.hazards.length}
        <p class="small"><span class="sub-inline">Hazards it answers:</span> {item.hazards.slice(0, 4).map((h) => hazardName(app.catalogue, h)).join('; ')}.</p>
      {/if}
      {#if workings.length}
        <p class="sub">How the amount is worked out</p>
        <ul>{#each workings as r (r.id)}<li>{r.plain}</li>{/each}</ul>
      {/if}
      <Sources ids={sourceIds} label="Sources for the amount and the price" what={item.name} />
      <ExplainButton kind="item" id={item.item_id} label="How the plan chose this" />
    </details>
  </div>
</article>

<style>
  .item {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--s3);
    padding: var(--s4) 0;
    border-top: 1px solid var(--border);
  }
  .item__check {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s1);
    min-width: 4.25rem;
    padding-top: 0.2rem;
  }
  .item__check input {
    width: 1.6rem;
    height: 1.6rem;
    cursor: pointer;
  }
  .item__check label {
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    text-align: center;
  }
  .item__name {
    margin: 0 0 var(--s1);
    font-size: var(--text-lg);
  }
  .is-done .item__name {
    text-decoration: line-through;
    text-decoration-thickness: 1px;
    color: var(--text-muted);
  }
  .item__qty {
    display: flex;
    flex-wrap: wrap;
    gap: 0 var(--s2);
    align-items: center;
    margin-bottom: var(--s2);
  }
  .done-mark {
    display: inline-flex;
    align-items: center;
    gap: 0.25em;
    color: var(--good);
    font-weight: 650;
    font-size: var(--text-sm);
  }
  .item__why {
    margin-bottom: var(--s2);
  }
  .item__paid {
    max-width: 22rem;
  }
  .item__more {
    margin-bottom: var(--s2);
  }
  .item__more summary {
    color: var(--accent);
    min-height: 36px;
    font-size: var(--text-sm);
  }
  .sub {
    font-weight: 650;
    margin: var(--s2) 0 var(--s1);
  }
  .sub-inline {
    font-weight: 650;
  }
  .compact {
    padding: var(--s3) 0;
  }
  .compact .item__name {
    font-size: var(--text-base);
  }
  .compact .item__why {
    font-size: var(--text-sm);
    color: var(--text-muted);
    margin-bottom: var(--s1);
  }
  .compact .item__qty:empty {
    display: none;
  }
</style>
