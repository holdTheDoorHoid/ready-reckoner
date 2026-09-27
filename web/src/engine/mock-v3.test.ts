/**
 * The stand-in engine with contract v3's inputs (DESIGN-DELTA-v3 §3.1–3.2): it accepts every new
 * answer, checks only its types, tidies it as the engine does, echoes it word for word, and its
 * defaults carry none of it.
 */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from './fixtures';
import { createMockEngine, tidyInput } from './mock';
import type { PlanInput, Problem } from './types';
import { prepareMarkdownOf } from './v3-shim';
import { clone } from '../test/helpers';

/** Philadelphia with every v3 answer filled in, some with stray spaces and over-long lists. */
function withEverything(): PlanInput {
  const input = clone(FIXTURES['philadelphia-renters-4']);
  input.people[0]!.profile = {
    name: ' Ana Sample ',
    date_of_birth: '3 March 1984',
    phone: '555-0101',
    email: 'ana@example.org',
    place: { kind: 'work', name: 'Sample Logistics', address: '400 Example Ave', phone: '555-0110', plan: 'Stay inside', pickup: 'n/a', safest_spot: 'Stairwell B' },
    doctor: { name: 'Dr Example', phone: '555-0120', address: '5 Clinic Rd' },
    pharmacy: { name: 'Corner Pharmacy' },
    conditions: 'High blood pressure',
    medications: [{}, { name: 'Blood pressure tablet', dose: '10 mg', schedule: 'Mornings', purpose: 'Blood pressure' }],
    allergies: 'Penicillin',
    blood_type: 'A+',
    insurance: { carrier: 'Sample Health', plan_name: 'Silver', member_id: 'XJ-000000', group_number: 'G-0000', phone: '555-0130' },
    id_notes: 'Passport in the pouch',
    notes: 'Wears glasses to read',
  };
  input.family_plan = {
    home: {
      address: '12 Sample Street',
      electric_utility: { name: 'Sample Power', phone: '555-0190' },
      gas_utility: { name: 'Sample Gas' },
      water_utility: { phone: '555-0191' },
      insurer: { name: 'Sample Renters Insurance' },
      policy_number: 'R-000000',
      landlord_or_mortgage: { name: 'Example Property Co.' },
      where_kit: 'Hall closet',
      where_documents: 'Go-bag pouch',
      where_cash: 'Go-bag envelope',
      where_keys: 'With Rosa',
    },
    neighbourhood: {
      hospital: { name: 'Sample General Hospital', address: '1 Health Way', phone: '555-0160' },
      urgent_care: { name: 'Sample Urgent Care' },
      pharmacy: { name: 'Corner Pharmacy' },
      shelter: { name: 'Sample High School' },
      county_emergency_office: { phone: '555-0161' },
      alerts: 'County text alerts',
    },
    pets: Array.from({ length: 9 }, (_, i) => ({ name: `Pet ${i + 1}`, microchip: `98500000000000${i}` })),
    vehicles: [{ description: 'Blue 2016 hatchback', plate: 'SMP-0000', insurer: { name: 'Sample Auto' }, policy_number: 'A-000000', kept_in_car: 'Blanket' }],
    documents: {
      accounts: [{ institution: 'First Sample Bank', kind: 'Checking', phone: '555-0196', last4: '4000 1234 5678 0042' }],
      policies: [{ insurer: 'Sample Life', kind: 'Life', policy_number: 'L-000000', phone: '555-0198' }],
      where_originals: 'Fire box',
      where_copies: 'With Rosa',
      digital_backup: 'USB stick in the fire box',
    },
  };
  return input;
}

