<!--
  Screen 2, Who is in your household: a card per person (age group, earns income, pregnant or
  nursing), medical needs revealed only when the household says someone has them, and pets.
-->
<script lang="ts">
  import ChoiceGroup from '../components/ChoiceGroup.svelte';
  import CheckRow from '../components/CheckRow.svelte';
  import Field from '../components/Field.svelte';
  import Icon from '../components/Icon.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import NumberField from '../components/NumberField.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import Stepper from '../components/Stepper.svelte';
  import type { AgeBand, Person } from '../engine/types';
  import { AGE_BANDS, MOBILITY_LEVELS } from '../engine/types';
  import { useApp } from '../lib/app.svelte';
  import { AGE, DEVICE, MOBILITY } from '../lib/labels';

  const app = useApp();
  const input = $derived(app.plan?.input);

  function hasMedical(p: Person): boolean {
    const m = p.medical;
    return m.daily_rx || m.refrigerated_rx || m.powered_device !== 'none' || m.mobility !== 'none' || m.dietary.length > 0 || m.epinephrine;
  }

  let medicalOpen = $state<'yes' | 'no'>(app.plan?.input.people.some(hasMedical) ? 'yes' : 'no');
  let status = $state('');

  function syncEarners() {
    if (!app.plan) return;
    app.plan.input.finances.income.earners = app.plan.input.people.filter((p) => p.earner).length;
  }

  function newPerson(): Person {
    return {
      age_band: 'adult',
      pregnant_or_nursing: false,
      medical: { daily_rx: false, refrigerated_rx: false, powered_device: 'none', mobility: 'none', dietary: [], epinephrine: false },
      earner: false,
    };
  }

  function addPerson() {
    app.plan?.input.people.push(newPerson());
    status = `Person ${app.plan?.input.people.length} added.`;
  }

  function removePerson(i: number) {
    if (!app.plan || app.plan.input.people.length <= 1) return;
    app.plan.input.people.splice(i, 1);
    syncEarners();
    status = `Person ${i + 1} removed.`;
  }

  function setAge(p: Person, band: AgeBand) {
    p.age_band = band;
    if (band === 'infant' || band === 'toddler' || band === 'child') {
      p.earner = false;
      p.pregnant_or_nursing = false;
    }
    if (band === 'senior') p.pregnant_or_nursing = false;
    if ((band === 'infant' || band === 'toddler') && p.commute) delete p.commute;
    syncEarners();
  }

  function setMedicalOpen(v: 'yes' | 'no') {
    medicalOpen = v;
    if (v === 'no' && app.plan) {
      let cleared = false;
      for (const p of app.plan.input.people) {
        if (hasMedical(p)) cleared = true;
        p.medical = { daily_rx: false, refrigerated_rx: false, powered_device: 'none', mobility: 'none', dietary: [], epinephrine: false };
      }
      if (cleared) status = 'Medical details cleared.';
    }
  }

  function deviceKind(p: Person): 'none' | 'cpap' | 'oxygen' | 'other' {
    const d = p.medical.powered_device;
    return typeof d === 'object' ? 'other' : d;
  }

  function setDevice(p: Person, kind: 'none' | 'cpap' | 'oxygen' | 'other') {
    p.medical.powered_device = kind === 'other' ? { other: { watts: 60 } } : kind;
  }

  const canEarn = (band: AgeBand) => band === 'teen' || band === 'adult' || band === 'senior';
  const canBePregnant = (band: AgeBand) => band === 'teen' || band === 'adult';
</script>

