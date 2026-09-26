import { describe, expect, it } from 'vitest';

import { FIXTURE_NAMES, FIXTURES } from './fixtures';
import { ENGINE_API_VERSION, RETURN_PERIODS } from './types';

// Every household file on disk, found by Vite at build time (keys are relative paths).
const onDisk = Object.keys(import.meta.glob('../../../fixtures/households/*.json'))
  .map((path) => path.replace(/^.*\//, '').replace(/\.json$/, ''))
  .sort();

describe('fixtures', () => {
  it('imports every household file in fixtures/households', () => {
    expect(onDisk.length).toBeGreaterThan(0);
    expect([...FIXTURE_NAMES].sort()).toEqual(onDisk);
  });

  it('parses into households with at least one person and a return-period dial', () => {
    for (const name of FIXTURE_NAMES) {
      const plan = FIXTURES[name];
      expect(plan.people.length).toBeGreaterThan(0);
      expect(RETURN_PERIODS).toContain(plan.dials.return_period);
    }
    expect(FIXTURES['coos-bay-well-owner-2'].dials.return_period).toBe('one_in_500');
    expect(FIXTURES['sugar-land-ev-household-3'].dials.return_period).toBe('one_in_50');
    expect(FIXTURES['hays-kansas-farm-5'].location.county_fips).toBe('20051');
  });

  it('exports the contract version', () => {
    expect(ENGINE_API_VERSION).toBe(2);
  });
});
