<!--
  Step 7, Your places (#/places; optional, DESIGN-DELTA-v3 §2.2): four cards.

  - Your home (`family_plan.home`, contract v3): the street address; electricity, gas and water,
    each with its company, its outage number and where it shuts off (the v2 shut-off questions);
    the home insurer and policy number; the landlord or mortgage company; where the kit, the
    documents, the cash and the spare keys are; the safest spot at home (with the reviewed
    plan_shelter guide) and the neighbours who check on you (v2).
  - Meeting places and staying in touch (v2, unchanged): the out-of-area contact, where to meet near
    home and outside the neighbourhood, numbers to know by heart (with the plan_communication
    guide), who picks up the children (households with children), what each person does at work or
    school and the safest spot there, kept as the general answers (each person's own place is on
    step 6).
  - Your neighbourhood (`family_plan.neighbourhood`): the nearest hospital with an emergency room,
    urgent care, the pharmacy, the community shelter, the county emergency management office and
    how the household gets local alerts.
  - Getting out (v2, unchanged): where you would go, two ways out, the roadside number (households
    with a vehicle).

  `#/places/<card>` opens the step at one card (`home`, `touch`, `neighbourhood`, `leave`); the old
  family-plan addresses redirect here. Every field is optional free text, saved as typed and
  tidied when left the way the engine tidies it; nothing blocks Continue.
-->
<script lang="ts">
  import ContactFields from '../components/ContactFields.svelte';
  import Icon from '../components/Icon.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import PlanGuide from '../components/PlanGuide.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import StepPrivacy from '../components/StepPrivacy.svelte';
  import TextField from '../components/TextField.svelte';
  import type { Contact } from '../engine/types';
  import { FAMILY_PLAN_SHORT_MAX, FAMILY_PLAN_TEXT_MAX, NUMBERS_BY_HEART_MAX, ROUTES_MAX } from '../engine/types';
  import { jumpTo } from '../lib/anchors';
  import { useApp } from '../lib/app.svelte';
  import {
    addNumber,
    type FamilyContact,
    type FamilyNote,
    hasChildren,
    hasGas,
    hasVehicle,
    removeNumber,
    setContactPart,
    setNote,
    setNumber,
    setPlanText,
    setRoadside,
    setRoute,
    tidyContactPart,
    tidyNote,
    tidyNumber,
    tidyPlanText,
    tidyRoadside,
    tidyRoute,
  } from '../lib/family';
  import { href, useRouter } from '../lib/router.svelte';
  import { HOME_MAX, NEIGHBOURHOOD_MAX, type Path } from '../lib/tidy';
  import { tick } from 'svelte';

  const app = useApp();
  const router = useRouter();
  const input = $derived(app.plan?.input);
  const plan = $derived(input?.family_plan);
  const home = $derived(plan?.home);
  const hood = $derived(plan?.neighbourhood);
  const county = $derived(app.result.output?.location.county_name);
  let status = $state('');

  /** The cards, each reachable as `#/places/<card>`. */
  const CARDS = ['home', 'touch', 'neighbourhood', 'leave'] as const;

  // Opened at one card (#/places/home): go there once the page has settled.
  $effect(() => {
    const card = router.current.id === 'places' ? router.current.param : undefined;
    if (!card || !(CARDS as readonly string[]).includes(card) || !app.plan) return;
    const timer = setTimeout(() => jumpTo(`places-${card}`, { focus: 'h2' }), 0);
    return () => clearTimeout(timer);
  });

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
  function edit(path: Path, text: string) {
    if (app.plan) setPlanText(app.plan.input, path, text);
  }
  function leave(path: Path, max: number) {
    if (app.plan) tidyPlanText(app.plan.input, path, max);
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

  const ROUTE_LABELS = ['First way out', 'Second way out'];
  const withChildren = $derived(!!input && (hasChildren(input) || !!plan?.school_pickup));
  const renting = $derived(input?.housing.tenure === 'rent');
</script>

<!-- A contact inside the home or neighbourhood group at `path` (for example ['home', 'insurer']). -->
{#snippet place(path: Path, value: Contact | undefined, legend: string, context: string, opts: { help?: string; address?: boolean; labels?: Partial<Record<'name' | 'phone' | 'address', string>> } = {})}
  <ContactFields
    id="pl-{path.join('-')}"
    {legend}
    help={opts.help}
    {value}
    {context}
    labels={opts.labels}
    address={opts.address}
    oninput={(part, t) => edit([...path, part], t)}
    onleave={(part, max) => leave([...path, part], max)}
  />
{/snippet}

<div class="page page--narrow">
  <ProgressSteps current="places" />
  <h1 id="page-title" tabindex="-1">Your places</h1>

  {#if !app.plan || !input}
    <div class="card gate">
      <p>You haven't started a plan on this device yet. It takes about ten minutes, and nothing you enter leaves your browser.</p>
      <p class="button-row"><a class="button button--primary" href={href('start')}>Start a plan</a></p>
    </div>
  {:else}
    <StepPrivacy>
      Everything here is optional. Your answers fill the pages about your home, your neighbourhood and getting out in your binder; anything
      left blank prints as a line to fill in by hand. They are kept only on this device and in any plan file you save, never sent anywhere,
      and never used to work out your plan.
    </StepPrivacy>

    <nav class="toc no-print" aria-label="Parts of this step">
      <ul>
        <li><a href="#places-home" onclick={(e) => jumpTo('places-home', { focus: 'h2' }) && e.preventDefault()}>Your home</a></li>
        <li><a href="#places-touch" onclick={(e) => jumpTo('places-touch', { focus: 'h2' }) && e.preventDefault()}>Meeting places and staying in touch</a></li>
        <li><a href="#places-neighbourhood" onclick={(e) => jumpTo('places-neighbourhood', { focus: 'h2' }) && e.preventDefault()}>Your neighbourhood</a></li>
        <li><a href="#places-leave" onclick={(e) => jumpTo('places-leave', { focus: 'h2' }) && e.preventDefault()}>Getting out</a></li>
      </ul>
    </nav>

    <section id="places-home" class="card step-card" aria-labelledby="pl-home-title">
      <h2 id="pl-home-title">Your home</h2>
      <p class="section-intro">What anyone at home, or a helper, needs to know about the place itself. A gas leak or a burst pipe does far less harm if anyone at home can shut it off in seconds.</p>
      <TextField
        id="pl-home-address"
        label="Street address"
        help="As you would give it to a 911 dispatcher, with the apartment or unit number."
        value={home?.address}
        maxlength={HOME_MAX.address}
        oninput={(t) => edit(['home', 'address'], t)}
        onleave={() => leave(['home', 'address'], HOME_MAX.address)}
      />
      {@render utility('electric_utility', 'Electricity', 'Electric company', 'the electric company', 'shutoff_electric', 'Electrical panel or main breaker', 'Where it is, and which switch turns off everything.')}
      {@render utility(
        'gas_utility',
        'Gas',
        'Gas company',
        'the gas company',
        'shutoff_gas',
        'Gas shut-off',
        hasGas(input) ? 'Where it is, and the tool that turns it.' : 'Where it is, and the tool that turns it. Leave it blank if you have no gas.',
      )}
      {@render utility('water_utility', 'Water', 'Water company', 'the water company', 'shutoff_water', 'Main water shut-off', 'Often near the water meter, where the pipe comes into the home.')}
      <ContactFields
        id="pl-home-insurer"
        legend={renting ? 'Renters insurance' : 'Home insurance'}
        help="Who to call to start a claim."
        value={home?.insurer}
        context="of the home insurer"
        labels={{ name: 'Insurance company', phone: 'Claims phone' }}
        oninput={(part, t) => edit(['home', 'insurer', part], t)}
        onleave={(part, max) => leave(['home', 'insurer', part], max)}
      >
        <TextField
          id="pl-home-policy"
          label="Policy number"
          context="of the home insurance"
          value={home?.policy_number}
          maxlength={HOME_MAX.policy_number}
          width="medium"
          oninput={(t) => edit(['home', 'policy_number'], t)}
          onleave={() => leave(['home', 'policy_number'], HOME_MAX.policy_number)}
        />
      </ContactFields>
      {@render place(['home', 'landlord_or_mortgage'], home?.landlord_or_mortgage, 'Landlord or mortgage company', 'of the landlord or mortgage company', {
        help: renting ? 'Who to call about damage and repairs.' : 'Call them after a disaster about your payments.',
      })}
      <fieldset class="group" aria-describedby="pl-where-help">
        <legend>Where things are</legend>
        <p class="help" id="pl-where-help">So anyone at home can find them fast, even in the dark.</p>
        {@render where('where_kit', 'Where the emergency kit is')}
        {@render where('where_documents', 'Where the documents are')}
        {@render where('where_cash', 'Where the cash is')}
        {@render where('where_keys', 'Where the spare keys are')}
      </fieldset>
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

    <section id="places-touch" class="card step-card" aria-labelledby="pl-touch-title">
      <h2 id="pl-touch-title">Meeting places and staying in touch</h2>
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
      <p class="small muted">Each person's own school, work or child care, with its phone and pick-up rules, goes on <a href={href('people')}>Your people</a>.</p>
    </section>

    <section id="places-neighbourhood" class="card step-card" aria-labelledby="pl-hood-title">
      <h2 id="pl-hood-title">Your neighbourhood</h2>
      <p class="section-intro">The places nearby you may need at short notice. Look them up once, on a calm day.</p>
      {@render place(['neighbourhood', 'hospital'], hood?.hospital, 'Nearest hospital with an emergency room', 'of the nearest hospital', { address: true })}
      {@render place(['neighbourhood', 'urgent_care'], hood?.urgent_care, 'Urgent care', 'of the urgent care clinic', {
        help: "For injuries and illness that can't wait but are not an emergency.",
        address: true,
      })}
      {@render place(['neighbourhood', 'pharmacy'], hood?.pharmacy, 'The pharmacy you use', 'of the pharmacy', { address: true })}
      {@render place(['neighbourhood', 'shelter'], hood?.shelter, 'Where the community opens a shelter', 'of the community shelter', {
        help: 'Often a school or a community center. Your county emergency office can tell you.',
        address: true,
      })}
      {@render place(['neighbourhood', 'county_emergency_office'], hood?.county_emergency_office, 'County emergency management office', 'of the county emergency office', {
        help: county ? `The office that plans for emergencies in ${county}.` : 'The office that plans for emergencies in your county.',
      })}
      <TextField
        id="pl-neighbourhood-alerts"
        label="How you get local alerts"
        help="Your county's alert sign-up, a local radio station, or both."
        value={hood?.alerts}
        maxlength={NEIGHBOURHOOD_MAX.alerts}
        multiline
        oninput={(t) => edit(['neighbourhood', 'alerts'], t)}
        onleave={() => leave(['neighbourhood', 'alerts'], NEIGHBOURHOOD_MAX.alerts)}
      />
    </section>

    <section id="places-leave" class="card step-card" aria-labelledby="pl-leave-title">
      <h2 id="pl-leave-title">Getting out</h2>
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
      <!-- awaiting: web-maps — its PinMapButton ("Set your home point and meeting places on a map", DESIGN-DELTA-v3 §2.2, §9.4) goes here once both branches are on v0.3. -->
    </section>

    <p class="visually-hidden" aria-live="polite">{status}</p>
    <InterviewNav step="places" />
  {/if}
</div>

<!-- Electricity, gas or water: the company and its outage number, then where it shuts off (the v2 question). -->
{#snippet utility(key: 'electric_utility' | 'gas_utility' | 'water_utility', legend: string, company: string, who: string, shutoff: FamilyNote, shutoffLabel: string, shutoffHelp: string)}
  <ContactFields
    id="pl-home-{key}"
    {legend}
    value={home?.[key]}
    context="of {who}"
    labels={{ name: company, phone: 'Outage number' }}
    oninput={(part, t) => edit(['home', key, part], t)}
    onleave={(part, max) => leave(['home', key, part], max)}
  >
    <TextField
      id={shutoff === 'shutoff_gas' ? 'fp-gas' : shutoff === 'shutoff_water' ? 'fp-water' : 'fp-electric'}
      label={shutoffLabel}
      help={shutoffHelp}
      value={plan?.[shutoff]}
      maxlength={FAMILY_PLAN_TEXT_MAX}
      multiline
      oninput={(t) => note(shutoff, t)}
      onleave={() => noteLeft(shutoff)}
    />
  </ContactFields>
{/snippet}

<!-- "Where the … is" in the home group. -->
{#snippet where(key: 'where_kit' | 'where_documents' | 'where_cash' | 'where_keys', label: string)}
  <TextField
    id="pl-home-{key}"
    {label}
    value={home?.[key]}
    maxlength={HOME_MAX[key]}
    oninput={(t) => edit(['home', key], t)}
    onleave={() => leave(['home', key], HOME_MAX[key])}
  />
{/snippet}

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
  /* A group's title reads as a small heading above the answers it groups. */
  .group > legend {
    font-size: var(--text-lg);
  }
  .group > :global(.field:last-child) {
    margin-bottom: 0;
  }
  /* A name and a phone side by side when there is room; the grid gap spaces them either way. */
  .pair {
    display: grid;
    gap: var(--s3) var(--s4);
    /* Inputs line up even when one answer has help text and its neighbour has none. */
    align-items: end;
  }
  .pair :global(.field) {
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
</style>
