/**
 * The seven fixture households (`fixtures/households/*.json`), typed as `PlanInput`, for the mock
 * engine, tests and screenshots. `crates/rr-types` embeds the same files.
 *
 * TypeScript widens the string values of an imported JSON file to `string`, so a JSON import can't
 * simply be assigned to `PlanInput`. `fixture()` checks structure instead: every required field is
 * present with the right primitive type, and no field exists that `PlanInput` does not declare (at
 * any depth). Enum values are checked on the Rust side, which parses these files with unknown
 * fields and unknown ids rejected, and which compares the id lists in `types.ts` with its own.
 *
 * The dev server must be allowed to read the repository root (`server.fs.allow`) if app code
 * imports this module, because the fixtures live outside `web/`.
 */
import type { PlanInput } from './types';

import chicagoStudentZeroBudget1 from '../../../fixtures/households/chicago-student-zero-budget-1.json';
import coosBayWellOwner2 from '../../../fixtures/households/coos-bay-well-owner-2.json';
import haysKansasFarm5 from '../../../fixtures/households/hays-kansas-farm-5.json';
import miamiCondoRetiree1 from '../../../fixtures/households/miami-condo-retiree-1.json';
import philadelphiaRenters4 from '../../../fixtures/households/philadelphia-renters-4.json';
import phoenixApartmentCpap1 from '../../../fixtures/households/phoenix-apartment-cpap-1.json';
import sugarLandEvHousehold3 from '../../../fixtures/households/sugar-land-ev-household-3.json';

/** `T` with every string-literal union widened to `string`, as a JSON import types it. */
type Widened<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends boolean
      ? boolean
      : T extends readonly (infer E)[]
        ? Widened<E>[]
        : T extends object
          ? { [K in keyof T]: Widened<T[K]> }
          : T;

type KeysOfUnion<T> = T extends unknown ? keyof T : never;
type PropOf<T, K extends PropertyKey> = T extends unknown ? (K extends keyof T ? T[K] : never) : never;

/** `F` with every key that `T` does not declare, at any depth, mapped to `never`. */
type NoExtraKeys<F, T> = F extends readonly (infer FE)[]
  ? NoExtraKeys<FE, T extends readonly (infer TE)[] ? TE : never>[]
  : F extends object
    ? { [K in keyof F]: K extends KeysOfUnion<T> ? NoExtraKeys<F[K], NonNullable<PropOf<T, K>>> : never }
    : F;

/** True when a JSON import of type `F` matches `PlanInput` (see the module comment). */
type MatchesPlanInput<F> = [F] extends [Widened<PlanInput>]
  ? [F] extends [NoExtraKeys<F, PlanInput>]
    ? true
    : false
  : false;

/** Accepts a JSON import only if it matches `PlanInput`; otherwise `npm run check` fails here. */
function fixture<F>(json: F & (MatchesPlanInput<F> extends true ? unknown : never)): PlanInput {
  // Structure is checked above and enum values by the Rust tests, so the cast is sound.
  return json as unknown as PlanInput;
}

/** Every fixture household by name (the file stem). */
export const FIXTURES = {
  'chicago-student-zero-budget-1': fixture(chicagoStudentZeroBudget1),
  'coos-bay-well-owner-2': fixture(coosBayWellOwner2),
  'hays-kansas-farm-5': fixture(haysKansasFarm5),
  'miami-condo-retiree-1': fixture(miamiCondoRetiree1),
  'philadelphia-renters-4': fixture(philadelphiaRenters4),
  'phoenix-apartment-cpap-1': fixture(phoenixApartmentCpap1),
  'sugar-land-ev-household-3': fixture(sugarLandEvHousehold3),
} as const satisfies Record<string, PlanInput>;

export type FixtureName = keyof typeof FIXTURES;

/** Fixture names in file order. */
export const FIXTURE_NAMES = Object.keys(FIXTURES) as FixtureName[];

// Self-test of the check above: each line must hold, or `npm run check` fails. If the check ever
// stopped catching drift, the `false` cases would flip and this would stop compiling.
type Expect<T extends true> = T;
type Not<T extends boolean> = T extends true ? false : true;
type Philly = typeof philadelphiaRenters4;
type KansasPerson = (typeof haysKansasFarm5)['people'][number];

export type FixtureCheckSelfTest = [
  // Every real fixture matches, including people with and without a commute.
  Expect<MatchesPlanInput<typeof haysKansasFarm5>>,
  Expect<MatchesPlanInput<typeof chicagoStudentZeroBudget1>>,
  // A field PlanInput does not declare, at the top level or nested, is rejected.
  Expect<Not<MatchesPlanInput<Philly & { surprise: true }>>>,
  Expect<Not<MatchesPlanInput<Omit<Philly, 'location'> & { location: Philly['location'] & { lat: number } }>>>,
  Expect<Not<MatchesPlanInput<Omit<Philly, 'people'> & { people: (KansasPerson & { allergies: string[] })[] }>>>,
  // A missing required field is rejected.
  Expect<Not<MatchesPlanInput<Omit<Philly, 'dials'>>>>,
  // A wrong primitive type is rejected.
  Expect<Not<MatchesPlanInput<Omit<Philly, 'dials'> & { dials: { return_period: string; climate: string; horizon_years: string } }>>>,
];
