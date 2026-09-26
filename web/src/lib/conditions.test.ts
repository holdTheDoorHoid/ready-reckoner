/** The rr-content condition tests (policy.rs), ported: the screen must trim a block as the packet does. */
import { describe, expect, it } from 'vitest';

import { createMockEngine } from '../engine/mock';
import { FIXTURES } from '../engine/fixtures';
import type { PlanOutput } from '../engine/types';
import { applyConditions, applyConditionsFor, householdFacts, parseCondition, type HouseholdFacts } from './conditions';

/** A high-rise flat, someone who is deaf, a generator, SNAP, and tornadoes likely enough to matter. */
const FLAT: HouseholdFacts = {
  hazardRelevant: (h) => h === 'tornado',
  home: () => 'apartment_high_rise',
  hasAccessNeed: (n) => n === 'hearing',
  hasItem: (i) => i === 'power_generator',
  hasBenefit: (b) => b === 'snap_wic',
};

describe('conditional spans', () => {
  it('are kept or dropped cleanly', () => {
    const text = 'Warm the core first. {if:avalanche}In avalanche country, carry a beacon.{/if} Check on neighbours.';
    expect(applyConditions(text, () => true)).toBe('Warm the core first. In avalanche country, carry a beacon. Check on neighbours.');
    expect(applyConditions(text, () => false)).toBe('Warm the core first. Check on neighbours.');
    // At the end of a paragraph, and inside a sentence.
    expect(applyConditions('Stay inside. {if:tsunami}Do not go back to the shore.{/if}\n\nNext.', () => false)).toBe('Stay inside. \n\nNext.');
    const clause = 'Leave if you are told to{if:landslide}, or if you feel unsafe near a slope{/if}.';
    expect(applyConditions(clause, () => false)).toBe('Leave if you are told to.');
    expect(applyConditions(clause, (h) => h === 'landslide')).toBe('Leave if you are told to, or if you feel unsafe near a slope.');
    // At the start of a line.
    expect(applyConditions('{if:drought}In a drought, save water.{/if} Then rest.', () => false)).toBe('Then rest.');
  });

  it('leave malformed markers as they are', () => {
    expect(applyConditions('a {if:avalanche}b', () => false)).toBe('a {if:avalanche}b');
    expect(applyConditions('a b{/if}', () => false)).toBe('a b{/if}');
  });

  it('parse every kind of condition, and reject malformed ones', () => {
    expect(parseCondition('avalanche')).toEqual({ kind: 'hazard', id: 'avalanche' });
    expect(parseCondition('home:apartment_high_rise')).toEqual({ kind: 'home', kinds: ['apartment_high_rise'] });
    expect(parseCondition('not_home:apartment_low_rise|apartment_high_rise')).toEqual({ kind: 'not_home', kinds: ['apartment_low_rise', 'apartment_high_rise'] });
    expect(parseCondition('home:castle')).toBeNull();
    expect(parseCondition('home:')).toBeNull();
    expect(parseCondition('home:detached|detached')).toBeNull();
    expect(parseCondition('need:hearing|vision')).toEqual({ kind: 'need', values: ['hearing', 'vision'] });
    expect(parseCondition('has:power_generator')).toEqual({ kind: 'has', values: ['power_generator'] });
    expect(parseCondition('benefit:snap_wic|federal_pay')).toEqual({ kind: 'benefit', values: ['snap_wic', 'federal_pay'] });
    for (const bad of ['need:telepathy', 'need:', 'need:hearing|hearing', 'benefit:lottery', 'has:Power Generator', 'has:a||b']) {
      expect(parseCondition(bad), bad).toBeNull();
    }
  });

  it('apply every kind for a household; a household-free view keeps every span; a malformed one keeps its text', () => {
    const text =
      'A. {if:tornado}B.{/if} {if:hurricane}C.{/if} {if:home:apartment_high_rise}D.{/if} ' +
      '{if:not_home:apartment_high_rise}E.{/if} {if:need:hearing|vision}F.{/if} ' +
      '{if:need:dialysis}G.{/if} {if:has:power_generator}H.{/if} ' +
      '{if:has:water_drum_55gal}I.{/if} {if:benefit:snap_wic}J.{/if} ' +
      '{if:benefit:va}K.{/if} Z.';
    expect(applyConditionsFor(text, FLAT)).toBe('A. B. D. F. H. J. Z.');
    expect(applyConditionsFor(text, null)).toBe('A. B. C. D. E. F. G. H. I. J. K. Z.');
    expect(applyConditionsFor('{if:need:x}Y.{/if}', FLAT)).toBe('Y.');
  });
});

describe('the household as the conditions see it', () => {
  it('keeps a hazard with a ten-year chance of at least 1 in 100, drops one not in the register, and keeps unknown ids', async () => {
    const input = FIXTURES['philadelphia-renters-4'];
    const r = await createMockEngine().assess(input);
    if (!r.ok) throw new Error(r.error.message);
    const output: PlanOutput = r.value;
    const facts = householdFacts(input, output)!;
    const likely = output.register.find((h) => 1 - Math.exp(-10 * h.rate_per_year) >= 0.01)!;
    expect(facts.hazardRelevant(likely.id)).toBe(true);
    expect(output.register.some((h) => h.id === 'tsunami')).toBe(false);
    expect(facts.hazardRelevant('tsunami')).toBe(false);
    expect(facts.hazardRelevant('zombies')).toBe(true);
    expect(facts.home()).toBe(input.housing.kind);
    expect(facts.hasBenefit('snap_wic')).toBe(false);
    // Before there is an assessment, every hazard's advice stays.
    expect(householdFacts(input, undefined)!.hazardRelevant('tsunami')).toBe(true);
    expect(householdFacts(null, output)).toBeNull();
  });

  it('knows the access needs, the benefits and the items the household has or its plan buys', async () => {
    const input = structuredClone(FIXTURES['philadelphia-renters-4']);
    input.people[0]!.access_needs = ['limited_english'];
    input.finances.benefits = ['va'];
    input.existing = [{ item_id: 'lantern', qty: 1 }];
    const r = await createMockEngine().assess(input);
    if (!r.ok) throw new Error(r.error.message);
    const facts = householdFacts(input, r.value)!;
    expect(facts.hasAccessNeed('limited_english')).toBe(true);
    expect(facts.hasAccessNeed('hearing')).toBe(false);
    expect(facts.hasBenefit('va')).toBe(true);
    expect(facts.hasItem('lantern')).toBe(true);
    expect(facts.hasItem('water_stored')).toBe(true);
    expect(facts.hasItem('radiation_meter')).toBe(false);
  });
});
