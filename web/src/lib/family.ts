/**
 * The household's own plan (`PlanInput.family_plan`): how its text is tidied, the edits the
 * optional interview steps make to it, which questions a household sees, and which plan steps it
 * answers.
 *
 * The plan is free text the engine only echoes (into the binder and the wallet cards); it is
 * never computed with and never required. `PlanInput::from_json` tidies it before anything else
 * (rr-types `FamilyPlan::tidy`): text trimmed, notes cut at 300 characters, names and numbers at
 * 80, lists at 2 routes, 4 people and 5 numbers; blank text becomes absent, empty entries are
 * dropped, and an empty plan disappears. Contract v3 (DESIGN-DELTA-v3 §3.2) adds the home, the
 * neighbourhood, pets, vehicles and documents, with the limits in `./tidy`, and an address on
 * every contact. `tidyFamilyPlan` does all of that, so the mock engine prints what the real one
 * prints, and the forms tidy each field the same way when it is left.
 *
 * Since v0.3.0 the questions are steps 7 and 8 of the interview (`#/places`, `#/contacts`); the v2
 * family-plan screen and its address `#/family` are gone (the address redirects).
 *
 * It is kept in the saved plan (this browser, and the file "Save a copy" writes) and nowhere else.
 */
import type { Contact, FamilyPlan, Holds, PlanInput, TrustedPerson } from '../engine/types';
import { FAMILY_PLAN_SHORT_MAX, FAMILY_PLAN_TEXT_MAX, HOLDS, NUMBERS_BY_HEART_MAX, ROUTES_MAX, TRUSTED_CIRCLE_MAX } from '../engine/types';
import type { DocumentsInfo, HomeInfo, Neighbourhood, PetInfo, VehicleInfo } from '../engine/v3-shim';
import {
  addRow,
  CONTACT_SPEC,
  DOCUMENTS_SPEC,
  HOME_SPEC,
  NEIGHBOURHOOD_SPEC,
  type Path,
  PET_SPEC,
  PETS_MAX,
  removeRow,
  setTextAt,
  tidyText,
  tidyTextAt,
  tidyValue,
  VEHICLE_SPEC,
  VEHICLES_MAX,
} from './tidy';

export { tidyText } from './tidy';

/** The free-text notes (300 characters each), in the order the engine's struct declares them. */
export const FAMILY_NOTES = [
  'meeting_place_near',
  'meeting_place_far',
  'school_pickup',
  'work_plans',
  'shelter_spot_home',
  'shelter_spot_work',
  'where_we_would_go',
  'neighbours_who_check',
  'who_takes_animals',
  'shutoff_gas',
  'shutoff_water',
  'shutoff_electric',
] as const;
export type FamilyNote = (typeof FAMILY_NOTES)[number];

/** The two contacts: a name and a phone number each. */
export const FAMILY_CONTACTS = ['out_of_area_contact', 'lawyer'] as const;
export type FamilyContact = (typeof FAMILY_CONTACTS)[number];

/**
 * The v2 family-plan screen's parts. Its addresses (`#/family/<section>`) now redirect to the card
 * on step 7 or 8 that holds the same questions (`FAMILY_REDIRECTS` in the router), and the plan
 * steps that answer one of them link there (`FamilyPlanLink`).
 */
export const FAMILY_SECTIONS = ['contact', 'children', 'shelter', 'leave', 'home', 'circle', 'lawyer'] as const;
export type FamilySection = (typeof FAMILY_SECTIONS)[number];

// ---------------------------------------------------------------------------------------------
// Tidying, as the engine tidies (rr-types `FamilyPlan::tidy`)
// ---------------------------------------------------------------------------------------------

/** A contact tidied: name and phone (80 characters each) and, since contract v3, an address (200). */
function tidyContact(c: Contact | undefined): Contact | undefined {
  return tidyValue(c, CONTACT_SPEC) as Contact | undefined;
}

function tidyList(list: readonly string[] | undefined, maxLen: number, maxItems: number): string[] {
  return (list ?? []).flatMap((t) => {
    const kept = tidyText(t, maxLen);
    return kept === undefined ? [] : [kept];
  }).slice(0, maxItems);
}

function tidyPerson(p: TrustedPerson): TrustedPerson | undefined {
  const out: TrustedPerson = {};
  const name = tidyText(p.name, FAMILY_PLAN_SHORT_MAX);
  const phone = tidyText(p.phone, FAMILY_PLAN_SHORT_MAX);
  if (name !== undefined) out.name = name;
  if (phone !== undefined) out.phone = phone;
  if (p.holds?.length) out.holds = [...p.holds];
  return name === undefined && phone === undefined && !out.holds ? undefined : out;
}