describe('the stand-in engine and contract v3’s answers', () => {
  it('accepts every new answer and works out the same plan as without them', async () => {
    const engine = createMockEngine();
    const plain = await engine.assess(clone(FIXTURES['philadelphia-renters-4']));
    const full = await engine.assess(withEverything());
    expect(plain.ok && full.ok).toBe(true);
    if (!plain.ok || !full.ok) return;
    // Echo-only: nothing the engine works out changes.
    expect(full.value.buckets).toEqual(plain.value.buckets);
    expect(full.value.plan).toEqual(plain.value.plan);
    expect(full.value.register).toEqual(plain.value.register);
  });

  it('echoes the answers word for word, tidied as the engine tidies them', async () => {
    const out = await createMockEngine().assess(withEverything());
    expect(out.ok).toBe(true);
    if (!out.ok) return;
    const md = prepareMarkdownOf(out.value);
    for (const said of [
      '**Ana Sample (person 1, adult).**',
      'Where they spend the day (work): Name: Sample Logistics',
      'Doctor: Dr Example, 555-0120, 5 Clinic Rd',
      'Medicines: Blood pressure tablet, 10 mg, Mornings, Blood pressure',
      'Insurance: Sample Health, Silver, XJ-000000, G-0000, 555-0130',
      '**Our home.** Address: 12 Sample Street',
      'Electric company: Sample Power, 555-0190',
      '**Our neighbourhood.** Hospital: Sample General Hospital, 555-0160, 1 Health Way',
      'Alerts: County text alerts',
      'Vehicle: Blue 2016 hatchback; Plate: SMP-0000',
      'Accounts: First Sample Bank, Checking, 555-0196, ending 0042',
      'Policies: Sample Life, Life, L-000000, 555-0198',
      '> **Wallet card: Ana Sample (person 1, adult)**',
      '> - Medical notes: Allergies: Penicillin; Conditions: High blood pressure; Blood type: A+',
    ]) {
      expect(md, said).toContain(said);
    }
    // Eight animals at most, and never a full account number.
    expect(md).toContain('Name: Pet 8');
    expect(md).not.toContain('Pet 9');
    expect(md).not.toContain('4000 1234');
  });

  it('tidies the new answers before anything else, as PlanInput::from_json does', () => {
    const tidied = tidyInput(withEverything());
    expect(tidied.people[0]!.profile?.name).toBe('Ana Sample');
    expect(tidied.people[0]!.profile?.medications).toEqual([{ name: 'Blood pressure tablet', dose: '10 mg', schedule: 'Mornings', purpose: 'Blood pressure' }]);
    expect(tidied.family_plan?.pets).toHaveLength(8);
    expect(tidied.family_plan?.documents?.accounts?.[0]?.last4).toBe('0042');
    // A place with nothing but its kind, and an empty profile, disappear.
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.people[1]!.profile = { place: { kind: 'school' }, email: '  ' };
    expect('profile' in tidyInput(input).people[1]!).toBe(false);
  });

  it('checks only the types: a place needs its kind, and unknown fields are refused', async () => {
    const engine = createMockEngine();
    const problems = async (input: unknown): Promise<string[]> => {
      const r = await engine.assess(input as PlanInput);
      return r.ok ? [] : ((r.error.details as { problems: Problem[] }).problems ?? []).map((p) => `${p.code} ${p.field}`);
    };
    const noKind = withEverything() as unknown as { people: { profile: { place: Record<string, unknown> } }[] };
    delete noKind.people[0]!.profile.place.kind;
    expect(await problems(noKind)).toEqual(['schema people[0].profile.place.kind']);
    const badKind = withEverything() as unknown as { people: { profile: { place: Record<string, unknown> } }[] };
    badKind.people[0]!.profile.place.kind = 'office';
    expect(await problems(badKind)).toEqual(['schema people[0].profile.place.kind']);
    const extra = withEverything() as unknown as { family_plan: { home: Record<string, unknown> } };
    extra.family_plan.home.surprise = 'x';
    expect(await problems(extra)).toEqual(['schema family_plan.home.surprise']);
    const notText = withEverything() as unknown as { family_plan: { documents: { accounts: Record<string, unknown>[] } } };
    notText.family_plan.documents.accounts[0]!.last4 = 42;
    expect(await problems(notText)).toEqual(['schema family_plan.documents.accounts[0].last4']);
  });

  it('starts a new plan with none of them', async () => {
    const d = await createMockEngine().defaults();
    expect(d.ok).toBe(true);
    if (!d.ok) return;
    expect(d.value.people.every((p) => !('profile' in p))).toBe(true);
    expect('family_plan' in d.value).toBe(false);
  });
});
