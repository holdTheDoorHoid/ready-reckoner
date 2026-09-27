/** Step 6's answers on each person (DESIGN-DELTA-v3 §2.1, contract v3 `Person.profile`). */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import type { Person } from '../engine/types';
import { clone } from '../test/helpers';
import {
  addMedication,
  defaultPlaceKind,
  hasProfile,
  personHeading,
  personRef,
  removeMedication,
  setPlaceKind,
  setProfileText,
  tidyPeopleProfiles,
  tidyProfile,
  tidyProfileText,
} from './profile';
import { MEDICATIONS_MAX, PROFILE_MAX } from './tidy';

const adult = (): Person => clone(FIXTURES['philadelphia-renters-4'].people[0]!);

describe('a person card', () => {
  it('is headed "Person 1 (adult)" until a name is given, then by the name', () => {
    const p = adult();
    expect(personHeading(p, 0)).toBe('Person 1 (adult)');
    expect(personHeading({ ...p, age_band: 'senior' }, 2)).toBe('Person 3 (older adult)');
    expect(personHeading({ ...p, age_band: 'teen' }, 1)).toBe('Person 2 (teenager)');
    setProfileText(p, ['name'], '  Ana  ');
    expect(personHeading(p, 0)).toBe('Ana');
    expect(personRef(p, 0)).toBe('Ana');
    setProfileText(p, ['name'], '   ');
    expect(personHeading(p, 0)).toBe('Person 1 (adult)');
    expect(personRef(p, 0)).toBe('person 1');
  });

  it('gives a new place the likeliest kind for the age, which the person can change', () => {
    expect(defaultPlaceKind('infant')).toBe('childcare');
    expect(defaultPlaceKind('toddler')).toBe('childcare');
    expect(defaultPlaceKind('child')).toBe('school');
    expect(defaultPlaceKind('teen')).toBe('school');
    expect(defaultPlaceKind('adult')).toBe('work');
    expect(defaultPlaceKind('senior')).toBe('other');
    const p: Person = { ...adult(), age_band: 'child' };
    setProfileText(p, ['place', 'name'], 'Sample Elementary');
    expect(p.profile?.place).toEqual({ kind: 'school', name: 'Sample Elementary' });
    setPlaceKind(p, 'childcare');
    expect(p.profile?.place).toEqual({ kind: 'childcare', name: 'Sample Elementary' });
    // A kind chosen first stays when text follows.
    const q = adult();
    setPlaceKind(q, 'other');
    setProfileText(q, ['place', 'phone'], '555-0110');
    expect(q.profile?.place).toEqual({ kind: 'other', phone: '555-0110' });
  });

  it('saves answers as typed, tidies them when left, and keeps no empty profile', () => {
    const p = adult();
    setProfileText(p, ['allergies'], '  Penicillin  ');
    expect(p.profile).toEqual({ allergies: '  Penicillin  ' });
    tidyProfileText(p, ['allergies'], PROFILE_MAX.allergies);
    expect(p.profile).toEqual({ allergies: 'Penicillin' });
    setProfileText(p, ['insurance', 'member_id'], 'XJ-000');
    setProfileText(p, ['allergies'], '');
    setProfileText(p, ['insurance', 'member_id'], '');
    expect(p.profile).toBeUndefined();
    expect('profile' in p).toBe(false);
    expect(hasProfile(p)).toBe(false);
  });

  it('keeps up to 12 medicines, each row in its place, and removes them', () => {
    const p = adult();
    for (let i = 0; i < MEDICATIONS_MAX; i++) expect(addMedication(p)).toBe(true);
    expect(addMedication(p)).toBe(false);
    expect(p.profile?.medications).toHaveLength(12);
    setProfileText(p, ['medications', 3, 'name'], 'Inhaler');
    expect(hasProfile(p)).toBe(true);
    expect(tidyProfile(p.profile)).toEqual({ medications: [{ name: 'Inhaler' }] });
    for (let i = MEDICATIONS_MAX - 1; i >= 0; i--) removeMedication(p, i);
    expect(p.profile).toBeUndefined();
  });
});

describe('the engine’s tidy of every profile (the mock engine)', () => {
  it('tidies each person’s profile, drops empty ones, and leaves the household untouched', () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.people[0]!.profile = { name: ' Ana ', place: { kind: 'work' } };
    input.people[1]!.profile = { medications: [{}], email: ' ' };
    const people = tidyPeopleProfiles(input);
    expect(people[0]!.profile).toEqual({ name: 'Ana' });
    expect('profile' in people[1]!).toBe(false);
    expect(people[2]).toBe(input.people[2]);
    expect(input.people[0]!.profile).toEqual({ name: ' Ana ', place: { kind: 'work' } });
  });
});
