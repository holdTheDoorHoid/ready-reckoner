<!--
  Step 6, Your people (#/people; optional, DESIGN-DELTA-v3 §2.1): one card per person from step 2,
  in the same order, headed "Person 1 (adult)" until a name is given and by the name after. Each
  card asks, in this order: name or nickname, date of birth, phone, email; where they spend the day
  (the kind of place, its name, address, phone, its own emergency plan, pick-up rules and the safest
  spot there); doctor and pharmacy; conditions, medicines (up to 12), allergies and blood type;
  health insurance and ID notes; and anything else a helper should know.

  The answers live on the person (contract v3 `Person.profile`), so a person removed on step 2
  takes theirs along. Every field is optional and echo-only: saved as typed, tidied when left the
  way the engine tidies it, printed on the person's page in the binder and on their wallet card,
  never used to work anything out and never checked. Nothing here stops Continue, and "Skip for
  now" moves on without marking the step answered. The groups after the first four fields fold
  away; a group that already holds answers starts open.
-->
<script lang="ts">
  import ChoiceGroup from '../components/ChoiceGroup.svelte';
  import ContactFields from '../components/ContactFields.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import RowList from '../components/RowList.svelte';
  import StepPrivacy from '../components/StepPrivacy.svelte';
  import TextField from '../components/TextField.svelte';
  import type { Person } from '../engine/types';
  import { PLACE_KINDS, type PersonProfile } from '../engine/v3-shim';
  import { useApp } from '../lib/app.svelte';
  import {
    addMedication,
    PLACE_KIND,
    personHeading,
    personRef,
    removeMedication,
    setPlaceKind,
    setProfileText,
    tidyProfile,
    tidyProfileText,
  } from '../lib/profile';
  import { href } from '../lib/router.svelte';
  import { HEALTH_INSURANCE_MAX, MEDICATION_MAX, MEDICATIONS_MAX, type Path, PLACE_MAX, PROFILE_MAX, rowCount, textAt } from '../lib/tidy';

  const app = useApp();
  const input = $derived(app.plan?.input);
  let status = $state('');

  interface Groups {
    place: boolean;
    care: boolean;
    health: boolean;
    cover: boolean;
  }

  /** Which folded groups of a profile hold answers. */
  function groupsOf(profile: PersonProfile | undefined): Groups {
    const p = tidyProfile(profile);
    return {
      place: !!p?.place,
      care: !!(p?.doctor || p?.pharmacy),
      health: !!(p?.conditions || p?.medications || p?.allergies || p?.blood_type),
      cover: !!(p?.insurance || p?.id_notes),
    };
  }

  // Groups with answers start open. Read once: a group never folds itself while someone types in it.
  const openAtStart: Groups[] = (app.plan?.input.people ?? []).map((p) => groupsOf(p.profile));

  function medicineLegend(person: Person, i: number, m: number): string {
    const name = person.profile?.medications?.[m]?.name?.trim();
    return name ? `${name} (medicine ${m + 1} for ${personRef(person, i)})` : `Medicine ${m + 1} for ${personRef(person, i)}`;
  }
</script>

<!-- One free-text answer in person `i`'s profile, at `path` (for example ['doctor', 'phone']). -->
{#snippet text(
  person: Person,
  i: number,
  path: Path,
  label: string,
  max: number,
  opts: { help?: string; context?: string; multiline?: boolean; inputmode?: 'tel' | 'email' | 'text'; width?: 'short' | 'medium' } = {},
)}
  <TextField
    id="pp-{i}-{path.join('-')}"
    {label}
    help={opts.help}
    context={opts.context ?? `for ${personRef(person, i)}`}
    value={textAt(person.profile, path)}
    maxlength={max}
    multiline={opts.multiline}
    inputmode={opts.inputmode}
    width={opts.width}
    oninput={(t) => setProfileText(person, path, t)}
    onleave={() => tidyProfileText(person, path, max)}
  />
{/snippet}

