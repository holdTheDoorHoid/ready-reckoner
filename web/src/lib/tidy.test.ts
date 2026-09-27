/**
 * The optional steps' limits and the engine's tidy (DESIGN-DELTA-v3 §3.1–3.2), and editing answers
 * by path. The limits are read straight out of the design delta's §3 blocks and compared with the
 * forms' limits, field by field.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { repoRoot } from '../test/real';
import {
  ACCOUNT_MAX,
  ACCOUNTS_MAX,
  addRow,
  CONTACT_MAX,
  DOCUMENTS_MAX,
  HEALTH_INSURANCE_MAX,
  HOME_MAX,
  lastFour,
  MEDICATION_MAX,
  MEDICATIONS_MAX,
  NEIGHBOURHOOD_MAX,
  PET_MAX,
  PETS_MAX,
  PLACE_MAX,
  PLACE_SPEC,
  POLICIES_MAX,
  POLICY_MAX,
  PROFILE_MAX,
  PROFILE_SPEC,
  removeRow,
  rowCount,
  setTextAt,
  textAt,
  tidyText,
  tidyTextAt,
  tidyValue,
  VEHICLE_MAX,
  VEHICLES_MAX,
} from './tidy';

/** Every `field?: string(N)` and `list: [X] (≤ N)` in the delta's §3.1–3.2 blocks, by struct. */
function deltaCaps(): Record<string, Record<string, number>> {
  const doc = readFileSync(join(repoRoot(), 'docs', 'DESIGN-DELTA-v3.md'), 'utf8');
  const section = doc.slice(doc.indexOf('### 3.1'), doc.indexOf('### 3.3'));
  const blocks = [...section.matchAll(/```\n([\s\S]*?)```/g)].map((m) => m[1]!).join('\n');
  const out: Record<string, Record<string, number>> = {};
  // A struct starts with its name and "{" and runs to the matching "}" (the blocks never nest braces).
  for (const m of blocks.matchAll(/^\s*([A-Z]\w+)\s*\{([^}]*)\}/gm)) {
    const fields: Record<string, number> = {};
    for (const f of m[2]!.matchAll(/(\w+)\??:\s*string\((\d+)\)/g)) fields[f[1]!] = Number(f[2]);
    for (const f of m[2]!.matchAll(/(\w+):\s*\[\w+\]\s*\(≤\s*(\d+)\)/g)) fields[`${f[1]!}[]`] = Number(f[2]);
    out[m[1]!] = { ...(out[m[1]!] ?? {}), ...fields };
  }
  return out;
}

describe('the limits are the design delta’s (§3.1–3.2)', () => {
  const caps = deltaCaps();

  it('found every struct in the delta', () => {
    for (const s of ['PersonProfile', 'Place', 'Medication', 'Insurance', 'Contact', 'HomeInfo', 'Neighbourhood', 'PetInfo', 'VehicleInfo', 'DocumentsInfo', 'AccountInfo', 'PolicyInfo']) {
      expect(caps[s], s).toBeDefined();
    }
  });

  it('person, place, medicine and health insurance', () => {
    const { 'medications[]': medications, ...profile } = caps.PersonProfile!;
    expect(PROFILE_MAX).toEqual(profile);
    expect(MEDICATIONS_MAX).toBe(medications);
    expect(PLACE_MAX).toEqual(caps.Place);
    expect(MEDICATION_MAX).toEqual(caps.Medication);
    expect(HEALTH_INSURANCE_MAX).toEqual(caps.Insurance);
  });

  it('contacts keep v2’s 80 for name and phone, and add the address', () => {
    expect(CONTACT_MAX).toEqual({ name: 80, phone: 80, ...caps.Contact });
  });

  it('home, neighbourhood, pets, vehicles, documents, accounts and policies', () => {
    expect(HOME_MAX).toEqual(caps.HomeInfo);
    expect(NEIGHBOURHOOD_MAX).toEqual(caps.Neighbourhood);
    expect(PET_MAX).toEqual(caps.PetInfo);
    expect(VEHICLE_MAX).toEqual(caps.VehicleInfo);
    const { 'accounts[]': accounts, 'policies[]': policies, ...documents } = caps.DocumentsInfo!;
    expect(DOCUMENTS_MAX).toEqual(documents);
    expect([ACCOUNTS_MAX, POLICIES_MAX]).toEqual([accounts, policies]);
    const { last4, ...account } = caps.AccountInfo!;
    expect(ACCOUNT_MAX).toEqual(account);
    expect(last4).toBe(4);
    expect(POLICY_MAX).toEqual(caps.PolicyInfo);
  });

  it('the group limits: 12 medicines, 8 animals, 4 vehicles, 12 accounts, 8 policies', () => {
    const plan = deltaCaps().FamilyPlan!;
    expect([MEDICATIONS_MAX, PETS_MAX, VEHICLES_MAX, ACCOUNTS_MAX, POLICIES_MAX]).toEqual([12, plan['pets[]'], plan['vehicles[]'], 12, 8]);
  });
});

