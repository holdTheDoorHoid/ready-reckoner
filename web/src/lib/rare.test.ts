/**
 * The rare-but-severe box's helpers (REVIEW §2.4; hazard-expansion Deliverable C): "why here" in
 * one sentence, "what it changes" with its tick, the family opt-in, what the allowance bought, the
 * "Also checked" line read from the packet notes, and the nuclear chain link. Fixed inputs; the
 * box itself is checked on the screen in screens/displays.test.ts.
 */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import type { Dials, HazardProfile, PlanInput } from '../engine/types';
import { RARE_HAZARD_IDS } from '../engine/types';
import {
  allowance,
  alsoCheckedFor,
  changesNothing,
  firstSentence,
  locationChain,
  optedInFamilies,
  parseAlsoChecked,
  whatItChanges,
  whyHereShort,
  withFamily,
} from './rare';

function rare(extra: Partial<HazardProfile>): HazardProfile {
  return {
    id: 'nuclear_attack',
    name: 'Nuclear attack',
    tier: 'societal',
    display: 'rare_catastrophic',
    rate_per_year: 2.4e-4,
    rate_range: [3e-5, 3.6e-3],
    annual_probability: 2.4e-4,
    probability_range: [3e-5, 3.6e-3],
    severity: 1,
    climate_multiplier: 1,
    confidence: 'prior',
    sources: ['fri_nuclear_risk_2024'],
    frequency_sentence: 'Between 1 in 3,300 and 1 in 28 households like yours would be in a blast zone or under dangerous fallout in the next ten years.',
    buckets: ['supplies'],
    family: 'nuclear_attack',
    range_only: true,
    ...extra,
  };
}

describe('why here and what it changes', () => {
  it('keeps the first sentence of the location factor, never cutting at an abbreviation', () => {
    expect(firstSentence('You live in the Philadelphia metro area. Because of its size, a place like this is a target.')).toBe(
      'You live in the Philadelphia metro area.',
    );
    expect(firstSentence('The U.S. Air Force expects them to be targets. That is why.')).toBe('The U.S. Air Force expects them to be targets.');
    expect(firstSentence('Your county is at a middle geomagnetic latitude (about 40°). Solar storms drive currents.')).toBe(
      'Your county is at a middle geomagnetic latitude (about 40°).',
    );
    expect(firstSentence('One sentence only.')).toBe('One sentence only.');
  });

  it('says a row without a location term is the same everywhere', () => {
    expect(whyHereShort(rare({ location_factor: undefined }))).toMatch(/^The same everywhere/);
    expect(
      whyHereShort(rare({ location_factor: { class: 'C1', label: 'You live in the Chicago metro area. More words.', multiplier: [0.3, 0.6, 0.9], sources: [] } })),
    ).toBe('You live in the Chicago metro area.');
  });

  it('ticks "nothing beyond your basics" and "nothing new", and never an action', () => {
    expect(changesNothing(rare({ what_it_changes: 'Nothing beyond your basics.' }))).toBe(true);
    expect(changesNothing(rare({ what_it_changes: 'Nothing new: the pandemic row already sizes your food and medicine.' }))).toBe(true);
    expect(changesNothing(rare({ what_it_changes: 'One free step: pick your shelter spot at home and at work.' }))).toBe(false);
    expect(changesNothing(rare({ what_it_changes: 'Keep some cash in small bills (already in your plan).' }))).toBe(false);
    // The engine always sends it for a rare row; the box never leaves the cell empty.
    expect(whatItChanges(rare({ what_it_changes: undefined }))).toBe('Nothing beyond your basics.');
  });

  it('turns the nuclear location factor into the chance of being in a blast or fallout zone', () => {
    const c1 = rare({ location_factor: { class: 'C1', label: '', multiplier: [0.3, 0.6, 0.9], sources: [] } });
    expect(locationChain(c1)).toBe(
      'If a large attack on the country happened, the chance that your county would be in a blast or dangerous-fallout zone: about 6 in 10 (3 to 9 in 10).',
    );
    const e = rare({ location_factor: { class: 'E', label: '', multiplier: [0.01, 0.03, 0.1], sources: [] } });
    expect(locationChain(e)).toMatch(/about 3 in 100 \(1 to 10 in 100\)\.$/);
    // Other families' factors are ratios, already said in their "why here" words.
    expect(locationChain(rare({ id: 'geomagnetic_storm', location_factor: { class: 'high', label: '', multiplier: [2, 2, 2], sources: [] } }))).toBeNull();
  });
});

