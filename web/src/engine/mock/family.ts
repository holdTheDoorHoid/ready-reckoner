/**
 * The mock packet's family plan, wallet cards and access-needs line: what the household wrote in
 * the interview's optional steps, echoed word for word (tidied as the engine tidies it), with
 * blanks to fill in where it wrote nothing. Placeholder layout in the shape packet v2 prints
 * (DESIGN-DELTA §3, rr-plan): the family plan right after the summary, then one wallet card per
 * person as a block quote, which the app prints as a card to cut out.
 *
 * Contract v3 (DESIGN-DELTA-v3 §2): each person's profile, the home, the neighbourhood, pets,
 * vehicles and documents are echoed after the v2 plan, only when the household gave any, so a v2
 * household's mock packet is unchanged; a wallet card carries the person's name and medical notes.
 * The binder workstreams replace this with the real binder's pages.
 */
import type { Contact, FamilyPlan, Person, PlanInput } from '../types';
import type { PersonProfile } from '../v3-shim';
import { hasAnimals, hasChildren, hasVehicle, tidyFamilyPlan } from '../../lib/family';
import { ACCESS_NEED, HOLD } from '../../lib/labels';
import { AGE_WORD, PLACE_KIND, tidyProfile } from '../../lib/profile';

/** A write-in line on paper. */
const BLANK = '__________';

