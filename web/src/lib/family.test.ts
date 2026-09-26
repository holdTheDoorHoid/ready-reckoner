import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import type { FamilyPlan, PlanInput } from '../engine/types';
import { FAMILY_PLAN_SHORT_MAX, FAMILY_PLAN_TEXT_MAX, NUMBERS_BY_HEART_MAX, TRUSTED_CIRCLE_MAX } from '../engine/types';
import detroitJson from '../../../fixtures/households/pending/detroit-snap-3.json';
import { clone } from '../test/helpers';
import {
  addNumber,
  addTrustedPerson,
  familySectionFor,
  hasFamilyPlan,
  removeNumber,
  removeTrustedPerson,
  setContactPart,
  setHolds,
  setNote,
  setRoute,
  tidyContactPart,
  tidyFamilyPlan,
  tidyNote,
  tidyRoute,
  tidyText,
} from './family';

const detroit = detroitJson as unknown as PlanInput;

describe('tidying the family plan as the engine does (rr-types FamilyPlan::tidy)', () => {
  it('trims, caps, drops blanks and empty entries, and never requires anything', () => {
    const long = 'x'.repeat(FAMILY_PLAN_TEXT_MAX + 50);
    const plan: FamilyPlan = {
      meeting_place_near: '  the corner mailbox \n',
      meeting_place_far: '   ',
      out_of_area_contact: { name: ' ' },
      where_we_would_go: long,
      routes: [' north ', '', 'west', 'south'],
      trusted_circle: [{}, { name: ' Rosa ', phone: '555-0100', holds: ['spare_key'] }, { holds: ['documents'] }, { name: 'B' }, { name: 'C' }, { name: 'D' }],
      lawyer: { name: 'J. Ortiz', phone: ` ${'5'.repeat(100)} ` },
      numbers_by_heart: Array.from({ length: 8 }, (_, i) => `555-010${i}`),
    };
    const out = tidyFamilyPlan(plan)!;
    expect(out.meeting_place_near).toBe('the corner mailbox');
    expect(out.meeting_place_far).toBeUndefined();
    expect(out.out_of_area_contact).toBeUndefined();
    expect(Array.from(out.where_we_would_go!).length).toBe(FAMILY_PLAN_TEXT_MAX);
    expect(out.routes).toEqual(['north', 'west']);
    // The empty person is dropped; the four kept are the first four with something in them.
    expect(out.trusted_circle).toHaveLength(TRUSTED_CIRCLE_MAX);
    expect(out.trusted_circle![0]!.name).toBe('Rosa');
    expect(out.trusted_circle![1]!.holds).toEqual(['documents']);
    expect(out.trusted_circle![3]!.name).toBe('C');
    expect(Array.from(out.lawyer!.phone!).length).toBe(FAMILY_PLAN_SHORT_MAX);
    expect(out.numbers_by_heart).toHaveLength(NUMBERS_BY_HEART_MAX);
    // The argument is left as it was.
    expect(plan.meeting_place_near).toBe('  the corner mailbox \n');
  });

  it('cuts on character boundaries, not UTF-16 units', () => {
    const out = tidyFamilyPlan({ school_pickup: 'é'.repeat(FAMILY_PLAN_TEXT_MAX + 1) })!;
    expect(Array.from(out.school_pickup!).length).toBe(FAMILY_PLAN_TEXT_MAX);
    // An emoji is one character to the engine and two UTF-16 units to JavaScript.
    expect(tidyText('🙂'.repeat(5), 3)).toBe('🙂🙂🙂');
    expect(tidyText('  a b  ', 80)).toBe('a b');
    expect(tidyText('abc   def', 4)).toBe('abc');
    expect(tidyText('   ', 80)).toBeUndefined();
  });

  it('makes a plan with nothing in it disappear, and keeps a filled one word for word', () => {
    expect(tidyFamilyPlan({ work_plans: '  ' })).toBeUndefined();
    expect(tidyFamilyPlan({})).toBeUndefined();
    expect(tidyFamilyPlan(undefined)).toBeUndefined();
    expect(hasFamilyPlan({ lawyer: {} })).toBe(false);
    // The Detroit household's plan is already tidy: it comes back exactly, in the engine's order.
    expect(tidyFamilyPlan(detroit.family_plan)).toEqual(detroit.family_plan);
    expect(Object.keys(tidyFamilyPlan(detroit.family_plan)!)).toEqual(Object.keys(detroit.family_plan!));
  });
});

