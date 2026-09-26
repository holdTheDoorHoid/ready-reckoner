<!--
  Screen 5, What you already have (optional): safety equipment, quantities of common supplies, and
  free steps already done. Everything entered here comes off the plan. Skippable.
-->
<script lang="ts">
  import CheckRow from '../components/CheckRow.svelte';
  import Icon from '../components/Icon.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import NumberField from '../components/NumberField.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import type { Item } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { plural } from '../lib/format';
  import { useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();
  const input = $derived(app.plan?.input);

  /** Alarms are asked as yes/no above, so they are left out of the supplies list. */
  const ASKED_AS_ALARMS = new Set(['smoke_alarm', 'co_alarm', 'fire_extinguisher']);
  const GROUP_ORDER = ['water', 'food', 'medical', 'power', 'comms', 'thermal', 'evacuate', 'get_home', 'money', 'documents', 'sanitation', 'home', 'fire', 'animals'];
  const GROUP_NAMES: Record<string, string> = {
    water: 'Water',
    food: 'Food',
    medical: 'Medicine and first aid',
    power: 'Light and power',
    comms: 'Phones and news',
    thermal: 'Heat and cold',
    evacuate: 'Bags for leaving',
    get_home: 'Bags for leaving',
    money: 'Money',
    documents: 'Papers',
    sanitation: 'Toilet and hygiene',
    home: 'Around the home',
    fire: 'Fire safety',
    animals: 'Pets and animals',
  };

  const eaters = $derived(Math.max(1, input?.people.filter((p) => p.age_band !== 'infant').length ?? 1));
  const needsMedicine = $derived(input?.people.some((p) => p.medical.daily_rx) ?? false);
  const hasPets = $derived((input?.pets.dogs ?? 0) + (input?.pets.cats ?? 0) + (input?.pets.small ?? 0) > 0);
  const hasInfant = $derived(input?.people.some((p) => p.age_band === 'infant') ?? false);

  function relevant(item: Item): boolean {
    if (item.free || item.rare_catastrophic || ASKED_AS_ALARMS.has(item.id)) return false;
    if (item.id === 'medication_reserve') return needsMedicine;
    if (item.id === 'pet_food_reserve') return hasPets;
    if (item.id === 'infant_formula_reserve') return hasInfant;
    return true;
  }

  const groups = $derived.by(() => {
    const items = (app.catalogue?.items ?? []).filter(relevant);
    const byGroup = new Map<string, Item[]>();
    for (const item of items) {
      const g = GROUP_NAMES[item.category] ?? 'Other';
      byGroup.set(g, [...(byGroup.get(g) ?? []), item]);
    }
    const order = [...new Set(GROUP_ORDER.map((c) => GROUP_NAMES[c]!)), 'Other'];
    return order.filter((g) => byGroup.has(g)).map((g) => ({ name: g, items: byGroup.get(g)! }));
  });
  const basics = $derived(groups.map((g) => ({ ...g, items: g.items.filter((i) => i.tier === 'h72') })).filter((g) => g.items.length));
  const more = $derived(groups.map((g) => ({ ...g, items: g.items.filter((i) => i.tier !== 'h72') })).filter((g) => g.items.length));

  const freeSteps = $derived.by(() => {
    const fromPlan = app.result.output?.plan.months[0]?.items.filter((i) => i.kind === 'free_action').map((i) => i.item_id);
    const all = (app.catalogue?.items ?? []).filter((i) => i.free);
    return fromPlan ? all.filter((i) => fromPlan.includes(i.id)) : all;
  });

  function owned(id: string): number | undefined {
    return input?.existing.find((o) => o.item_id === id)?.qty;
  }

  function checkedOffOnPlan(id: string): boolean {
    return app.plan?.purchases.some((p) => p.item_id === id) ?? false;
  }

  function setOwned(id: string, qty: number | undefined) {
    if (!app.plan) return;
    const list = app.plan.input.existing;
    const i = list.findIndex((o) => o.item_id === id);
    if (!qty) {
      if (i >= 0) list.splice(i, 1);
    } else if (i >= 0) {
      list[i]!.qty = qty;
    } else {
      list.push({ item_id: id, qty });
    }
  }

  function skip() {
    app.completeStep('have');
    router.go('risks');
  }
</script>

{#snippet itemField(item: Item)}
  {#if item.energy_kcal_per_unit}
    <NumberField
      id="have-{item.id}"
      label={item.name}
      help="About how many days could your household eat from what is in the cupboards now?"
      value={owned(item.id) !== undefined ? Math.round(((owned(item.id) ?? 0) / eaters) * 10) / 10 : undefined}
      optional
      suffix="days"
      example="3"
      onchange={(v) => setOwned(item.id, v === undefined ? undefined : Math.round(v * eaters * 10) / 10)}
    />
  {:else if item.unit === 'dollar'}
    <NumberField id="have-{item.id}" label={item.name} value={owned(item.id)} optional prefix="$" example="100" onchange={(v) => setOwned(item.id, v)} />
  {:else}
    <NumberField
      id="have-{item.id}"
      label={item.name}
      value={owned(item.id)}
      optional
      suffix={plural(item.unit, 2)}
      example="2"
      onchange={(v) => setOwned(item.id, v)}
    />
  {/if}
{/snippet}

<div class="page page--narrow">
  <ProgressSteps current="have" />
  <h1 id="page-title" tabindex="-1">What you already have</h1>
  <p class="lead">Optional. Anything you already have comes off your plan, so it costs you nothing more.</p>

  {#if input}
    <p class="skip">
      <button type="button" class="button" onclick={skip}>Skip this step <Icon name="chevron-right" /></button>
      <span class="small muted">The plan will include checking what you have.</span>
    </p>

    <section aria-labelledby="safety-title">
      <h2 id="safety-title">Safety equipment</h2>
      <CheckRow label="A working smoke alarm on each level" bind:checked={input.housing.alarms.smoke} />
      <CheckRow label="A carbon monoxide alarm" help="Matters most if you have gas appliances or use a generator." bind:checked={input.housing.alarms.co} />
      <CheckRow label="A fire extinguisher" bind:checked={input.housing.alarms.extinguisher} />
    </section>

    <section aria-labelledby="supplies-title">
      <h2 id="supplies-title">Supplies</h2>
      <p class="section-intro">Leave blank anything you don't have. Rough numbers are fine.</p>
      {#each basics as group (group.name)}
        <h3>{group.name}</h3>
        {#each group.items as item (item.id)}{@render itemField(item)}{/each}
      {/each}
      {#if more.length}
        <details class="more">
          <summary>More things you might have</summary>
          {#each more as group (group.name)}
            <h3>{group.name}</h3>
            {#each group.items as item (item.id)}{@render itemField(item)}{/each}
          {/each}
        </details>
      {/if}
    </section>

    <section aria-labelledby="done-title">
      <h2 id="done-title">Free steps you may have done already</h2>
      <p class="section-intro">Many households have done some of these. Each one counts.</p>
      {#each freeSteps as step (step.id)}
        {#if checkedOffOnPlan(step.id)}
          <CheckRow label={step.name} help="Checked off on your plan." checked disabled />
        {:else}
          <CheckRow label={step.name} checked={(owned(step.id) ?? 0) >= 1} onchange={(on) => setOwned(step.id, on ? 1 : undefined)} />
        {/if}
      {/each}
    </section>

    <InterviewNav step="have" nextLabel="See your risks" />
  {/if}
</div>

<style>
  .skip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s3);
  }
  h3 {
    margin-top: var(--s5);
    font-size: var(--text-base);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .more {
    margin-top: var(--s4);
  }
</style>
