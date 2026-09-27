<!--
  Step 8, Contacts, pets, vehicles and documents (#/contacts; optional, DESIGN-DELTA-v3 §2.3):

  - Your trusted circle (up to 4 people, each with what they hold) and a lawyer (v2, unchanged).
  - Pets and animals (`family_plan.pets`, up to 8): name, kind, description, medicines, vet,
    microchip or tag number, where the records are; and v2's "who takes the animals if you can't".
    Shown to households with animals (or with answers already).
  - Vehicles (`family_plan.vehicles`, up to 4): description, plate, insurer and policy number, what
    stays in the car. Shown to households with a vehicle (or with answers already).
  - Documents and money (`family_plan.documents`): accounts (up to 12: institution, kind, phone and
    the last four digits only; a pasted full number is cut to its last four digits as it is typed,
    so it never reaches storage), insurance policies not given elsewhere (up to 8), and where the
    originals, the copies and the digital backup are.

  Then "Put it on paper": the wallet cards and the binder, and marking the plan's household-plan
  step done. `#/contacts/<card>` opens the step at one card (`circle`, `lawyer`, `pets`, `vehicles`,
  `documents`). Every field is optional; nothing blocks Continue. The paragraph at the top says the
  saved file can be protected with a passphrase (§7), because these answers are sensitive.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import ContactFields from '../components/ContactFields.svelte';
  import Icon from '../components/Icon.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import RowList from '../components/RowList.svelte';
  import StepPrivacy from '../components/StepPrivacy.svelte';
  import TextField from '../components/TextField.svelte';
  import type { Holds } from '../engine/types';
  import { FAMILY_PLAN_SHORT_MAX, FAMILY_PLAN_TEXT_MAX, HOLDS, TRUSTED_CIRCLE_MAX } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import {
    addPlanRow,
    addTrustedPerson,
    type FamilyContact,
    hasAnimals,
    hasVehicle,
    HOUSEHOLD_PLAN_STEPS,
    removePlanRow,
    removeTrustedPerson,
    setContactPart,
    setHolds,
    setNote,
    setPlanText,
    setTrustedPart,
    tidyContactPart,
    tidyNote,
    tidyPlanText,
    tidyTrustedPart,
  } from '../lib/family';
  import { HOLD } from '../lib/labels';
  import { allPlanItems } from '../lib/lookup';
  import { href, useRouter } from '../lib/router.svelte';
  import {
    ACCOUNT_MAX,
    ACCOUNTS_MAX,
    DOCUMENTS_MAX,
    lastFour,
    type Path,
    PET_MAX,
    PETS_MAX,
    POLICIES_MAX,
    POLICY_MAX,
    rowCount,
    textAt,
    VEHICLE_MAX,
    VEHICLES_MAX,
  } from '../lib/tidy';

  const app = useApp();
  const router = useRouter();
  const input = $derived(app.plan?.input);
  const plan = $derived(input?.family_plan);
  let status = $state('');

  /** The cards, each reachable as `#/contacts/<card>`. */
  const CARDS = ['circle', 'lawyer', 'pets', 'vehicles', 'documents'] as const;

  // Opened at one card (#/contacts/circle): go there once the page has settled.
  $effect(() => {
    const card = router.current.id === 'contacts' ? router.current.param : undefined;
    if (!card || !(CARDS as readonly string[]).includes(card) || !app.plan) return;
    const timer = setTimeout(() => jumpTo(`contacts-${card}`, { focus: 'h2' }), 0);
    return () => clearTimeout(timer);
  });

  function edit(path: Path, text: string) {
    if (app.plan) setPlanText(app.plan.input, path, text);
  }
  function leave(path: Path, max: number) {
    if (app.plan) tidyPlanText(app.plan.input, path, max);
  }
  function contact(key: FamilyContact, part: 'name' | 'phone', text: string) {
    if (app.plan) setContactPart(app.plan.input, key, part, text);
  }
  function contactLeft(key: FamilyContact, part: 'name' | 'phone') {
    if (app.plan) tidyContactPart(app.plan.input, key, part);
  }

  /**
   * The last four digits of whatever is typed or pasted, shown and saved at once: a full account
   * number never reaches the saved plan, not even for a moment.
   */
  function typeLast4(el: HTMLInputElement, index: number) {
    const kept = lastFour(el.value) ?? '';
    if (el.value !== kept) el.value = kept;
    edit(['documents', 'accounts', index, 'last4'], kept);
  }

  async function newPerson() {
    if (!app.plan || !addTrustedPerson(app.plan.input)) return;
    const n = app.plan.input.family_plan?.trusted_circle?.length ?? 0;
    status = `Person ${n} added to your trusted circle.`;
    await tick();
    document.getElementById(`fp-circle-${n - 1}-name`)?.focus();
  }

  function dropPerson(i: number) {
    if (!app.plan) return;
    removeTrustedPerson(app.plan.input, i);
    status = `Person ${i + 1} removed from your trusted circle.`;
    document.getElementById('fp-add-person')?.focus();
  }

  function holds(i: number, what: Holds, on: boolean) {
    if (app.plan) setHolds(app.plan.input, i, what, on);
  }

  /** A row's legend: "Animal 2", or what the row is called once typed ("Biscuit (animal 2)"). */
  function legendOf(noun: string, i: number, named: string | undefined): string {
    const name = named?.trim();
    return name ? `${name} (${noun.toLowerCase()} ${i + 1})` : `${noun} ${i + 1}`;
  }

  /** The plan's "Make a household plan" step, if the plan has one. */
  const householdStep = $derived(
    app.result.output ? allPlanItems(app.result.output).find((x) => HOUSEHOLD_PLAN_STEPS.includes(x.item.item_id))?.item : undefined,
  );

  function markHouseholdStepDone() {
    if (!householdStep || householdStep.done) return;
    app.record(householdStep);
    status = `Marked as done on your plan: ${householdStep.name}.`;
  }

  const showPets = $derived(!!input && (hasAnimals(input) || !!plan?.pets?.length || !!plan?.who_takes_animals));
  const showVehicles = $derived(!!input && (hasVehicle(input) || !!plan?.vehicles?.length));
</script>

<!-- One free-text answer inside `family_plan` at `path`. -->
{#snippet text(path: Path, label: string, max: number, opts: { help?: string; context?: string; multiline?: boolean; inputmode?: 'tel' | 'text'; width?: 'short' | 'medium' } = {})}
  <TextField
    id="ct-{path.join('-')}"
    {label}
    help={opts.help}
    context={opts.context}
    value={textAt(plan, path)}
    maxlength={max}
    multiline={opts.multiline}
    inputmode={opts.inputmode}
    width={opts.width}
    oninput={(t) => edit(path, t)}
    onleave={() => leave(path, max)}
  />
{/snippet}

<div class="page page--narrow">
  <ProgressSteps current="contacts" />
  <h1 id="page-title" tabindex="-1">Contacts, pets, vehicles and documents</h1>

  {#if !app.plan || !input}
    <div class="card gate">
      <p>You haven't started a plan on this device yet. It takes about ten minutes, and nothing you enter leaves your browser.</p>
      <p class="button-row"><a class="button button--primary" href={href('start')}>Start a plan</a></p>
    </div>
  {:else}
    <StepPrivacy>
      Everything here is optional. Your answers fill the contacts, pets, vehicles and documents pages of your binder; anything left blank
      prints as a line to fill in by hand. They are kept only on this device and in any plan file you save, never sent anywhere, and never
      used to work out your plan. Because these details are sensitive, when you save your plan to a file you can protect it with a
      passphrase.
    </StepPrivacy>

    <nav class="toc no-print" aria-label="Parts of this step">
      <ul>
        {#each [['circle', 'Your trusted circle'], ['lawyer', 'A lawyer'], ...(showPets ? [['pets', 'Pets and animals']] : []), ...(showVehicles ? [['vehicles', 'Vehicles']] : []), ['documents', 'Documents and money']] as [card, title] (card)}
          <li><a href="#contacts-{card}" onclick={(e) => jumpTo(`contacts-${card}`, { focus: 'h2' }) && e.preventDefault()}>{title}</a></li>
        {/each}
      </ul>
    </nav>

    <section id="contacts-circle" class="card step-card" aria-labelledby="fp-circle-title">
      <h2 id="fp-circle-title">Your trusted circle</h2>
      <p class="section-intro">
        Up to {TRUSTED_CIRCLE_MAX} people who have agreed ahead of time to help each other: a bed for a night, a ride, care for children or
        pets. Agree it on a calm day; in a crisis there is no time to ask.
      </p>
      {#if plan?.trusted_circle?.length}
        <ol class="circle">
          {#each plan.trusted_circle as person, i (i)}
            <li class="circle__person">
              <div class="circle__head">
                <h3>Person {i + 1}</h3>
                <button type="button" class="button button--quiet button--small" onclick={() => dropPerson(i)}>
                  <Icon name="trash" /> Remove<span class="visually-hidden">{' '}person {i + 1} from your trusted circle</span>
                </button>
              </div>
              <div class="pair">
                <TextField
                  id="fp-circle-{i}-name"
                  label="Name"
                  context="of person {i + 1}"
                  value={person.name}
                  maxlength={FAMILY_PLAN_SHORT_MAX}
                  oninput={(t) => app.plan && setTrustedPart(app.plan.input, i, 'name', t)}
                  onleave={() => app.plan && tidyTrustedPart(app.plan.input, i, 'name')}
                />
                <TextField
                  id="fp-circle-{i}-phone"
                  label="Phone"
                  context="of person {i + 1}"
                  value={person.phone}
                  maxlength={FAMILY_PLAN_SHORT_MAX}
                  inputmode="tel"
                  oninput={(t) => app.plan && setTrustedPart(app.plan.input, i, 'phone', t)}
                  onleave={() => app.plan && tidyTrustedPart(app.plan.input, i, 'phone')}
                />
              </div>
              <fieldset>
                <legend>What {person.name?.trim() || `person ${i + 1}`} holds for you</legend>
                <div class="choices choices--2">
                  {#each HOLDS as what (what)}
                    <label class="choice">
                      <input
                        type="checkbox"
                        checked={person.holds?.includes(what) ?? false}
                        onchange={(e) => holds(i, what, (e.currentTarget as HTMLInputElement).checked)}
                      />
                      <span class="choice__text">
                        <span>{HOLD[what].label}</span>
                        {#if HOLD[what].help}<span class="choice__help">{HOLD[what].help}</span>{/if}
                      </span>
                    </label>
                  {/each}
                </div>
              </fieldset>
            </li>
          {/each}
        </ol>
      {/if}
      {#if (plan?.trusted_circle?.length ?? 0) < TRUSTED_CIRCLE_MAX}
        <button id="fp-add-person" type="button" class="button" onclick={newPerson}><Icon name="plus" /> Add someone</button>
      {:else}
        <p class="small muted" id="fp-add-person" tabindex="-1">That is four people, the most the plan keeps.</p>
      {/if}
    </section>

    <section id="contacts-lawyer" class="card step-card" aria-labelledby="fp-lawyer-title">
      <h2 id="fp-lawyer-title">A lawyer</h2>
      <p class="section-intro">Find one on a calm day and keep the number on paper. Legal aid helps people with low incomes with problems such as eviction.</p>
      <div class="pair">
        <TextField
          id="fp-lawyer-name"
          label="Lawyer's name, or the office"
          value={plan?.lawyer?.name}
          maxlength={FAMILY_PLAN_SHORT_MAX}
          oninput={(t) => contact('lawyer', 'name', t)}
          onleave={() => contactLeft('lawyer', 'name')}
        />
        <TextField
          id="fp-lawyer-phone"
          label="Phone"
          context="of the lawyer"
          value={plan?.lawyer?.phone}
          maxlength={FAMILY_PLAN_SHORT_MAX}
          inputmode="tel"
          oninput={(t) => contact('lawyer', 'phone', t)}
          onleave={() => contactLeft('lawyer', 'phone')}
        />
      </div>
    </section>

    {#if showPets}
      <section id="contacts-pets" class="card step-card" aria-labelledby="ct-pets-title">
        <h2 id="ct-pets-title">Pets and animals</h2>
        <p class="section-intro">What a helper, a shelter or a boarding kennel would need to know about each animal. Up to {PETS_MAX}.</p>
        <RowList
          id="ct-pet"
          count={rowCount(plan, ['pets'])}
          max={PETS_MAX}
          legend={(i) => legendOf('Animal', i, plan?.pets?.[i]?.name)}
          addLabel="Add an animal"
          fullText="That is {PETS_MAX} animals, the most the binder keeps."
          onadd={() => !!app.plan && addPlanRow(app.plan.input, ['pets'], PETS_MAX)}
          onremove={(i) => app.plan && removePlanRow(app.plan.input, ['pets'], i)}
          onstatus={(s) => (status = s)}
        >
          {#snippet row(i)}
            {@const of = `of animal ${i + 1}`}
            <div class="pair">
              {@render text(['pets', i, 'name'], 'Name', PET_MAX.name, { context: of })}
              {@render text(['pets', i, 'kind'], 'Kind of animal', PET_MAX.kind, { help: 'For example a dog, a cat or a horse.', context: of })}
            </div>
            {@render text(['pets', i, 'description'], 'What it looks like', PET_MAX.description, { help: 'Colour, size and markings: what would help someone find it.', context: of })}
            {@render text(['pets', i, 'medications'], 'Medicines', PET_MAX.medications, { help: 'Any medicine it takes, and when.', context: of, multiline: true })}
            <ContactFields
              id="ct-pets-{i}-vet"
              legend="Vet"
              value={plan?.pets?.[i]?.vet}
              context="of the vet for animal {i + 1}"
              oninput={(part, t) => edit(['pets', i, 'vet', part], t)}
              onleave={(part, max) => leave(['pets', i, 'vet', part], max)}
            />
            <div class="pair">
              {@render text(['pets', i, 'microchip'], 'Microchip or tag number', PET_MAX.microchip, { context: of })}
              {@render text(['pets', i, 'records_where'], 'Where the records are', PET_MAX.records_where, { help: 'Vaccination records: shelters and kennels ask for them.', context: of })}
            </div>
          {/snippet}
        </RowList>
        <TextField
          id="fp-animals"
          label="Who takes the animals if you can't"
          help="Someone who has agreed, and knows where the carriers, leashes and food are."
          value={plan?.who_takes_animals}
          maxlength={FAMILY_PLAN_TEXT_MAX}
          multiline
          oninput={(t) => app.plan && setNote(app.plan.input, 'who_takes_animals', t)}
          onleave={() => app.plan && tidyNote(app.plan.input, 'who_takes_animals')}
        />
      </section>
    {:else}
      <p class="card step-card small muted" id="contacts-pets">No pets or animals on <a href={href('who')}>Who is in your household</a>. Add them there if you have any, and they get a place here.</p>
    {/if}

    {#if showVehicles}
      <section id="contacts-vehicles" class="card step-card" aria-labelledby="ct-vehicles-title">
        <h2 id="ct-vehicles-title">Vehicles</h2>
        <p class="section-intro">So anyone can report a vehicle, claim for it, or find what is in it. Up to {VEHICLES_MAX}.</p>
        <RowList
          id="ct-vehicle"
          count={rowCount(plan, ['vehicles'])}
          max={VEHICLES_MAX}
          legend={(i) => legendOf('Vehicle', i, plan?.vehicles?.[i]?.description)}
          addLabel="Add a vehicle"
          fullText="That is {VEHICLES_MAX} vehicles, the most the binder keeps."
          onadd={() => !!app.plan && addPlanRow(app.plan.input, ['vehicles'], VEHICLES_MAX)}
          onremove={(i) => app.plan && removePlanRow(app.plan.input, ['vehicles'], i)}
          onstatus={(s) => (status = s)}
        >
          {#snippet row(i)}
            {@const of = `of vehicle ${i + 1}`}
            <div class="pair">
              {@render text(['vehicles', i, 'description'], 'Description', VEHICLE_MAX.description, { help: 'For example "blue 2016 hatchback".', context: of })}
              {@render text(['vehicles', i, 'plate'], 'License plate', VEHICLE_MAX.plate, { context: of, width: 'medium' })}
            </div>
            <ContactFields
              id="ct-vehicles-{i}-insurer"
              legend="Insurance"
              value={plan?.vehicles?.[i]?.insurer}
              context="of the insurer of vehicle {i + 1}"
              labels={{ name: 'Insurance company', phone: 'Claims phone' }}
              oninput={(part, t) => edit(['vehicles', i, 'insurer', part], t)}
              onleave={(part, max) => leave(['vehicles', i, 'insurer', part], max)}
            >
              {@render text(['vehicles', i, 'policy_number'], 'Policy number', VEHICLE_MAX.policy_number, { context: of, width: 'medium' })}
            </ContactFields>
            {@render text(['vehicles', i, 'kept_in_car'], 'What stays in the car', VEHICLE_MAX.kept_in_car, {
              help: 'For example a kit, a blanket, water and a phone charger.',
              context: of,
              multiline: true,
            })}
          {/snippet}
        </RowList>
      </section>
    {:else}
      <p class="card step-card small muted" id="contacts-vehicles">No vehicles on <a href={href('travel')}>How you get around</a>. Add them there if you have any, and they get a place here.</p>
    {/if}

    <section id="contacts-documents" class="card step-card" aria-labelledby="ct-documents-title">
      <h2 id="ct-documents-title">Documents and money</h2>
      <p class="section-intro">
        After a disaster, banks and insurers are easier to reach with the right numbers to hand. Ready Reckoner never asks for a full account
        number.
      </p>
      <fieldset class="group" aria-describedby="ct-accounts-help">
        <legend>Accounts</legend>
        <p class="help" id="ct-accounts-help">Banks, credit cards and other accounts: who to call, and the last four digits to tell them apart. Up to {ACCOUNTS_MAX}.</p>
        <RowList
          id="ct-account"
          count={rowCount(plan, ['documents', 'accounts'])}
          max={ACCOUNTS_MAX}
          legend={(i) => legendOf('Account', i, plan?.documents?.accounts?.[i]?.institution)}
          addLabel="Add an account"
          fullText="That is {ACCOUNTS_MAX} accounts, the most the binder keeps."
          onadd={() => !!app.plan && addPlanRow(app.plan.input, ['documents', 'accounts'], ACCOUNTS_MAX)}
          onremove={(i) => app.plan && removePlanRow(app.plan.input, ['documents', 'accounts'], i)}
          onstatus={(s) => (status = s)}
        >
          {#snippet row(i)}
            {@const of = `of account ${i + 1}`}
            <div class="pair">
              {@render text(['documents', 'accounts', i, 'institution'], 'Bank or company', ACCOUNT_MAX.institution, { context: of })}
              {@render text(['documents', 'accounts', i, 'kind'], 'Kind of account', ACCOUNT_MAX.kind, { help: 'For example checking, savings or a credit card.', context: of })}
              {@render text(['documents', 'accounts', i, 'phone'], 'Phone', ACCOUNT_MAX.phone, { context: of, inputmode: 'tel' })}
              <div class="field">
                <label for="ct-documents-accounts-{i}-last4">Last four digits<span class="visually-hidden">{' '}{of}</span></label>
                <span class="help" id="ct-documents-accounts-{i}-last4-help">Only the last four. Anything longer is cut to its last four digits.</span>
                <input
                  id="ct-documents-accounts-{i}-last4"
                  class="input input--short"
                  type="text"
                  inputmode="numeric"
                  autocomplete="off"
                  aria-describedby="ct-documents-accounts-{i}-last4-help"
                  value={plan?.documents?.accounts?.[i]?.last4 ?? ''}
                  oninput={(e) => typeLast4(e.currentTarget as HTMLInputElement, i)}
                />
              </div>
            </div>
          {/snippet}
        </RowList>
      </fieldset>
      <fieldset class="group" aria-describedby="ct-policies-help">
        <legend>Insurance policies not already given</legend>
        <p class="help" id="ct-policies-help">Policies not on the home, vehicle or people pages, such as life, disability or flood insurance. Up to {POLICIES_MAX}.</p>
        <RowList
          id="ct-policy"
          count={rowCount(plan, ['documents', 'policies'])}
          max={POLICIES_MAX}
          legend={(i) => legendOf('Policy', i, plan?.documents?.policies?.[i]?.insurer)}
          addLabel="Add a policy"
          fullText="That is {POLICIES_MAX} policies, the most the binder keeps."
          onadd={() => !!app.plan && addPlanRow(app.plan.input, ['documents', 'policies'], POLICIES_MAX)}
          onremove={(i) => app.plan && removePlanRow(app.plan.input, ['documents', 'policies'], i)}
          onstatus={(s) => (status = s)}
        >
          {#snippet row(i)}
            {@const of = `of policy ${i + 1}`}
            <div class="pair">
              {@render text(['documents', 'policies', i, 'insurer'], 'Insurance company', POLICY_MAX.insurer, { context: of })}
              {@render text(['documents', 'policies', i, 'kind'], 'Kind of policy', POLICY_MAX.kind, { help: 'For example life, disability or flood.', context: of })}
              {@render text(['documents', 'policies', i, 'policy_number'], 'Policy number', POLICY_MAX.policy_number, { context: of })}
              {@render text(['documents', 'policies', i, 'phone'], 'Phone', POLICY_MAX.phone, { context: of, inputmode: 'tel' })}
            </div>
          {/snippet}
        </RowList>
      </fieldset>
      {@render text(['documents', 'where_originals'], 'Where the originals are', DOCUMENTS_MAX.where_originals, {
        help: 'Birth certificates, deeds, titles, passports: the papers that are hard to replace.',
      })}
      {@render text(['documents', 'where_copies'], 'Where the copies are', DOCUMENTS_MAX.where_copies, { help: 'For example in the go-bag, or with someone in your trusted circle.' })}
      {@render text(['documents', 'digital_backup'], 'Where the digital backup is', DOCUMENTS_MAX.digital_backup, {
        help: 'A drive or an online account. Write where it is, never the password.',
      })}
    </section>

    <section class="card finish" aria-labelledby="ct-finish-title">
      <h2 id="ct-finish-title">Put it on paper</h2>
      <p>Your binder prints these answers: a page for each person, your home and places, and a wallet card for each person with the numbers and meeting places on it.</p>
      <p class="button-row">
        <a class="button button--primary" href={href('binder', 'wallet-cards')}><Icon name="print" /> Print wallet cards</a>
        <a class="button" href={href('binder')}>See your binder</a>
      </p>
      {#if householdStep}
        {#if householdStep.done}
          <p class="good"><Icon name="check" /> "{householdStep.name}" is done on your plan.</p>
        {:else}
          <p>
            <button type="button" class="button button--small" onclick={markHouseholdStepDone}>Mark "{householdStep.name}" as done</button>
          </p>
        {/if}
      {/if}
    </section>

    <p class="visually-hidden" aria-live="polite">{status}</p>
    <InterviewNav step="contacts" nextLabel="See your risks" />
  {/if}
</div>

<style>
  .gate {
    max-width: var(--w-text);
  }
  .toc ul {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s3);
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s4);
    font-size: var(--text-sm);
  }
  .toc li + li {
    margin-top: 0;
  }
  .toc a {
    display: inline-flex;
    align-items: center;
    min-height: 36px;
  }
  .step-card {
    margin-bottom: var(--s5);
  }
  .step-card h2 {
    margin-top: 0;
  }
  .group {
    margin-bottom: var(--s5);
  }
  /* Two answers side by side when there is room; the grid gap spaces them either way. */
  .pair {
    display: grid;
    gap: var(--s3) var(--s4);
    margin-bottom: var(--s4);
  }
  .pair :global(.field) {
    margin-bottom: 0;
  }
  @media (min-width: 36rem) {
    .pair {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    }
  }
  .field > label {
    display: block;
  }
  .circle {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s4);
    display: grid;
    gap: var(--s4);
  }
  .circle li + li {
    margin-top: 0;
  }
  .circle__person {
    padding: var(--s3) var(--s4);
    border: 1px solid var(--border);
    border-radius: var(--r2);
  }
  .circle__head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s3);
  }
  .circle__head h3 {
    margin: 0;
  }
  .finish h2 {
    margin-top: 0;
    font-size: var(--text-lg);
  }
  .good {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    color: var(--good);
    font-weight: 600;
  }
</style>
