<!--
  Screen 3, How you get around: vehicles and their fuel, and each person's regular trip (work for
  adults, school for children): distance, how, and whether they could stay home instead.
-->
<script lang="ts">
  import ChoiceGroup from '../components/ChoiceGroup.svelte';
  import CheckRow from '../components/CheckRow.svelte';
  import Field from '../components/Field.svelte';
  import Icon from '../components/Icon.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import NumberField from '../components/NumberField.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import type { Fuel, Person } from '../engine/types';
  import { COMMUTE_MODES, FUELS } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { AGE, COMMUTE_MODE, FUEL, kmToMiles, milesToKm } from '../lib/labels';

  const app = useApp();
  const input = $derived(app.plan?.input);
  let status = $state('');

  const isChild = (p: Person) => p.age_band === 'child' || p.age_band === 'teen';
  const travels = (p: Person) => p.age_band !== 'infant' && p.age_band !== 'toddler';

  function setTravels(p: Person, on: boolean) {
    if (on) p.commute = { distance_km: milesToKm(5), mode: isChild(p) ? 'transit' : 'car', remote_possible: false };
    else delete p.commute;
  }

  function addVehicle() {
    app.plan?.input.mobility.vehicles.push({ fuel: 'gas' });
    status = 'Vehicle added.';
  }

  function removeVehicle(i: number) {
    app.plan?.input.mobility.vehicles.splice(i, 1);
    status = 'Vehicle removed.';
  }

  const hasEv = $derived(input?.mobility.vehicles.some((v) => v.fuel === 'ev') ?? false);
</script>

<div class="page page--narrow">
  <ProgressSteps current="travel" />
  <h1 id="page-title" tabindex="-1">How you get around</h1>
  <p class="lead">How far people travel decides what to keep for getting home, and how you would leave in a hurry.</p>

  {#if input}
    <section aria-labelledby="vehicles-title">
      <h2 id="vehicles-title">Vehicles</h2>
      {#if input.mobility.vehicles.length === 0}
        <p class="note">No vehicle. The plan leans on walking routes, transit and neighbours for leaving quickly.</p>
      {/if}
      <ul class="vehicles">
        {#each input.mobility.vehicles as vehicle, i (i)}
          <li class="vehicle">
            <Field id="vehicle-{i}" label="Vehicle {i + 1} runs on" help="Changes what to do before a storm: fill up or charge.">
              {#snippet children({ describedBy })}
                <select id="vehicle-{i}" class="input input--medium" aria-describedby={describedBy} bind:value={vehicle.fuel}>
                  {#each FUELS as f (f)}<option value={f as Fuel}>{FUEL[f].label}</option>{/each}
                </select>
              {/snippet}
            </Field>
            <button type="button" class="button button--quiet button--small" onclick={() => removeVehicle(i)}>
              <Icon name="trash" /> Remove<span class="visually-hidden">{' '}vehicle {i + 1}</span>
            </button>
          </li>
        {/each}
      </ul>
      <p><button type="button" class="button" onclick={addVehicle}><Icon name="plus" /> Add a vehicle</button></p>
      {#if hasEv}
        <p class="note">Because you have an electric car, your plan includes keeping it charged before storms.</p>
      {/if}
    </section>

    <section aria-labelledby="trips-title">
      <h2 id="trips-title">Regular trips</h2>
      <p class="section-intro">If roads or transit stop, people may have to walk home. Rough distances are fine.</p>
      {#each input.people as person, i (i)}
        {#if travels(person)}
          <fieldset class="card trip">
            <legend>Person {i + 1}: {AGE[person.age_band].label.toLowerCase()}</legend>
            <CheckRow
              label={isChild(person) ? 'Goes to school or daycare away from home' : 'Travels to work or school'}
              help="Someone away from home may need to get back on foot."
              checked={!!person.commute}
              onchange={(on) => setTravels(person, on)}
            />
            {#if person.commute}
              {@const commute = person.commute}
              <NumberField
                id="trip-{i}-distance"
                label="One-way distance"
                help="In miles. About 3 miles is an hour's walk."
                value={kmToMiles(commute.distance_km)}
                suffix="miles"
                example="8"
                onchange={(v) => {
                  if (v !== undefined) commute.distance_km = milesToKm(v);
                }}
              />
              <ChoiceGroup
                legend="Usually by"
                name="trip-{i}-mode"
                help="Transit and roads can stop in a storm or outage."
                options={COMMUTE_MODES.map((v) => ({ value: v, label: COMMUTE_MODE[v].label }))}
                bind:value={commute.mode}
                columns={4}
              />
              <CheckRow label={isChild(person) ? 'Can learn from home when needed' : 'Can work from home when needed'} help="Staying home on a bad day avoids getting stranded." bind:checked={commute.remote_possible} />
            {/if}
          </fieldset>
        {/if}
      {/each}
    </section>
    <p class="visually-hidden" aria-live="polite">{status}</p>
    <InterviewNav step="travel" />
  {/if}
</div>

<style>
  .vehicles {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .vehicle {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--s3);
  }
  .vehicle :global(.field) {
    margin-bottom: var(--s3);
    flex: 1 1 14rem;
  }
  .vehicle .button {
    margin-bottom: var(--s3);
  }
  .note {
    padding: var(--s3) var(--s4);
    background: var(--note-soft);
    border-radius: var(--r2);
  }
  .trip {
    margin-bottom: var(--s4);
  }
  .trip legend {
    font-size: var(--text-lg);
    padding-top: var(--s1);
  }
</style>