describe('editing the family plan in a saved household', () => {
  const fresh = () => clone(FIXTURES['philadelphia-renters-4']);

  it('saves notes as typed, tidies them on leaving, and leaves no empty plan behind', () => {
    const input = fresh();
    expect(input.family_plan).toBeUndefined();
    setNote(input, 'meeting_place_near', ' The corner ');
    expect(input.family_plan).toEqual({ meeting_place_near: ' The corner ' });
    tidyNote(input, 'meeting_place_near');
    expect(input.family_plan).toEqual({ meeting_place_near: 'The corner' });
    setNote(input, 'meeting_place_near', '');
    expect(input.family_plan).toBeUndefined();
    setNote(input, 'shutoff_gas', '   ');
    tidyNote(input, 'shutoff_gas');
    expect(input.family_plan).toBeUndefined();
  });

  it('keeps contacts, routes, numbers and the circle in place while they are edited', () => {
    const input = fresh();
    setContactPart(input, 'lawyer', 'phone', '555-0199 ');
    tidyContactPart(input, 'lawyer', 'phone');
    expect(input.family_plan?.lawyer).toEqual({ phone: '555-0199' });
    setContactPart(input, 'lawyer', 'phone', '');
    expect(input.family_plan).toBeUndefined();

    setRoute(input, 1, 'The river road');
    expect(input.family_plan?.routes).toEqual(['', 'The river road']);
    setRoute(input, 0, 'I-95 north');
    tidyRoute(input, 0);
    expect(input.family_plan?.routes).toEqual(['I-95 north', 'The river road']);
    setRoute(input, 1, '');
    expect(input.family_plan?.routes).toEqual(['I-95 north']);
    setRoute(input, 5, 'ignored');
    expect(input.family_plan?.routes).toEqual(['I-95 north']);

    for (let i = 0; i < NUMBERS_BY_HEART_MAX; i++) expect(addNumber(input)).toBe(true);
    expect(addNumber(input)).toBe(false);
    expect(input.family_plan?.numbers_by_heart).toHaveLength(NUMBERS_BY_HEART_MAX);
    for (let i = NUMBERS_BY_HEART_MAX - 1; i >= 0; i--) removeNumber(input, i);
    expect(input.family_plan?.numbers_by_heart).toBeUndefined();

    for (let i = 0; i < TRUSTED_CIRCLE_MAX; i++) expect(addTrustedPerson(input)).toBe(true);
    expect(addTrustedPerson(input)).toBe(false);
    setHolds(input, 0, 'medical_poa', true);
    setHolds(input, 0, 'spare_key', true);
    expect(input.family_plan?.trusted_circle?.[0]?.holds).toEqual(['spare_key', 'medical_poa']);
    setHolds(input, 0, 'spare_key', false);
    setHolds(input, 0, 'medical_poa', false);
    expect(input.family_plan?.trusted_circle?.[0]?.holds).toBeUndefined();
    for (let i = TRUSTED_CIRCLE_MAX - 1; i >= 0; i--) removeTrustedPerson(input, i);
    expect(input.family_plan?.trusted_circle).toBeUndefined();
    expect(input.family_plan).toEqual({ routes: ['I-95 north'] });
  });
});

describe('plan steps that the family plan answers', () => {
  it('links the household-plan step to the plan, and the circle, lawyer, leaving and shut-off steps to their parts', () => {
    expect(familySectionFor('comms_contact_card')).toBe('contact');
    expect(familySectionFor('plan_family_contacts')).toBe('contact');
    expect(familySectionFor('community_trusted_circle')).toBe('circle');
    expect(familySectionFor('docs_legal_readiness')).toBe('lawyer');
    expect(familySectionFor('evac_know_zone')).toBe('leave');
    expect(familySectionFor('fire_learn_shutoffs')).toBe('home');
    expect(familySectionFor('water_stored')).toBeUndefined();
  });
});
