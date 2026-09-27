/**
 * Step 6, Your people (DESIGN-DELTA-v3 §2.1): one card per person from step 2, in the same order,
 * whose answers live on the person itself (`Person.profile`, contract v3 §3.1). A person removed on
 * step 2 takes their answers with them; nothing here is ever required, computed with or checked.
 *
 * The card is headed "Person 1 (adult)" until a name is given, then by the name. A place needs a
 * kind (`Place.kind` is required by the contract), so typing into an empty place first gives it
 * the likeliest kind for the person's age, which the card shows chosen and the person can change.
 */
import type { AgeBand, Person, PersonProfile, PlaceKind, PlanInput } from '../engine/types';
import { addRow, MEDICATIONS_MAX, type Path, PROFILE_SPEC, removeRow, setTextAt, tidyTextAt, tidyValue } from './tidy';

/** A person in words, where the plan has no name for them: "adult", "teenager". */
export const AGE_WORD: Record<AgeBand, string> = {
  infant: 'baby',
  toddler: 'toddler',
  child: 'child',
  teen: 'teenager',
  adult: 'adult',
  senior: 'older adult',
};

/** Where a person spends the day, in words (`Place.kind`). */
export const PLACE_KIND: Record<PlaceKind, { label: string; place: string }> = {
  work: { label: 'Work', place: 'the workplace' },
  school: { label: 'School', place: 'the school' },
  childcare: { label: 'Child care', place: 'the child care' },
  other: { label: 'Somewhere else', place: 'the place' },
};

/** The person's name as typed, trimmed; undefined until one is given. */
export function personName(person: Person): string | undefined {
  const name = person.profile?.name?.trim();
  return name ? name : undefined;
}

/** "Person 1 (adult)" until a name is given, then the name. */
export function personHeading(person: Person, index: number): string {
  return personName(person) ?? `Person ${index + 1} (${AGE_WORD[person.age_band]})`;
}

/** How to refer to the person inside a sentence or a label: the name, or "person 1". */
export function personRef(person: Person, index: number): string {
  return personName(person) ?? `person ${index + 1}`;
}

/** The likeliest kind of place for someone this age: school for children, child care for the youngest. */
export function defaultPlaceKind(age: AgeBand): PlaceKind {
  if (age === 'infant' || age === 'toddler') return 'childcare';
  if (age === 'child' || age === 'teen') return 'school';
  if (age === 'adult') return 'work';
  return 'other';
}

/** The profile as the engine keeps it (trimmed, cut, empties dropped), or undefined when nothing is filled in. */
export function tidyProfile(profile: PersonProfile | undefined): PersonProfile | undefined {
  return tidyValue(profile, PROFILE_SPEC) as PersonProfile | undefined;
}

/** True when anything about the person is filled in (after tidying). */
export function hasProfile(person: Person): boolean {
  return tidyProfile(person.profile) !== undefined;
}

// ---------------------------------------------------------------------------------------------
// Editing (the step 6 form); every path starts at `profile`
// ---------------------------------------------------------------------------------------------

const under = (path: Path): Path => ['profile', ...path];

/** Set an answer as typed; blank removes it, and an emptied profile disappears. */
export function setProfileText(person: Person, path: Path, text: string): void {
  if (path[0] === 'place' && text !== '' && !person.profile?.place) setPlaceKind(person, defaultPlaceKind(person.age_band));
  setTextAt(person, under(path), text);
}

/** Tidy an answer when the person leaves the field, as the engine will. */
export function tidyProfileText(person: Person, path: Path, max: number): void {
  tidyTextAt(person, under(path), max);
}

/** Choose where the person spends the day. */
export function setPlaceKind(person: Person, kind: PlaceKind): void {
  person.profile ??= {};
  const profile = person.profile;
  if (profile.place) profile.place.kind = kind;
  else profile.place = { kind };
}

/** Add an empty medicine row (at most 12); false when the list is full. */
export function addMedication(person: Person): boolean {
  return addRow(person, under(['medications']), MEDICATIONS_MAX);
}

export function removeMedication(person: Person, index: number): void {
  removeRow(person, under(['medications']), index);
}

/** Every person's profile tidied as the engine tidies it (the mock engine's `tidyInput`). */
export function tidyPeopleProfiles(input: PlanInput): PlanInput['people'] {
  return input.people.map((person) => {
    if (person.profile === undefined) return person;
    const profile = tidyProfile(person.profile);
    const out = { ...person };
    if (profile) out.profile = profile;
    else delete out.profile;
    return out;
  });
}
