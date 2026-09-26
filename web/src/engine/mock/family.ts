/**
 * The mock packet's family plan, wallet cards and access-needs line: what the household wrote on
 * the family-plan screen, echoed word for word (tidied as the engine tidies it), with blanks to
 * fill in where it wrote nothing. Placeholder layout in the shape packet v2 prints (DESIGN-DELTA
 * §3, rr-plan): the family plan right after the summary, then one wallet card per person as a
 * block quote, which the app prints as a card to cut out.
 *
 * awaiting: plan2 (the real packet's family plan and wallet cards)
 */
import type { FamilyPlan, PlanInput } from '../types';
import { hasAnimals, hasChildren, hasVehicle, tidyFamilyPlan } from '../../lib/family';
import { ACCESS_NEED, HOLD } from '../../lib/labels';

/** A person on a card: household members have no names in the plan, only an age group. */
const AGE_WORD: Record<PlanInput['people'][number]['age_band'], string> = {
  infant: 'baby',
  toddler: 'toddler',
  child: 'child',
  teen: 'teenager',
  adult: 'adult',
  senior: 'older adult',
};

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

function contactText(c: FamilyPlan['out_of_area_contact']): string | undefined {
  if (!c) return undefined;
  const parts = [c.name, c.phone].filter((x): x is string => !!x).map(md);
  return parts.length ? parts.join(', ') : undefined;
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
  out.push(...hazardLines);

  out.push('## Wallet cards', '');
  out.push('Cut out one card for each person and keep it in a wallet or phone case.', '');
  input.people.forEach((person, i) => {
    out.push(`> **Wallet card: person ${i + 1} (${AGE_WORD[person.age_band]})**`, '>');
    out.push(`> - Out-of-area contact: ${contactText(plan.out_of_area_contact) ?? BLANK}`);
    out.push(`> - Meet near home: ${plan.meeting_place_near ? md(plan.meeting_place_near) : BLANK}`);
    out.push(`> - Meet outside the neighbourhood: ${plan.meeting_place_far ? md(plan.meeting_place_far) : BLANK}`);
    out.push(`> - Lawyer: ${contactText(plan.lawyer) ?? BLANK}`);
    out.push(`> - Trusted circle: ${circleLine(plan) ?? BLANK}`);
    out.push(`> - Know by heart: ${numbersLine(plan) ?? BLANK}`);
    out.push(`> - Medical notes: ${BLANK}`);
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