/** Household text made safe for a Markdown table cell or list item: one line, no formatting. */
function md(text: string): string {
  return text
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/\\/g, '\\\\')
    .replace(/\|/g, '\\|')
    .replace(/([*_`[\]<>#])/g, '\\$1');
}

function contactText(c: Contact | undefined): string | undefined {
  if (!c) return undefined;
  const parts = [c.name, c.phone, c.address].filter((x): x is string => !!x).map(md);
  return parts.length ? parts.join(', ') : undefined;
}

/** "Label: answer; Label: answer" for the answers given, escaped; undefined when none is. */
function pairs(rows: [string, string | undefined][]): string | undefined {
  const given = rows.filter((r): r is [string, string] => !!r[1]).map(([label, value]) => `${label}: ${value}`);
  return given.length ? given.join('; ') : undefined;
}

const t = (text: string | undefined) => (text ? md(text) : undefined);

/** One person's profile on one line (contract v3 `Person.profile`), or undefined when empty. */
function profileLine(p: PersonProfile): string | undefined {
  const place = p.place;
  return pairs([
    ['Date of birth', t(p.date_of_birth)],
    ['Phone', t(p.phone)],
    ['Email', t(p.email)],
    [
      place ? `Where they spend the day (${PLACE_KIND[place.kind].label.toLowerCase()})` : 'Where they spend the day',
      place ? pairs([['Name', t(place.name)], ['Address', t(place.address)], ['Phone', t(place.phone)], ['Its plan', t(place.plan)], ['Pick-up', t(place.pickup)], ['Safest spot', t(place.safest_spot)]]) : undefined,
    ],
    ['Doctor', contactText(p.doctor)],
    ['Pharmacy', contactText(p.pharmacy)],
    ['Conditions', t(p.conditions)],
    ['Medicines', p.medications?.map((m) => [m.name, m.dose, m.schedule, m.purpose].filter((x): x is string => !!x).map(md).join(', ')).join(' / ')],
    ['Allergies', t(p.allergies)],
    ['Blood type', t(p.blood_type)],
    [
      'Insurance',
      p.insurance ? [p.insurance.carrier, p.insurance.plan_name, p.insurance.member_id, p.insurance.group_number, p.insurance.phone].filter((x): x is string => !!x).map(md).join(', ') : undefined,
    ],
    ['ID', t(p.id_notes)],
    ['Also', t(p.notes)],
  ]);
}

/** The heading a person has in the packet: the name when given, then the age group. */
function personTitle(person: Person, i: number): string {
  const name = tidyProfile(person.profile)?.name;
  return name ? `${md(name)} (person ${i + 1}, ${AGE_WORD[person.age_band]})` : `person ${i + 1} (${AGE_WORD[person.age_band]})`;
}

/** The v3 answers after the v2 plan: people, home, neighbourhood, animals, vehicles, documents. */
function v3Lines(input: PlanInput, plan: FamilyPlan): string[] {
  const out: string[] = [];
  const people = input.people.flatMap((person, i) => {
    const profile = tidyProfile(person.profile);
    const line = profile ? profileLine(profile) : undefined;
    return profile ? [`- **${personTitle(person, i)}.**${line ? ` ${line}.` : ''}`] : [];
  });
  if (people.length) out.push('**Our people.**', '', ...people, '');
  const home = plan.home;
  if (home) {
    const line = pairs([
      ['Address', t(home.address)],
      ['Electric company', contactText(home.electric_utility)],
      ['Gas company', contactText(home.gas_utility)],
      ['Water company', contactText(home.water_utility)],
      ['Insurer', contactText(home.insurer)],
      ['Policy number', t(home.policy_number)],
      ['Landlord or mortgage company', contactText(home.landlord_or_mortgage)],
      ['Kit', t(home.where_kit)],
      ['Documents', t(home.where_documents)],
      ['Cash', t(home.where_cash)],
      ['Spare keys', t(home.where_keys)],
    ]);
    if (line) out.push(`**Our home.** ${line}.`, '');
  }
  const hood = plan.neighbourhood;
  if (hood) {
    const line = pairs([
      ['Hospital', contactText(hood.hospital)],
      ['Urgent care', contactText(hood.urgent_care)],
      ['Pharmacy', contactText(hood.pharmacy)],
      ['Shelter', contactText(hood.shelter)],
      ['County emergency office', contactText(hood.county_emergency_office)],
      ['Alerts', t(hood.alerts)],
    ]);
    if (line) out.push(`**Our neighbourhood.** ${line}.`, '');
  }
  const pets = (plan.pets ?? []).map(
    (a) =>
      `- ${pairs([['Name', t(a.name)], ['Kind', t(a.kind)], ['Looks like', t(a.description)], ['Medicines', t(a.medications)], ['Vet', contactText(a.vet)], ['Microchip or tag', t(a.microchip)], ['Records', t(a.records_where)]])}`,
  );
  if (pets.length) out.push('**Our animals.**', '', ...pets, '');
  const vehicles = (plan.vehicles ?? []).map(
    (v) => `- ${pairs([['Vehicle', t(v.description)], ['Plate', t(v.plate)], ['Insurer', contactText(v.insurer)], ['Policy number', t(v.policy_number)], ['In the car', t(v.kept_in_car)]])}`,
  );
  if (vehicles.length) out.push('**Our vehicles.**', '', ...vehicles, '');
  const docs = plan.documents;
  if (docs) {
    const line = pairs([
      ['Accounts', docs.accounts?.map((a) => [a.institution, a.kind, a.phone, a.last4 ? `ending ${a.last4}` : undefined].filter((x): x is string => !!x).map(md).join(', ')).join(' / ')],
      ['Policies', docs.policies?.map((p) => [p.insurer, p.kind, p.policy_number, p.phone].filter((x): x is string => !!x).map(md).join(', ')).join(' / ')],
      ['Originals', t(docs.where_originals)],
      ['Copies', t(docs.where_copies)],
      ['Digital backup', t(docs.digital_backup)],
    ]);
    if (line) out.push(`**Documents and money.** ${line}.`, '');
  }
  return out;
}

/** A wallet card's medical line: allergies, conditions and blood type when the profile has them. */
function medicalNotes(person: Person): string | undefined {
  const p = tidyProfile(person.profile);
  return p ? pairs([['Allergies', t(p.allergies)], ['Conditions', t(p.conditions)], ['Blood type', t(p.blood_type)]]) : undefined;
}

function circleLine(plan: FamilyPlan): string | undefined {
  const people = (plan.trusted_circle ?? []).map((p) => [p.name, p.phone].filter((x): x is string => !!x).map(md).join(' ')).filter(Boolean);
  return people.length ? people.join('; ') : undefined;
}

function numbersLine(plan: FamilyPlan): string | undefined {
  const numbers = (plan.numbers_by_heart ?? []).map(md);
  return numbers.length ? numbers.join('; ') : undefined;
}

/** The "Your family plan" and "Wallet cards" sections, as Markdown lines. */
export function familyPlanSections(input: PlanInput, hazardLines: string[]): string[] {
  const plan: FamilyPlan = tidyFamilyPlan(input.family_plan) ?? {};
  const out: string[] = [];
  const cell = (v: string | undefined) => (v ? md(v) : '');

  out.push('## Your family plan', '');
  out.push(
    tidyFamilyPlan(input.family_plan)
      ? 'This is the plan you wrote. Fill in anything still blank together, then keep a copy in each go-bag and one on the fridge.'
      : 'Fill this in together, or on the "Your family plan" screen. Keep a copy in each go-bag and one on the fridge.',
    '',
  );
  out.push('| Plan | Your answer |', '| --- | --- |');
  const rows: [string, string][] = [
    ['Meeting place near home', cell(plan.meeting_place_near)],
    ['Meeting place outside the neighbourhood', cell(plan.meeting_place_far)],
    ['Out-of-area contact (name and phone)', contactText(plan.out_of_area_contact) ?? ''],
  ];
  if (hasChildren(input) || plan.school_pickup) rows.push(['Who picks up the children, and from where', cell(plan.school_pickup)]);
  rows.push(
    ['Work and school plans', cell(plan.work_plans)],
    ['Shelter spot at home', cell(plan.shelter_spot_home)],
    ['Shelter spot at work or school', cell(plan.shelter_spot_work)],
    ['Where we would go if we had to leave', cell(plan.where_we_would_go)],
    ['Route out, first choice', cell(plan.routes?.[0])],
    ['Route out, second choice', cell(plan.routes?.[1])],
    ['Neighbours who check on us', cell(plan.neighbours_who_check)],
  );
  if (hasAnimals(input) || plan.who_takes_animals) rows.push(['Who takes the animals if we cannot', cell(plan.who_takes_animals)]);
  rows.push(
    ['Gas shut-off', cell(plan.shutoff_gas)],
    ['Water shut-off', cell(plan.shutoff_water)],
    ['Electrical panel', cell(plan.shutoff_electric)],
  );
  if (hasVehicle(input) || plan.roadside_assistance) rows.push(['Roadside assistance', cell(plan.roadside_assistance)]);
  rows.push(['Lawyer (name and phone)', contactText(plan.lawyer) ?? '']);
  for (const [label, value] of rows) out.push(`| ${label} | ${value} |`);
  out.push('');

  out.push('**Our trusted circle.** People who have agreed to help, and what they hold for us.', '');
  out.push('| Name | Phone | Holds for us |', '| --- | --- | --- |');
  const circle = plan.trusted_circle ?? [];
  for (const p of circle) {
    out.push(`| ${cell(p.name)} | ${cell(p.phone)} | ${(p.holds ?? []).map((h) => HOLD[h].short).join(', ')} |`);
  }
  for (let i = circle.length; i < 2; i++) out.push('| | | |');
  out.push('');
  out.push(`**Numbers we know by heart:** ${numbersLine(plan) ?? BLANK}`, '');
  out.push(...v3Lines(input, plan));
  out.push(...hazardLines);

  out.push('## Wallet cards', '');
  out.push('Cut out one card for each person and keep it in a wallet or phone case.', '');
  input.people.forEach((person, i) => {
    out.push(`> **Wallet card: ${personTitle(person, i)}**`, '>');
    out.push(`> - Out-of-area contact: ${contactText(plan.out_of_area_contact) ?? BLANK}`);
    out.push(`> - Meet near home: ${plan.meeting_place_near ? md(plan.meeting_place_near) : BLANK}`);
    out.push(`> - Meet outside the neighbourhood: ${plan.meeting_place_far ? md(plan.meeting_place_far) : BLANK}`);
    out.push(`> - Lawyer: ${contactText(plan.lawyer) ?? BLANK}`);
    out.push(`> - Trusted circle: ${circleLine(plan) ?? BLANK}`);
    out.push(`> - Know by heart: ${numbersLine(plan) ?? BLANK}`);
    out.push(`> - Medical notes: ${medicalNotes(person) ?? BLANK}`);
    out.push('');
  });
  return out;
}

/** One line for the special-needs section when anyone has an access need; undefined otherwise. */
export function accessNeedsLine(input: PlanInput): string | undefined {
  const who = input.people.flatMap((p, i) => {
    const needs = p.access_needs ?? [];
    return needs.length ? [`person ${i + 1} ${needs.map((n) => ACCESS_NEED[n].short).join(' and ')}`] : [];
  });
  if (!who.length) return undefined;
  return `**Help in an emergency:** ${who.join('; ')}. Plan how each person gets warnings and help leaving, and ask your county whether it keeps a registry of people who may need help.`;
}