describe('the rare-event allowance', () => {
  it('reads the list, "all" and the old yes/no as the engine does', () => {
    expect(optedInFamilies({})).toEqual([]);
    expect(optedInFamilies({ rare_opt_in: ['all'] })).toEqual([...RARE_HAZARD_IDS]);
    expect(optedInFamilies({ rare_catastrophic_opt_in: true })).toEqual([...RARE_HAZARD_IDS]);
    expect(optedInFamilies({ rare_opt_in: ['mass_violence', 'nuclear_attack', 'nonsense'] })).toEqual(['nuclear_attack', 'mass_violence']);
  });

  it('writes one family at a time, "all" when every family is ticked, and clears the old yes/no', () => {
    expect(withFamily({}, 'nuclear_attack', true)).toEqual({ rare_opt_in: ['nuclear_attack'], rare_catastrophic_opt_in: false });
    expect(withFamily({ rare_catastrophic_opt_in: true }, 'mass_violence', false).rare_opt_in).toEqual(
      RARE_HAZARD_IDS.filter((id) => id !== 'mass_violence'),
    );
    const allButOne = RARE_HAZARD_IDS.filter((id) => id !== 'cbrn_attack');
    expect(withFamily({ rare_opt_in: [...allButOne] }, 'cbrn_attack', true).rare_opt_in).toEqual(['all']);
    expect(withFamily({ rare_opt_in: ['all'] }, 'all', false).rare_opt_in).toEqual([]);
  });

  it('says what the plan buys: the radiation meter for the nuclear row once it is ticked, nothing before', async () => {
    const engine = createMockEngine();
    const cat = await engine.catalogue();
    if (!cat.ok) throw new Error('catalogue');
    const withDials = (dials: Partial<Dials>): PlanInput => {
      const input = JSON.parse(JSON.stringify(FIXTURES['philadelphia-renters-4'])) as PlanInput;
      input.finances.monthly_budget_usd = 400;
      input.dials = { ...input.dials, ...dials };
      return input;
    };
    const off = await engine.assess(withDials({}));
    if (!off.ok) throw new Error('assess');
    const none = allowance(off.value, cat.value, withDials({}).dials, 400);
    expect(none).toEqual({ families: [], monthly_usd: 40, bought: [] });

    const on = await engine.assess(withDials({ rare_opt_in: ['nuclear_attack'] }));
    if (!on.ok) throw new Error('assess');
    const bought = allowance(on.value, cat.value, withDials({ rare_opt_in: ['nuclear_attack'] }).dials, 400);
    expect(bought.families).toEqual(['nuclear_attack']);
    expect(bought.bought.map((b) => [b.item.item_id, b.families])).toEqual([['radiation_meter', ['nuclear_attack']]]);

    // A family with nothing made for it buys nothing: the money stays in the main plan.
    const pandemic = await engine.assess(withDials({ rare_opt_in: ['severe_pandemic'] }));
    if (!pandemic.ok) throw new Error('assess');
    expect(allowance(pandemic.value, cat.value, withDials({ rare_opt_in: ['severe_pandemic'] }).dials, 400).bought).toEqual([]);
  });
});

describe('"Also checked"', () => {
  it('reads v0.2 notes with each rate in words, brackets and all', () => {
    const packet = [
      '### Notes on these numbers',
      '',
      '- These chances are for Philadelphia County as a whole.',
      '- Also checked, and under 1 in 100,000 a year here: dust storms (none recorded here), sinkholes (about 1 in 120,000 a year), an asteroid or comet impact (fewer than 1 in 1,000,000 a year) and a Yellowstone super-eruption (about 1 in 730,000 a year).',
    ].join('\n');
    expect(parseAlsoChecked(packet)).toEqual({
      lead: 'Also checked, and under 1 in 100,000 a year here',
      items: [
        { name: 'dust storms', rate: 'none recorded here' },
        { name: 'sinkholes', rate: 'about 1 in 120,000 a year' },
        { name: 'an asteroid or comet impact', rate: 'fewer than 1 in 1,000,000 a year' },
        { name: 'a Yellowstone super-eruption', rate: 'about 1 in 730,000 a year' },
      ],
    });
  });

  it('reads the v0.1 note (a colon inside brackets before the list), and Markdown escapes', () => {
    expect(parseAlsoChecked('- Also checked, and too rare here to list (under 1 in 100,000 a year): landslides and wildfires.')).toEqual({
      lead: 'Also checked, and too rare here to list (under 1 in 100,000 a year)',
      items: [{ name: 'landslides' }, { name: 'wildfires' }],
    });
    expect(parseAlsoChecked('- Also checked, and under 1 in 100,000 a year here: dam or levee failures \\(none recorded here\\).')?.items).toEqual([
      { name: 'dam or levee failures', rate: 'none recorded here' },
    ]);
    expect(parseAlsoChecked('No such note.')).toBeNull();
    expect(parseAlsoChecked(undefined)).toBeNull();
  });

  it('comes from the engine packet on every fixture, in the mock and in the real v0.1 goldens', async () => {
    const engine = createMockEngine();
    const cat = await engine.catalogue();
    if (!cat.ok) throw new Error('catalogue');
    for (const [name, input] of Object.entries(FIXTURES)) {
      const r = await engine.assess(input);
      if (!r.ok) throw new Error(name);
      const also = alsoCheckedFor(r.value, cat.value)!;
      expect(also.lead, name).toBe('Also checked, and under 1 in 100,000 a year here');
      expect(also.items.at(-1), name).toEqual({ name: 'a Yellowstone super-eruption', rate: 'about 1 in 730,000 a year' });
      // Nothing on the household's own list is also "checked and too rare".
      const listed = new Set(r.value.register.map((h) => h.name.toLowerCase()));
      for (const i of also.items) expect(listed.has(i.name), `${name}: ${i.name}`).toBe(false);
    }
  });
});
