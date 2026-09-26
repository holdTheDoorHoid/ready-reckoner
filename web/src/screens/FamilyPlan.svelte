<!--
  Your family plan (#/family): the household's own plan in plain forms (contract v2
  `PlanInput.family_plan`): how to stay in touch, the children and work, where to shelter, where to
  go and how, the shut-offs and neighbours, the trusted circle and a lawyer. Every field is
  optional free text. It is saved in the plan (this browser and the saved file) and nowhere else,
  never used to work anything out, and printed at the front of the packet and on a wallet card for
  each person ("Print wallet cards" opens the packet there). Help for staying in touch and for
  sheltering is the reviewed plan_communication and plan_shelter blocks, trimmed for this household.
  `#/family/<section>` opens the screen at one part (the plan's free steps link there).
-->
<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import PlanGuide from '../components/PlanGuide.svelte';
  import TextField from '../components/TextField.svelte';
  import type { Holds } from '../engine/types';
  import { FAMILY_PLAN_SHORT_MAX, FAMILY_PLAN_TEXT_MAX, HOLDS, NUMBERS_BY_HEART_MAX, ROUTES_MAX, TRUSTED_CIRCLE_MAX } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import {
    addNumber,
    addTrustedPerson,
    FAMILY_SECTIONS,
    type FamilyContact,
    type FamilyNote,
    type FamilySection,
    hasAnimals,
    hasChildren,
    hasGas,
    hasVehicle,
    HOUSEHOLD_PLAN_STEPS,
    isFamilySection,
    removeNumber,
    removeTrustedPerson,
    sectionId,
    setContactPart,
    setHolds,
    setNote,
    setNumber,
    setRoadside,
    setRoute,
    setTrustedPart,
    tidyContactPart,
    tidyNote,
    tidyNumber,
    tidyRoadside,
    tidyRoute,
    tidyTrustedPart,
  } from '../lib/family';
  import { HOLD } from '../lib/labels';
  import { allPlanItems } from '../lib/lookup';
  import { href, useRouter } from '../lib/router.svelte';

  const app = useApp();
  const router = useRouter();
  const input = $derived(app.plan?.input);
  const plan = $derived(input?.family_plan);
  let status = $state('');

  // Opened at one part (#/family/circle): go there once the page has settled.
  $effect(() => {
    const section = router.current.id === 'family' ? router.current.param : undefined;
    if (!isFamilySection(section) || !app.plan) return;
    const timer = setTimeout(() => jumpTo(sectionId(section), { focus: 'h2' }), 0);
    return () => clearTimeout(timer);
  });

  /** The plan's "Make a household plan" step, if the plan has one. */
  const householdStep = $derived(
    app.result.output ? allPlanItems(app.result.output).find((x) => HOUSEHOLD_PLAN_STEPS.includes(x.item.item_id))?.item : undefined,
  );

  function note(key: FamilyNote, text: string) {
    if (app.plan) setNote(app.plan.input, key, text);
  }
  function noteLeft(key: FamilyNote) {
    if (app.plan) tidyNote(app.plan.input, key);
  }
  function contact(key: FamilyContact, part: 'name' | 'phone', text: string) {
    if (app.plan) setContactPart(app.plan.input, key, part, text);
  }
  function contactLeft(key: FamilyContact, part: 'name' | 'phone') {
    if (app.plan) tidyContactPart(app.plan.input, key, part);
  }

  async function newNumber() {
    if (!app.plan || !addNumber(app.plan.input)) return;
    const n = app.plan.input.family_plan?.numbers_by_heart?.length ?? 0;
    status = `Number ${n} added.`;
    await tick();
    document.getElementById(`fp-number-${n - 1}`)?.focus();
  }

  function dropNumber(i: number) {
    if (!app.plan) return;
    removeNumber(app.plan.input, i);
    status = `Number ${i + 1} removed.`;
    document.getElementById('fp-add-number')?.focus();
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

  function markHouseholdStepDone() {
    if (!householdStep || householdStep.done) return;
    app.record(householdStep);
    status = `Marked as done on your plan: ${householdStep.name}.`;
  }

  const ROUTE_LABELS = ['First way out', 'Second way out'];

  const withChildren = $derived(!!input && (hasChildren(input) || !!plan?.school_pickup));
  const titles = $derived<Record<FamilySection, string>>({
    contact: 'Staying in touch',
    children: withChildren ? 'Children, school and work' : 'Work and school',
    shelter: 'Where to shelter',
    leave: 'If you have to leave',
    home: 'Around the home',
    circle: 'Your trusted circle',
    lawyer: 'A lawyer',
  });
</script>

<div class="page page--narrow">
  <h1 id="page-title" tabindex="-1">Your family plan</h1>
  <p class="lead">
    Where to meet, who to call, where to shelter and who helps, in your own words. It prints at the front of your packet and on a wallet card
    for each person.
  </p>

  {#if !app.plan || !input}
    <div class="card gate">
      <p>You haven't started a plan on this device yet. It takes about ten minutes, and nothing you enter leaves your browser.</p>
      <p class="button-row"><a class="button button--primary" href={href('start')}>Start a plan</a></p>
    </div>
  {:else}
    <div class="privacy">
      <p><Icon name="lock" /> <span>Saved only in this browser and in the plan file you save. Nothing here is sent anywhere or used to work out your plan.</span></p>
      <p class="small">
        Names and phone numbers are other people's details too, so keep a saved file somewhere safe. Until Ready Reckoner has a web address of
        its own, other pages at the same address could in principle read what this browser keeps. If that matters to you, write this plan on
        the printed packet instead.
      </p>
    </div>
    <p class="section-intro">Everything is optional. Fill in what you know now; anything left blank prints as a line to write on.</p>

    <nav class="toc no-print" aria-label="Parts of your family plan">
      <ul>
        {#each FAMILY_SECTIONS as section (section)}
          <li>
            <a
              href="#{sectionId(section)}"
              onclick={(e) => {
                if (jumpTo(sectionId(section), { focus: 'h2' })) e.preventDefault();
              }}>{titles[section]}</a
            >
          </li>
        {/each}
      </ul>
    </nav>

    <section id={sectionId('contact')} aria-labelledby="fp-contact-title">
      <h2 id="fp-contact-title">{titles.contact}</h2>
      <p class="section-intro">Phones fail and networks jam in a disaster. Decide now how you will reach each other and where you will meet.</p>
      <PlanGuide
        id="plan_communication"
        summary="How to plan staying in touch"
        fallback="Plan four ways to reach each other, each one for when the one before fails: call, then text, then everyone texts one contact out of the area, then go to the place you agreed to meet. Write the numbers on a card for each person, and learn two or three by heart."
      />
      <fieldset class="group" aria-describedby="fp-contact-help">
        <legend>Someone out of the area everyone checks in with</legend>
        <p class="help" id="fp-contact-help">Everyone texts this person to say they are safe, and they pass the news along. A call out of the area may get through when a local one doesn't.</p>
        <div class="pair">
          <TextField
            id="fp-contact-name"
            label="Name"
            context="of the contact out of the area"
            value={plan?.out_of_area_contact?.name}
            maxlength={FAMILY_PLAN_SHORT_MAX}
            oninput={(t) => contact('out_of_area_contact', 'name', t)}
            onleave={() => contactLeft('out_of_area_contact', 'name')}
          />
          <TextField
            id="fp-contact-phone"
            label="Phone"
            context="of the contact out of the area"
            value={plan?.out_of_area_contact?.phone}
            maxlength={FAMILY_PLAN_SHORT_MAX}
            inputmode="tel"
            oninput={(t) => contact('out_of_area_contact', 'phone', t)}
            onleave={() => contactLeft('out_of_area_contact', 'phone')}
          />
        </div>
      </fieldset>
      <TextField
        id="fp-meet-near"
        label="Where to meet near home"
        help="If you can't get back inside: a neighbour's porch, the corner mailbox."
        value={plan?.meeting_place_near}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('meeting_place_near', t)}
        onleave={() => noteLeft('meeting_place_near')}
      />
      <TextField
        id="fp-meet-far"
        label="Where to meet outside the neighbourhood"
        help="If you can't get home at all: a library, a relative's house, a place of worship."
        value={plan?.meeting_place_far}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('meeting_place_far', t)}
        onleave={() => noteLeft('meeting_place_far')}
      />
      <fieldset class="group" aria-describedby="fp-numbers-help">
        <legend>Numbers to know by heart</legend>
        <p class="help" id="fp-numbers-help">Two or three numbers everyone learns, in case a phone is lost or dead. Up to {NUMBERS_BY_HEART_MAX}.</p>
        {#each plan?.numbers_by_heart ?? [] as number, i (i)}
          <div class="entry">
            <div class="entry__field">
              <label for="fp-number-{i}">Number {i + 1}</label>
              <input
                id="fp-number-{i}"
                class="input input--medium"
                type="text"
                inputmode="tel"
                autocomplete="off"
                maxlength={FAMILY_PLAN_SHORT_MAX}
                value={number}
                oninput={(e) => app.plan && setNumber(app.plan.input, i, (e.currentTarget as HTMLInputElement).value)}
                onblur={() => app.plan && tidyNumber(app.plan.input, i)}
              />
            </div>
            <button type="button" class="button button--quiet button--small" onclick={() => dropNumber(i)}>
              <Icon name="trash" /> Remove<span class="visually-hidden">{' '}number {i + 1}</span>
            </button>
          </div>
        {/each}
        {#if (plan?.numbers_by_heart?.length ?? 0) < NUMBERS_BY_HEART_MAX}
          <button id="fp-add-number" type="button" class="button button--small" onclick={newNumber}><Icon name="plus" /> Add a number</button>
        {:else}
          <p class="small muted" id="fp-add-number" tabindex="-1">That is the most the wallet card holds.</p>
        {/if}
      </fieldset>
    </section>

    <section id={sectionId('children')} aria-labelledby="fp-children-title">
      <h2 id="fp-children-title">{titles.children}</h2>
      {#if withChildren}
        <TextField
          id="fp-school"
          label="Who picks up the children, and from where"
          help="Schools and child care release children only to adults on their list. Keep that list up to date."
          value={plan?.school_pickup}
          maxlength={FAMILY_PLAN_TEXT_MAX}
          multiline
          oninput={(t) => note('school_pickup', t)}
          onleave={() => noteLeft('school_pickup')}
        />
      {/if}
      <TextField
        id="fp-work"
        label="What each person does at work or school"
        help="Stay put, come home, or follow the workplace's own plan."
        value={plan?.work_plans}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('work_plans', t)}
        onleave={() => noteLeft('work_plans')}
      />
    </section>

    <section id={sectionId('shelter')} aria-labelledby="fp-shelter-title">
      <h2 id="fp-shelter-title">{titles.shelter}</h2>
      <p class="section-intro">Where to shelter safely is what people most want to know. Pick the spot now, for the dangers where you live.</p>
      <PlanGuide
        id="plan_shelter"
        summary="Where to shelter, danger by danger"
        fallback="For most storms: a small inside room on the lowest floor, away from windows. In a flood, go up, never down into a basement. For a chemical release, a room above ground you can seal. Follow local officials first."
      />
      <TextField
        id="fp-shelter-home"
        label="The safest spot at home"
        help="Where everyone goes when a warning comes."
        value={plan?.shelter_spot_home}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('shelter_spot_home', t)}
        onleave={() => noteLeft('shelter_spot_home')}
      />
      <TextField
        id="fp-shelter-work"
        label="The safest spot at work or school"
        help="Ask where the building's shelter area is."
        value={plan?.shelter_spot_work}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('shelter_spot_work', t)}
        onleave={() => noteLeft('shelter_spot_work')}
      />
    </section>

    <section id={sectionId('leave')} aria-labelledby="fp-leave-title">
      <h2 id="fp-leave-title">{titles.leave}</h2>
      <TextField
        id="fp-go"
        label="Where you would go"
        help="A friend or relative out of the area, or a town with hotels."
        value={plan?.where_we_would_go}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('where_we_would_go', t)}
        onleave={() => noteLeft('where_we_would_go')}
      />
      <fieldset class="group" aria-describedby="fp-routes-help">
        <legend>Two ways out</legend>
        <p class="help" id="fp-routes-help">Two different roads or transit lines, in case one is closed.{hasVehicle(input) ? '' : ' Without a car, name the bus or train, or who would drive you.'}</p>
        {#each ROUTE_LABELS.slice(0, ROUTES_MAX) as label, i (i)}
          <TextField
            id="fp-route-{i}"
            {label}
            value={plan?.routes?.[i]}
            maxlength={FAMILY_PLAN_TEXT_MAX}
            multiline
            oninput={(t) => app.plan && setRoute(app.plan.input, i, t)}
            onleave={() => app.plan && tidyRoute(app.plan.input, i)}
          />
        {/each}
      </fieldset>
      {#if hasAnimals(input) || plan?.who_takes_animals}
        <TextField
          id="fp-animals"
          label="Who takes the animals if you can't"
          help="Someone who has agreed, and knows where the carriers, leashes and food are."
          value={plan?.who_takes_animals}
          maxlength={FAMILY_PLAN_TEXT_MAX}
          multiline
          oninput={(t) => note('who_takes_animals', t)}
          onleave={() => noteLeft('who_takes_animals')}
        />
      {/if}
      {#if hasVehicle(input) || plan?.roadside_assistance}
        <TextField
          id="fp-roadside"
          label="Roadside assistance number"
          help="From your insurer, an auto club or a card. Check whether it covers a tow, and how far."
          value={plan?.roadside_assistance}
          maxlength={FAMILY_PLAN_SHORT_MAX}
          inputmode="tel"
          width="medium"
          oninput={(t) => app.plan && setRoadside(app.plan.input, t)}
          onleave={() => app.plan && tidyRoadside(app.plan.input)}
        />
      {/if}
    </section>

    <section id={sectionId('home')} aria-labelledby="fp-home-title">
      <h2 id="fp-home-title">{titles.home}</h2>
      <p class="section-intro">A gas leak or a burst pipe does far less harm if anyone at home can shut it off in seconds.</p>
      <TextField
        id="fp-gas"
        label="Gas shut-off"
        help={hasGas(input) ? 'Where it is, and the tool that turns it.' : 'Where it is, and the tool that turns it. Leave it blank if you have no gas.'}
        value={plan?.shutoff_gas}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('shutoff_gas', t)}
        onleave={() => noteLeft('shutoff_gas')}
      />
      <TextField
        id="fp-water"
        label="Main water shut-off"
        help="Often near the water meter, where the pipe comes into the home."
        value={plan?.shutoff_water}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('shutoff_water', t)}
        onleave={() => noteLeft('shutoff_water')}
      />
      <TextField
        id="fp-electric"
        label="Electrical panel or main breaker"
        help="Where it is, and which switch turns off everything."
        value={plan?.shutoff_electric}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('shutoff_electric', t)}
        onleave={() => noteLeft('shutoff_electric')}
      />
      <TextField
        id="fp-neighbours"
        label="Neighbours who check on you, and whom you check on"
        help="Neighbours are the first help in most disasters."
        value={plan?.neighbours_who_check}
        maxlength={FAMILY_PLAN_TEXT_MAX}
        multiline
        oninput={(t) => note('neighbours_who_check', t)}
        onleave={() => noteLeft('neighbours_who_check')}
      />
    </section>

    <section id={sectionId('circle')} aria-labelledby="fp-circle-title">
      <h2 id="fp-circle-title">{titles.circle}</h2>
      <p class="section-intro">
        Up to {TRUSTED_CIRCLE_MAX} people who have agreed ahead of time to help each other: a bed for a night, a ride, care for children or
        pets. Agree it on a calm day; in a crisis there is no time to ask.
      </p>
      {#if plan?.trusted_circle?.length}
        <ol class="circle">
          {#each plan.trusted_circle as person, i (i)}
            <li class="card circle__person">
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

    <section id={sectionId('lawyer')} aria-labelledby="fp-lawyer-title">
      <h2 id="fp-lawyer-title">{titles.lawyer}</h2>
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

    <section class="card finish" aria-labelledby="fp-finish-title">
      <h2 id="fp-finish-title">Put it on paper</h2>
      <p>Your packet prints this plan first, then a wallet card for each person with the numbers and meeting places on it.</p>
      <p class="button-row">
        <a class="button button--primary" href={href('packet', 'wallet-cards')}><Icon name="print" /> Print wallet cards</a>
        <a class="button" href={href('packet')}>See the whole packet</a>
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
  {/if}
</div>

<style>
  .gate {
    max-width: var(--w-text);
  }
  .privacy {
    padding: var(--s3) var(--s4);
    background: var(--surface-2);
    border-radius: var(--r2);
    margin-bottom: var(--s4);
  }
  .privacy p {
    margin: 0;
  }
  .privacy p:first-child {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    font-weight: 560;
    margin-bottom: var(--s2);
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
  .group {
    margin-bottom: var(--s5);
  }
  .group > :global(.field:last-child) {
    margin-bottom: 0;
  }
  /* A name and a phone side by side when there is room; the grid gap spaces them either way. */
  .pair {
    display: grid;
    gap: var(--s3) var(--s4);
    margin-bottom: var(--s5);
  }
  .pair :global(.field) {
    margin-bottom: 0;
  }
  .group > .pair:last-child {
    margin-bottom: 0;
  }
  @media (min-width: 36rem) {
    .pair {
      grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
    }
  }
  .entry {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--s2) var(--s3);
    margin-bottom: var(--s3);
  }
  .entry__field {
    flex: 0 1 20rem;
  }
  .entry__field label {
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
  .finish {
    margin-top: var(--s6);
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