<div class="page page--narrow">
  <ProgressSteps current="people" />
  <h1 id="page-title" tabindex="-1">Your people</h1>

  {#if !app.plan || !input}
    <div class="card gate">
      <p>You haven't started a plan on this device yet. It takes about ten minutes, and nothing you enter leaves your browser.</p>
      <p class="button-row"><a class="button button--primary" href={href('start')}>Start a plan</a></p>
    </div>
  {:else}
    <StepPrivacy>
      Everything here is optional. Your answers fill a page for each person in your binder, and a wallet card for each of them; anything
      left blank prints as a line to fill in by hand. They are kept only on this device and in any plan file you save, never sent anywhere,
      and never used to work out your plan.
    </StepPrivacy>

    <ol class="people">
      {#each input.people as person, i (i)}
        {@const ref = personRef(person, i)}
        {@const has = groupsOf(person.profile)}
        {@const open = openAtStart[i]}
        <li class="card person" id="person-{i + 1}">
          <h2 id="pp-{i}-title" class="person__title">{personHeading(person, i)}</h2>
          <div class="pair">
            {@render text(person, i, ['name'], 'Name or nickname', PROFILE_MAX.name, { help: 'As it should appear on their page and wallet card.' })}
            {@render text(person, i, ['date_of_birth'], 'Date of birth', PROFILE_MAX.date_of_birth, { help: 'Any way you write it, for example 3 March 1984.' })}
            {@render text(person, i, ['phone'], 'Phone', PROFILE_MAX.phone, { inputmode: 'tel' })}
            {@render text(person, i, ['email'], 'Email', PROFILE_MAX.email, { inputmode: 'email' })}
          </div>

          <details class="group" open={open?.place}>
            <summary>Where {ref} spends the day{has.place ? '' : ': nothing yet'}</summary>
            <div class="group__body">
              <ChoiceGroup
                legend="What kind of place"
                name="pp-{i}-place-kind"
                help="Where {ref} is on most weekdays."
                options={PLACE_KINDS.map((k) => ({ value: k, label: PLACE_KIND[k].label }))}
                value={person.profile?.place?.kind}
                onchange={(kind) => setPlaceKind(person, kind)}
                columns={2}
              />
              {@render text(person, i, ['place', 'name'], 'Name of the place', PLACE_MAX.name, { help: 'For example the name of the school or the company.' })}
              {@render text(person, i, ['place', 'address'], 'Address', PLACE_MAX.address, { context: `of where ${ref} spends the day` })}
              {@render text(person, i, ['place', 'phone'], 'Phone', PLACE_MAX.phone, { help: 'The main office or front desk.', context: `of where ${ref} spends the day`, inputmode: 'tel', width: 'medium' })}
              {@render text(person, i, ['place', 'plan'], 'Its own emergency plan', PLACE_MAX.plan, {
                help: 'What the place says it will do: keep everyone inside, move to another site, or send people home. Ask for it.',
                multiline: true,
              })}
              {@render text(person, i, ['place', 'pickup'], 'Pick-up rules', PLACE_MAX.pickup, {
                help: 'Who may collect them there, and what the place needs to see, such as a name on its list or photo ID.',
                multiline: true,
              })}
              {@render text(person, i, ['place', 'safest_spot'], 'The safest spot there', PLACE_MAX.safest_spot, {
                help: 'Where people shelter in that building when a warning comes.',
                multiline: true,
              })}
            </div>
          </details>

          <details class="group" open={open?.care}>
            <summary>Doctor and pharmacy{has.care ? '' : ': nothing yet'}</summary>
            <div class="group__body">
              <ContactFields
                id="pp-{i}-doctor"
                legend="Doctor"
                help="Their main doctor or clinic."
                value={person.profile?.doctor}
                context="of {ref}'s doctor"
                oninput={(part, t) => setProfileText(person, ['doctor', part], t)}
                onleave={(part, max) => tidyProfileText(person, ['doctor', part], max)}
              />
              <ContactFields
                id="pp-{i}-pharmacy"
                legend="Pharmacy"
                help="Where they fill prescriptions."
                value={person.profile?.pharmacy}
                context="of {ref}'s pharmacy"
                oninput={(part, t) => setProfileText(person, ['pharmacy', part], t)}
                onleave={(part, max) => tidyProfileText(person, ['pharmacy', part], max)}
              />
            </div>
          </details>

          <details class="group" open={open?.health}>
            <summary>Health and medicines{has.health ? '' : ': nothing yet'}</summary>
            <div class="group__body">
              {@render text(person, i, ['conditions'], 'Medical conditions', PROFILE_MAX.conditions, {
                help: 'Anything a helper or a hospital should know, for example diabetes, asthma or a heart condition.',
                multiline: true,
              })}
              <fieldset class="medicines" aria-describedby="pp-{i}-medicines-help">
                <legend>Medicines</legend>
                <p class="help" id="pp-{i}-medicines-help">
                  Each medicine as it is on the label: its name, the dose, when it is taken and what for. Up to {MEDICATIONS_MAX}.
                </p>
                <RowList
                  id="pp-{i}-med"
                  count={rowCount(person, ['profile', 'medications'])}
                  max={MEDICATIONS_MAX}
                  legend={(m) => medicineLegend(person, i, m)}
                  addLabel="Add a medicine"
                  fullText="That is {MEDICATIONS_MAX} medicines, the most the binder keeps for one person."
                  onadd={() => addMedication(person)}
                  onremove={(m) => removeMedication(person, m)}
                  onstatus={(s) => (status = s)}
                >
                  {#snippet row(m)}
                    <div class="pair">
                      {@render text(person, i, ['medications', m, 'name'], 'Medicine', MEDICATION_MAX.name, { context: `(medicine ${m + 1} for ${ref})` })}
                      {@render text(person, i, ['medications', m, 'dose'], 'Dose', MEDICATION_MAX.dose, { context: `(medicine ${m + 1} for ${ref})` })}
                      {@render text(person, i, ['medications', m, 'schedule'], 'When taken', MEDICATION_MAX.schedule, { context: `(medicine ${m + 1} for ${ref})` })}
                      {@render text(person, i, ['medications', m, 'purpose'], 'What for', MEDICATION_MAX.purpose, { context: `(medicine ${m + 1} for ${ref})` })}
                    </div>
                  {/snippet}
                </RowList>
              </fieldset>
              {@render text(person, i, ['allergies'], 'Allergies', PROFILE_MAX.allergies, {
                help: 'To medicines, foods or stings, and what happens.',
                multiline: true,
              })}
              {@render text(person, i, ['blood_type'], 'Blood type', PROFILE_MAX.blood_type, { help: 'If you know it, for example O+.', width: 'short' })}
            </div>
          </details>

          <details class="group" open={open?.cover}>
            <summary>Insurance and ID{has.cover ? '' : ': nothing yet'}</summary>
            <div class="group__body">
              <fieldset class="insurance" aria-describedby="pp-{i}-insurance-help">
                <legend>Health insurance</legend>
                <p class="help" id="pp-{i}-insurance-help">As it is on the insurance card.</p>
                <div class="pair">
                  {@render text(person, i, ['insurance', 'carrier'], 'Insurance company', HEALTH_INSURANCE_MAX.carrier)}
                  {@render text(person, i, ['insurance', 'plan_name'], 'Plan name', HEALTH_INSURANCE_MAX.plan_name)}
                  {@render text(person, i, ['insurance', 'member_id'], 'Member ID', HEALTH_INSURANCE_MAX.member_id)}
                  {@render text(person, i, ['insurance', 'group_number'], 'Group number', HEALTH_INSURANCE_MAX.group_number)}
                  {@render text(person, i, ['insurance', 'phone'], 'Phone on the card', HEALTH_INSURANCE_MAX.phone, { inputmode: 'tel' })}
                </div>
              </fieldset>
              {@render text(person, i, ['id_notes'], 'ID notes', PROFILE_MAX.id_notes, {
                help: 'A passport number, or where it is kept.',
                multiline: true,
              })}
            </div>
          </details>

          {@render text(person, i, ['notes'], 'Anything else a helper should know', PROFILE_MAX.notes, {
            help: 'For example: calms down with music, needs glasses to read, or speaks Spanish at home.',
            multiline: true,
          })}
        </li>
      {/each}
    </ol>
    <p class="small muted">Someone missing? Add them on <a href={href('who')}>Who is in your household</a>.</p>

    <p class="visually-hidden" aria-live="polite">{status}</p>
    <InterviewNav step="people" />
  {/if}
</div>

<style>
  .gate {
    max-width: var(--w-text);
  }
  .people {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--s4);
    display: grid;
    gap: var(--s5);
  }
  .people > li + li {
    margin-top: 0;
  }
  .person__title {
    margin: 0 0 var(--s4);
    overflow-wrap: anywhere;
  }
  /* Two answers side by side when there is room; the grid gap spaces them either way. */
  .pair {
    display: grid;
    gap: var(--s3) var(--s4);
    /* Inputs line up even when one answer has help text and its neighbour has none. */
    align-items: end;
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
  .group {
    margin-top: var(--s2);
    padding-top: var(--s2);
    border-top: 1px solid var(--border);
  }
  .group:last-of-type {
    margin-bottom: var(--s4);
  }
  .group__body {
    padding-top: var(--s3);
  }
  .medicines,
  .insurance {
    margin-bottom: var(--s5);
  }
  /* A group's title reads as a small heading above the answers it groups. */
  .medicines > legend,
  .insurance > legend {
    font-size: var(--text-lg);
  }
</style>