describe('tidying as the engine tidies', () => {
  it('trims, cuts at the limit counted in characters, and trims again', () => {
    expect(tidyText('  Ana  ', 60)).toBe('Ana');
    expect(tidyText('é'.repeat(70), 60)).toBe('é'.repeat(60));
    expect(tidyText('👩‍👧'.repeat(3), 4)).toBe(Array.from('👩‍👧'.repeat(3)).slice(0, 4).join(''));
    expect(tidyText(`${'x'.repeat(59)} y`, 60)).toBe('x'.repeat(59));
    expect(tidyText('   ', 60)).toBeUndefined();
    expect(tidyText(undefined, 60)).toBeUndefined();
  });

  it('keeps only the last four digits of an account number, whatever was typed or pasted', () => {
    expect(lastFour('1234')).toBe('1234');
    expect(lastFour('4000 1234 5678 9010')).toBe('9010');
    expect(lastFour('ending in 42')).toBe('42');
    expect(lastFour('no digits')).toBeUndefined();
    expect(lastFour('')).toBeUndefined();
  });

  it('drops empty rows and groups, cuts lists at their limit, and keeps the engine’s field order', () => {
    const tidied = tidyValue(
      {
        notes: '  Needs glasses  ',
        name: ' Ana ',
        medications: [{}, { name: ' ' }, ...Array.from({ length: 14 }, (_, i) => ({ name: `Medicine ${i + 1}` }))],
        doctor: { name: '', phone: '  ' },
        insurance: { member_id: ' XJ-1 ' },
        surprise: 'dropped',
      },
      PROFILE_SPEC,
    ) as Record<string, unknown>;
    expect(Object.keys(tidied)).toEqual(['name', 'medications', 'insurance', 'notes']);
    expect(tidied.name).toBe('Ana');
    expect(tidied.medications).toHaveLength(12);
    expect((tidied.medications as { name: string }[])[0]!.name).toBe('Medicine 1');
    expect(tidied.insurance).toEqual({ member_id: 'XJ-1' });
    expect(tidyValue({ doctor: {}, medications: [] }, PROFILE_SPEC)).toBeUndefined();
  });

  it('drops a place that has only its kind, and a kind that is not one of the choices', () => {
    expect(tidyValue({ kind: 'school' }, PLACE_SPEC)).toBeUndefined();
    expect(tidyValue({ kind: 'school', name: ' Sample Elementary ' }, PLACE_SPEC)).toEqual({ kind: 'school', name: 'Sample Elementary' });
    expect(tidyValue({ kind: 'moon', name: 'Base' }, PLACE_SPEC)).toEqual({ name: 'Base' });
  });
});

describe('editing by path', () => {
  it('creates the groups on the way, and removes any group left empty', () => {
    const person: Record<string, unknown> = { age_band: 'adult' };
    setTextAt(person, ['profile', 'doctor', 'name'], 'Dr Sample');
    expect(person.profile).toEqual({ doctor: { name: 'Dr Sample' } });
    setTextAt(person, ['profile', 'doctor', 'phone'], '555-0100');
    setTextAt(person, ['profile', 'doctor', 'name'], '');
    expect(person.profile).toEqual({ doctor: { phone: '555-0100' } });
    setTextAt(person, ['profile', 'doctor', 'phone'], '');
    expect('profile' in person).toBe(false);
    expect(person).toEqual({ age_band: 'adult' });
  });

  it('keeps what is typed until the field is left, then tidies it', () => {
    const root: Record<string, unknown> = {};
    setTextAt(root, ['home', 'address'], '  12 Sample St ');
    expect(textAt(root, ['home', 'address'])).toBe('  12 Sample St ');
    tidyTextAt(root, ['home', 'address'], 200);
    expect(textAt(root, ['home', 'address'])).toBe('12 Sample St');
    setTextAt(root, ['home', 'address'], '   ');
    tidyTextAt(root, ['home', 'address'], 200);
    expect(root).toEqual({});
  });

  it('adds rows up to the limit, keeps each row in place while it is edited, and removes rows', () => {
    const root: Record<string, unknown> = {};
    for (let i = 0; i < 4; i++) expect(addRow(root, ['vehicles'], 4)).toBe(true);
    expect(addRow(root, ['vehicles'], 4)).toBe(false);
    expect(rowCount(root, ['vehicles'])).toBe(4);
    setTextAt(root, ['vehicles', 1, 'plate'], 'ABC-123');
    setTextAt(root, ['vehicles', 1, 'plate'], '');
    // The emptied row keeps its place; the engine drops empty rows itself.
    expect(root.vehicles).toEqual([{}, {}, {}, {}]);
    setTextAt(root, ['vehicles', 2, 'description'], 'Blue hatchback');
    removeRow(root, ['vehicles'], 0);
    expect(root.vehicles).toEqual([{}, { description: 'Blue hatchback' }, {}]);
    for (let i = 2; i >= 0; i--) removeRow(root, ['vehicles'], i);
    expect(root).toEqual({});
    removeRow(root, ['vehicles'], 5);
    expect(root).toEqual({});
  });
});