/**
 * The family plan as the engine keeps it: a new object with every field tidied, in the engine's
 * field order, or undefined when nothing is filled in. The argument is not changed.
 */
export function tidyFamilyPlan(plan: FamilyPlan | undefined): FamilyPlan | undefined {
  if (!plan) return undefined;
  const out: FamilyPlan = {};
  const note = (key: FamilyNote) => {
    const v = tidyText(plan[key], FAMILY_PLAN_TEXT_MAX);
    if (v !== undefined) out[key] = v;
  };
  note('meeting_place_near');
  note('meeting_place_far');
  const contact = tidyContact(plan.out_of_area_contact);
  if (contact) out.out_of_area_contact = contact;
  note('school_pickup');
  note('work_plans');
  note('shelter_spot_home');
  note('shelter_spot_work');
  note('where_we_would_go');
  const routes = tidyList(plan.routes, FAMILY_PLAN_TEXT_MAX, ROUTES_MAX);
  if (routes.length) out.routes = routes;
  note('neighbours_who_check');
  note('who_takes_animals');
  note('shutoff_gas');
  note('shutoff_water');
  note('shutoff_electric');
  const circle = (plan.trusted_circle ?? []).flatMap((p) => {
    const kept = tidyPerson(p);
    return kept ? [kept] : [];
  }).slice(0, TRUSTED_CIRCLE_MAX);
  if (circle.length) out.trusted_circle = circle;
  const lawyer = tidyContact(plan.lawyer);
  if (lawyer) out.lawyer = lawyer;
  const roadside = tidyText(plan.roadside_assistance, FAMILY_PLAN_SHORT_MAX);
  if (roadside !== undefined) out.roadside_assistance = roadside;
  const numbers = tidyList(plan.numbers_by_heart, FAMILY_PLAN_SHORT_MAX, NUMBERS_BY_HEART_MAX);
  if (numbers.length) out.numbers_by_heart = numbers;
  // Contract v3 (DESIGN-DELTA-v3 §3.2).
  const home = tidyValue(plan.home, HOME_SPEC) as HomeInfo | undefined;
  if (home) out.home = home;
  const neighbourhood = tidyValue(plan.neighbourhood, NEIGHBOURHOOD_SPEC) as Neighbourhood | undefined;
  if (neighbourhood) out.neighbourhood = neighbourhood;
  const pets = tidyRows<PetInfo>(plan.pets, PET_SPEC, PETS_MAX);
  if (pets.length) out.pets = pets;
  const vehicles = tidyRows<VehicleInfo>(plan.vehicles, VEHICLE_SPEC, VEHICLES_MAX);
  if (vehicles.length) out.vehicles = vehicles;
  const documents = tidyValue(plan.documents, DOCUMENTS_SPEC) as DocumentsInfo | undefined;
  if (documents) out.documents = documents;
  return Object.keys(out).length ? out : undefined;
}

/** Rows tidied one by one, empty rows dropped, at most `max` kept. */
function tidyRows<T>(rows: readonly unknown[] | undefined, spec: Parameters<typeof tidyValue>[1], max: number): T[] {
  return (rows ?? []).map((r) => tidyValue(r, spec) as T | undefined).filter((r): r is T => r !== undefined).slice(0, max);
}

/** True when anything in the plan is filled in (after tidying). */
export function hasFamilyPlan(plan: FamilyPlan | undefined): boolean {
  return tidyFamilyPlan(plan) !== undefined;
}

// ---------------------------------------------------------------------------------------------
// Editing the plan in a saved household (the form)
// ---------------------------------------------------------------------------------------------

/**
 * The household's family plan, created empty if it has none yet. Every helper here reads a value
 * back after creating it (never the object it just assigned): in the app the plan is reactive
 * state, and only what is read back through it is watched.
 */
export function ensureFamilyPlan(input: PlanInput): FamilyPlan {
  input.family_plan ??= {};
  return input.family_plan;
}

/** Leaves no empty family plan behind: an untouched household keeps no `family_plan` at all. */
export function dropIfEmpty(input: PlanInput): void {
  const plan = input.family_plan;
  if (!plan) return;
  for (const key of Object.keys(plan) as (keyof FamilyPlan)[]) {
    const v = plan[key];
    if (v === undefined || (Array.isArray(v) && v.length === 0) || (typeof v === 'object' && !Array.isArray(v) && Object.keys(v).length === 0)) {
      delete plan[key];
    }
  }
  if (Object.keys(plan).length === 0) delete input.family_plan;
}

/** Set a note as typed; blank removes it. */
export function setNote(input: PlanInput, key: FamilyNote, text: string): void {
  const plan = ensureFamilyPlan(input);
  if (text === '') delete plan[key];
  else plan[key] = text;
  dropIfEmpty(input);
}

