import { describe, expect, it } from 'vitest';

import type { Dials } from '../engine/types';
import { RARE_HAZARD_IDS } from '../engine/types';
import {
  allowsEveryRareFamily,
  allowsRare,
  longHorizon,
  minimumKit,
  rareFamilies,
  rareSummary,
  setEveryRareFamily,
  setLongHorizon,
  setMinimumKit,
  setRareFamily,
} from './dials';

const base = (): Dials => ({ return_period: 'one_in_100', climate: 'today', horizon_years: 10 });

describe('the rare-family allowance, read as the engine reads it (Dials::rare_families)', () => {
  it('is off by default', () => {
    expect(rareFamilies(base())).toEqual([]);
    expect(allowsRare(base(), 'nuclear_attack')).toBe(false);
    expect(rareSummary(base())).toBe('none');
  });

  it('maps the v1 switch and "all" to every family', () => {
    const v1 = { ...base(), rare_catastrophic_opt_in: true };
    expect(rareFamilies(v1)).toEqual([...RARE_HAZARD_IDS]);
    expect(allowsEveryRareFamily(v1)).toBe(true);
    const all = { ...base(), rare_opt_in: ['all'] };
    expect(rareFamilies(all)).toEqual(rareFamilies(v1));
    expect(rareSummary(all)).toBe('all of them');
  });

  it('picks families in the engine order, collapses repeats and ignores unknown ids', () => {
    const some = { ...base(), rare_opt_in: ['mass_violence', 'nuclear_attack', 'nuclear_attack', 'not_a_family', 'house_fire'] };
    expect(rareFamilies(some)).toEqual(['nuclear_attack', 'mass_violence']);
    expect(allowsRare(some, 'nuclear_attack')).toBe(true);
    expect(allowsRare(some, 'geomagnetic_storm')).toBe(false);
    expect(allowsRare(some, 'house_fire')).toBe(false);
    expect(rareSummary(some)).toBe('2 of 9');
  });
});

describe('changing the allowance', () => {
  it('writes the list form, retires the v1 switch, and drops unknown ids', () => {
    const d = { ...base(), rare_catastrophic_opt_in: true };
    setRareFamily(d, 'nuclear_attack', false);
    expect(d.rare_catastrophic_opt_in).toBe(false);
    expect(d.rare_opt_in).toEqual(RARE_HAZARD_IDS.filter((f) => f !== 'nuclear_attack'));
    const odd = { ...base(), rare_opt_in: ['nuclear_attack', 'zombies'] };
    setRareFamily(odd, 'severe_pandemic', true);
    expect(odd.rare_opt_in).toEqual(['nuclear_attack', 'severe_pandemic']);
    setRareFamily(odd, 'nuclear_attack', false);
    setRareFamily(odd, 'severe_pandemic', false);
    expect(odd.rare_opt_in).toEqual([]);
  });

  it('turns every family on as "all" (which also covers families added later), or none', () => {
    const d = base();
    setEveryRareFamily(d, true);
    expect(d).toMatchObject({ rare_opt_in: ['all'], rare_catastrophic_opt_in: false });
    setRareFamily(d, 'mass_violence', false);
    expect(d.rare_opt_in).toHaveLength(RARE_HAZARD_IDS.length - 1);
    // Ticking the last one back makes every family ticked again: "all".
    setRareFamily(d, 'mass_violence', true);
    expect(d.rare_opt_in).toEqual(['all']);
    setEveryRareFamily(d, false);
    expect(d.rare_opt_in).toEqual([]);
  });
});

describe('bare-minimum mode and the long-horizon section', () => {
  it('default off and switch on and off', () => {
    const d = base();
    expect(minimumKit(d)).toBe(false);
    expect(longHorizon(d)).toBe(false);
    setMinimumKit(d, true);
    setLongHorizon(d, true);
    expect(d).toMatchObject({ minimum_kit: true, long_horizon: true });
    setMinimumKit(d, false);
    expect(minimumKit(d)).toBe(false);
  });
});