<div class="page page--narrow">
  <ProgressSteps current="who" />
  <h1 id="page-title" tabindex="-1">Who is in your household</h1>
  <p class="lead">Ages and needs change how much water, food and medicine to keep, and which risks matter most.</p>

  {#if input}
    <ol class="people">
      {#each input.people as person, i (i)}
        <li class="card person">
          <div class="person__head">
            <h2>Person {i + 1}</h2>
            {#if input.people.length > 1}
              <button type="button" class="button button--quiet button--small" onclick={() => removePerson(i)}>
                <Icon name="trash" /> Remove<span class="visually-hidden"> person {i + 1}</span>
              </button>
            {/if}
          </div>
          <Field id="person-{i}-age" label="Age group">
            {#snippet children({ describedBy })}
              <select
                id="person-{i}-age"
                class="input input--medium"
                aria-describedby={describedBy}
                value={person.age_band}
                onchange={(e) => setAge(person, (e.currentTarget as HTMLSelectElement).value as AgeBand)}
              >
                {#each AGE_BANDS as band (band)}<option value={band}>{AGE[band].label}</option>{/each}
              </select>
            {/snippet}
          </Field>
          {#if canEarn(person.age_band)}
            <CheckRow label="Earns income for the household" help="Used for the savings goal if a job is lost." bind:checked={person.earner} onchange={syncEarners} />
          {/if}
          {#if canBePregnant(person.age_band)}
            <CheckRow label="Pregnant or nursing" help="Adds water and food, and keeps prenatal care in the plan." bind:checked={person.pregnant_or_nursing} />
          {/if}

          {#if medicalOpen === 'yes'}
            <fieldset class="medical">
              <legend>Medical needs for person {i + 1}</legend>
              <CheckRow label="Takes a daily prescription medicine" bind:checked={person.medical.daily_rx} />
              <CheckRow label="Has medicine that must stay cold" help="For example insulin." bind:checked={person.medical.refrigerated_rx} />
              <Field id="person-{i}-device" label="Powered medical device" help="A device that needs electricity to work.">
                {#snippet children({ describedBy })}
                  <select
                    id="person-{i}-device"
                    class="input input--medium"
                    aria-describedby={describedBy}
                    value={deviceKind(person)}
                    onchange={(e) => setDevice(person, (e.currentTarget as HTMLSelectElement).value as 'none' | 'cpap' | 'oxygen' | 'other')}
                  >
                    {#each ['none', 'cpap', 'oxygen', 'other'] as const as kind (kind)}<option value={kind}>{DEVICE[kind].label}</option>{/each}
                  </select>
                {/snippet}
              </Field>
              {#if typeof person.medical.powered_device === 'object'}
                {@const device = person.medical.powered_device}
                <NumberField
                  id="person-{i}-watts"
                  label="The device's power, in watts"
                  help="It is on the label or the charger, often between 30 and 300 watts."
                  value={device.other.watts}
                  min={1}
                  max={5000}
                  example="60"
                  suffix="watts"
                  onchange={(v) => {
                    if (v !== undefined) device.other.watts = v;
                  }}
                />
              {/if}
              <ChoiceGroup
                legend="Getting around"
                name="person-{i}-mobility"
                options={MOBILITY_LEVELS.map((v) => ({ value: v, label: MOBILITY[v].label, help: MOBILITY[v].help }))}
                bind:value={person.medical.mobility}
                columns={3}
              />
              <Field id="person-{i}-diet" label="Special diet" help="For example: vegetarian, formula, low sodium. Separate with commas." optional>
                {#snippet children({ describedBy })}
                  <input
                    id="person-{i}-diet"
                    class="input"
                    type="text"
                    autocomplete="off"
                    aria-describedby={describedBy}
                    value={person.medical.dietary.join(', ')}
                    onchange={(e) =>
                      (person.medical.dietary = (e.currentTarget as HTMLInputElement).value
                        .split(',')
                        .map((s) => s.trim())
                        .filter(Boolean))}
                  />
                {/snippet}
              </Field>
              <CheckRow label="Carries an epinephrine auto-injector" bind:checked={person.medical.epinephrine} />
            </fieldset>
          {/if}
        </li>
      {/each}
    </ol>
    <p><button type="button" class="button" onclick={addPerson}><Icon name="plus" /> Add a person</button></p>

    <ChoiceGroup
      legend="Does anyone need daily medicine, a powered medical device, help getting around, or a special diet?"
      name="medical-open"
      help="If yes, you can add details for each person above. Details stay on this device."
      options={[
        { value: 'no', label: 'No' },
        { value: 'yes', label: 'Yes' },
      ]}
      value={medicalOpen}
      onchange={setMedicalOpen}
      columns={2}
    />

    <section aria-labelledby="pets-title">
      <h2 id="pets-title">Pets and animals</h2>
      <Stepper label="Dogs" value={input.pets.dogs} noun="dog" onchange={(v) => (input.pets.dogs = v)} />
      <Stepper label="Cats" value={input.pets.cats} noun="cat" onchange={(v) => (input.pets.cats = v)} />
      <Stepper label="Small pets" help="Birds, rabbits, fish and similar." value={input.pets.small} noun="small pet" onchange={(v) => (input.pets.small = v)} />
      <Stepper label="Large animals" help="Horses and livestock." value={input.pets.large_animals} noun="large animal" onchange={(v) => (input.pets.large_animals = v)} />
    </section>

    <p class="visually-hidden" aria-live="polite">{status}</p>
    <InterviewNav step="who" />
  {/if}
</div>

<style>
  .people {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s4);
    display: grid;
    gap: var(--s4);
  }
  .people li + li {
    margin-top: 0;
  }
  .person__head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s3);
  }
  .person__head h2 {
    margin: 0;
  }
  .medical {
    margin-top: var(--s4);
    padding-top: var(--s4);
    border-top: 1px solid var(--border);
  }
</style>