/** Tidy a note when the person leaves the field (trim and cap it, as the engine does). */
export function tidyNote(input: PlanInput, key: FamilyNote): void {
  const plan = input.family_plan;
  if (!plan) return;
  const v = tidyText(plan[key], FAMILY_PLAN_TEXT_MAX);
  if (v === undefined) delete plan[key];
  else plan[key] = v;
  dropIfEmpty(input);
}

/** Set a contact's name or phone as typed; blank removes it. */
export function setContactPart(input: PlanInput, key: FamilyContact, part: 'name' | 'phone', text: string): void {
  const plan = ensureFamilyPlan(input);
  plan[key] ??= {};
  const contact = plan[key]!;
  if (text === '') delete contact[part];
  else contact[part] = text;
  dropIfEmpty(input);
}

export function tidyContactPart(input: PlanInput, key: FamilyContact, part: 'name' | 'phone'): void {
  const contact = input.family_plan?.[key];
  if (!contact) return;
  const v = tidyText(contact[part], FAMILY_PLAN_SHORT_MAX);
  if (v === undefined) delete contact[part];
  else contact[part] = v;
  dropIfEmpty(input);
}

/** The roadside-assistance number as typed (80 characters). */
export function setRoadside(input: PlanInput, text: string): void {
  const plan = ensureFamilyPlan(input);
  if (text === '') delete plan.roadside_assistance;
  else plan.roadside_assistance = text;
  dropIfEmpty(input);
}

export function tidyRoadside(input: PlanInput): void {
  const plan = input.family_plan;
  if (!plan) return;
  const v = tidyText(plan.roadside_assistance, FAMILY_PLAN_SHORT_MAX);
  if (v === undefined) delete plan.roadside_assistance;
  else plan.roadside_assistance = v;
  dropIfEmpty(input);
}

/**
 * Set route 1 or 2 as typed. The routes keep their places while being edited (an empty first
 * route stays as ""), and empty routes at the end are dropped; the engine drops any empty route.
 */
export function setRoute(input: PlanInput, index: number, text: string): void {
  if (index < 0 || index >= ROUTES_MAX) return;
  const plan = ensureFamilyPlan(input);
  const routes = [...(plan.routes ?? [])];
  while (routes.length <= index) routes.push('');
  routes[index] = text;
  while (routes.length && routes[routes.length - 1]!.trim() === '') routes.pop();
  if (routes.length) plan.routes = routes;
  else delete plan.routes;
  dropIfEmpty(input);
}

/** A list entry (a route or a number) trimmed and capped when the person leaves it; the entry keeps its place. */
function tidyEntry(list: string[] | undefined, index: number, max: number): void {
  if (!list || index >= list.length) return;
  list[index] = tidyText(list[index], max) ?? '';
}

export function tidyRoute(input: PlanInput, index: number): void {
  const plan = input.family_plan;
  tidyEntry(plan?.routes, index, FAMILY_PLAN_TEXT_MAX);
  if (plan?.routes) {
    while (plan.routes.length && plan.routes[plan.routes.length - 1] === '') plan.routes.pop();
  }
  dropIfEmpty(input);
}

/** Add an empty number to the list (at most five); false when the list is full. */
export function addNumber(input: PlanInput): boolean {
  const plan = ensureFamilyPlan(input);
  plan.numbers_by_heart ??= [];
  const numbers = plan.numbers_by_heart;
  if (numbers.length >= NUMBERS_BY_HEART_MAX) return false;
  numbers.push('');
  return true;
}

export function setNumber(input: PlanInput, index: number, text: string): void {
  const numbers = input.family_plan?.numbers_by_heart;
  if (!numbers || index >= numbers.length) return;
  numbers[index] = text;
}

export function tidyNumber(input: PlanInput, index: number): void {
  tidyEntry(input.family_plan?.numbers_by_heart, index, FAMILY_PLAN_SHORT_MAX);
}

export function removeNumber(input: PlanInput, index: number): void {
  const numbers = input.family_plan?.numbers_by_heart;
  if (!numbers) return;
  numbers.splice(index, 1);
  dropIfEmpty(input);
}

/** Add an empty person to the trusted circle (at most four); false when the circle is full. */
export function addTrustedPerson(input: PlanInput): boolean {
  const plan = ensureFamilyPlan(input);
  plan.trusted_circle ??= [];
  const circle = plan.trusted_circle;
  if (circle.length >= TRUSTED_CIRCLE_MAX) return false;
  circle.push({});
  return true;
}

export function removeTrustedPerson(input: PlanInput, index: number): void {
  const circle = input.family_plan?.trusted_circle;
  if (!circle) return;
  circle.splice(index, 1);
  dropIfEmpty(input);
}

export function setTrustedPart(input: PlanInput, index: number, part: 'name' | 'phone', text: string): void {
  const person = input.family_plan?.trusted_circle?.[index];
  if (!person) return;
  if (text === '') delete person[part];
  else person[part] = text;
}

export function tidyTrustedPart(input: PlanInput, index: number, part: 'name' | 'phone'): void {
  const person = input.family_plan?.trusted_circle?.[index];
  if (!person) return;
  const v = tidyText(person[part], FAMILY_PLAN_SHORT_MAX);
  if (v === undefined) delete person[part];
  else person[part] = v;
}

/** Tick or untick one thing a person in the circle holds; the list keeps the engine's order. */
export function setHolds(input: PlanInput, index: number, what: Holds, on: boolean): void {
  const person = input.family_plan?.trusted_circle?.[index];
  if (!person) return;
  const held = new Set(person.holds ?? []);
  if (on) held.add(what);
  else held.delete(what);
  const next = HOLDS.filter((h) => held.has(h));
  if (next.length) person.holds = next;
  else delete person.holds;
}

// ---------------------------------------------------------------------------------------------
// Contract v3 groups (steps 7 and 8): the home, the neighbourhood, pets, vehicles and documents.
// Paths start inside `family_plan`, for example `['home', 'electric_utility', 'phone']`.
// ---------------------------------------------------------------------------------------------

const inPlan = (path: Path): Path => ['family_plan', ...path];

/** Set an answer as typed; blank removes it, and any group (or the whole plan) left empty. */
export function setPlanText(input: PlanInput, path: Path, text: string): void {
  setTextAt(input, inPlan(path), text);
}

/** Tidy an answer when the person leaves the field, as the engine will. */
export function tidyPlanText(input: PlanInput, path: Path, max: number): void {
  tidyTextAt(input, inPlan(path), max);
}

/** Add an empty row to a repeating group (a pet, a vehicle, an account, a policy); false when it is full. */
export function addPlanRow(input: PlanInput, path: Path, max: number): boolean {
  return addRow(input, inPlan(path), max);
}

/** Remove one row of a repeating group; an emptied group goes, and an emptied plan. */
export function removePlanRow(input: PlanInput, path: Path, index: number): void {
  removeRow(input, inPlan(path), index);
}

// ---------------------------------------------------------------------------------------------
// Which parts a household sees
// ---------------------------------------------------------------------------------------------

/** The household has children who go to school or child care (or might soon). */
export function hasChildren(input: PlanInput): boolean {
  return input.people.some((p) => p.age_band === 'infant' || p.age_band === 'toddler' || p.age_band === 'child' || p.age_band === 'teen');
}

export function hasAnimals(input: PlanInput): boolean {
  const { dogs, cats, small, large_animals } = input.pets;
  return dogs + cats + small + large_animals > 0;
}

export function hasVehicle(input: PlanInput): boolean {
  return input.mobility.vehicles.length > 0;
}

/** The home has gas: gas or propane heat, or a gas stove. */
export function hasGas(input: PlanInput): boolean {
  return input.housing.heating === 'gas' || input.housing.heating === 'propane' || input.housing.cooking === 'gas';
}

// ---------------------------------------------------------------------------------------------
// Plan steps the family plan answers
// ---------------------------------------------------------------------------------------------

/**
 * Free steps on the plan whose answer is written on this screen, and the section that holds it.
 * Catalogue ids from `content/items` (and the mock catalogue's own ids).
 */
const STEP_SECTIONS: Record<string, FamilySection> = {
  // "Make a household plan and a contact card for each person"
  comms_contact_card: 'contact',
  plan_family_contacts: 'contact',
  // "Your trusted circle: agree who helps whom"
  community_trusted_circle: 'circle',
  // "Legal readiness: a lawyer's number, a will and powers of attorney"
  docs_legal_readiness: 'lawyer',
  // "Plan how you would leave: zone, routes, destination and triggers"
  evac_know_zone: 'leave',
  go_stay_card: 'leave',
  // "Know and prepare your home: shut-offs, ..."
  fire_learn_shutoffs: 'home',
  utility_shutoffs: 'home',
  // The mock catalogue's tornado-spot step.
  safe_room_plan: 'shelter',
};

/** The family-plan section a plan step is written down in, if any. */
export function familySectionFor(itemId: string): FamilySection | undefined {
  return STEP_SECTIONS[itemId];
}

/** The plan steps that mean "make a household plan" (the first link on the plan screen). */
export const HOUSEHOLD_PLAN_STEPS: readonly string[] = ['comms_contact_card', 'plan_family_contacts'];
