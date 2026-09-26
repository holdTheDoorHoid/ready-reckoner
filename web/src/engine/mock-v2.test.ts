/** The mock engine's contract v2 input handling: it accepts, checks and tidies every new answer. */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from './fixtures';
import { createMockEngine, mockDefaults, tidyInput } from './mock';
import type { EngineError, PlanInput, PlanOutput, Problem } from './types';
import cameron from '../../../fixtures/households/pending/cameron-insulin-well-farm-2.json';
import detroit from '../../../fixtures/households/pending/detroit-snap-3.json';
import galveston from '../../../fixtures/households/pending/galveston-highrise-1.json';
import minot from '../../../fixtures/households/pending/minot-missile-field-3.json';
import missoula from '../../../fixtures/households/pending/missoula-smoke-2.json';
import sacramento from '../../../fixtures/households/pending/sacramento-leveed-2.json';
import sanJuan from '../../../fixtures/households/pending/san-juan-2.json';

const engine = createMockEngine();

/** The households staged for contract v2 (fixtures/households/pending), every v2 input among them. */
const PENDING: Record<string, PlanInput> = {
  cameron: cameron as unknown as PlanInput,
  detroit: detroit as unknown as PlanInput,
  galveston: galveston as unknown as PlanInput,
  minot: minot as unknown as PlanInput,
  missoula: missoula as unknown as PlanInput,
  sacramento: sacramento as unknown as PlanInput,
  'san-juan': sanJuan as unknown as PlanInput,
};

const clone = <T>(x: T): T => JSON.parse(JSON.stringify(x)) as T;

async function assess(input: PlanInput): Promise<PlanOutput> {
  const r = await engine.assess(input);
  if (!r.ok) throw new Error(`${r.error.code}: ${JSON.stringify(r.error.details)}`);
  return r.value;
}

async function problems(input: unknown): Promise<Problem[]> {
  const r = await engine.assess(input as PlanInput);
  if (r.ok) throw new Error('expected bad_input');
  return ((r.error as EngineError).details as { problems: Problem[] }).problems;
}

function days(o: PlanOutput, id: string): number {
  const t = o.buckets.find((b) => b.id === id)!.target;
  const c = o.buckets.find((b) => b.id === id)!.covered_today;
  if (t.kind !== 'days' || c.kind !== 'days') throw new Error(id);
  return c.value;
}

describe('mock engine: contract v2 inputs', () => {
  for (const [name, input] of Object.entries(PENDING)) {
    it(`plans for the staged v2 household ${name}`, async () => {
      const o = await assess(input);
      expect(o.buckets).toHaveLength(15);
      expect(o.packet_markdown).toContain('## Wallet cards');
    });
  }

  it('sends the v2 defaults the engine sends, and leaves out what means "not asked"', async () => {
    const d = await engine.defaults();
    if (!d.ok) throw new Error('defaults');
    expect(d.value).toEqual(mockDefaults());
    expect(d.value.housing.below_grade_bedroom).toBe(false);
    expect(d.value.people[0]!.access_needs).toEqual([]);
    expect(d.value.finances.benefits).toEqual([]);
    expect(d.value.dials).toMatchObject({ rare_opt_in: [], minimum_kit: false, long_horizon: false, rare_catastrophic_opt_in: false });
    for (const key of ['cooking', 'raw_water_source', 'water_system_record'] as const) expect(key in d.value.housing).toBe(false);
    expect('sewer_backup' in d.value.finances.insurance).toBe(false);
    expect('family_plan' in d.value).toBe(false);
  });

  it('checks the new answers: enums, dates, and rare families (unknown_id)', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']) as PlanInput & Record<string, unknown>;
    (input.housing as unknown as Record<string, unknown>).cooking = 'wood';
    (input.people[0] as unknown as Record<string, unknown>).access_needs = ['telepathy'];
    input.existing = [{ item_id: 'lantern', qty: 1, tested_on: 'yesterday' }];
    (input.family_plan as unknown) = { routes: 'north', trusted_circle: [{ holds: ['a_pony'] }], surprise: 'x' };
    const schema = await problems(input);
    expect(schema.every((p) => p.code === 'schema')).toBe(true);
    expect(schema.map((p) => p.field).sort()).toEqual(
      ['existing[0].tested_on', 'family_plan.routes', 'family_plan.surprise', 'family_plan.trusted_circle[0].holds[0]', 'housing.cooking', 'people[0].access_needs[0]'].sort(),
    );

    const families = clone(FIXTURES['philadelphia-renters-4']);
    families.dials.rare_opt_in = ['nuclear_attack', 'house_fire', 'Nuclear Attack', 'all'];
    expect((await problems(families)).map((p) => [p.code, p.field])).toEqual([
      ['unknown_id', 'dials.rare_opt_in[1]'],
      ['id_format', 'dials.rare_opt_in[2]'],
    ]);
  });

  it('tidies the family plan like PlanInput::from_json, and never reports a problem with it', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.family_plan = { meeting_place_near: '  The corner  ', work_plans: '   ', numbers_by_heart: ['', ' 555-0100 '] };
    expect(tidyInput(input).family_plan).toEqual({ meeting_place_near: 'The corner', numbers_by_heart: ['555-0100'] });
    expect(input.family_plan.work_plans).toBe('   ');
    const o = await assess(input);
    expect(o.packet_markdown).toContain('| Meeting place near home | The corner |');
    expect(o.packet_markdown).toContain('**Numbers we know by heart:** 555-0100');
    input.family_plan = { work_plans: '  ' };
    expect(tidyInput(input).family_plan).toBeUndefined();
    expect((await assess(input)).packet_markdown).toContain('Fill this in together');
  });

  it('prints the family plan word for word and one wallet card per person, and escapes what could break the page', async () => {
    const d = PENDING.detroit!;
    const o = await assess(d);
    const md = o.packet_markdown;
    expect(md.indexOf('## Your family plan')).toBeLessThan(md.indexOf('## Your risks'));
    expect(md).toContain('| Out-of-area contact (name and phone) | Cousin Tanya in Columbus, 555-0140 |');
    expect(md).toContain('| Tanya | 555-0140 | a spare key, copies of our papers |');
    expect(md.match(/^> \*\*Wallet card: person \d/gm)).toHaveLength(d.people.length);
    expect(md).toContain('> - Lawyer: Legal aid office, 555-0143');
    expect(md).toContain('**Help in an emergency:** person 3 is deaf or hard of hearing.');
    const tricky = clone(d);
    tricky.family_plan!.meeting_place_near = 'Corner | <b>bold</b> *star*\nnext line';
    const t = (await assess(tricky)).packet_markdown;
    expect(t).toContain('| Meeting place near home | Corner \\| \\<b\\>bold\\</b\\> \\*star\\* next line |');
  });

  it('uses the new home answers: a gas stove boils water, and a filter needs water to filter', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.existing = [];
    const without = await assess(input);
    input.housing.cooking = 'gas';
    const withGas = await assess(input);
    const target = withGas.buckets.find((b) => b.id === 'water_boil')!.target;
    if (target.kind !== 'days') throw new Error('water_boil is a duration bucket');
    expect(days(without, 'water_boil')).toBeLessThan(target.value);
    expect(days(withGas, 'water_boil')).toBe(target.value);

    const coos = clone(FIXTURES['coos-bay-well-owner-2']);
    const withFilter = (await assess(coos)).plan.months.flatMap((m) => m.items).some((i) => i.item_id === 'water_filter');
    coos.housing.raw_water_source = 'none';
    const noSource = (await assess(coos)).plan.months.flatMap((m) => m.items).some((i) => i.item_id === 'water_filter');
    expect(withFilter).toBe(true);
    expect(noSource).toBe(false);
  });

  it('accepts the answers it does not model without changing its plan: benefits, insurance extras, dials, tested-on dates', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.existing = [{ item_id: 'flashlights_headlamps', qty: 4 }];
    const before = await assess(input);
    input.finances.benefits = ['federal_pay'];
    input.finances.insurance.sewer_backup = false;
    input.finances.insurance.life_or_disability = true;
    input.dials.minimum_kit = true;
    input.dials.long_horizon = true;
    input.dials.rare_opt_in = ['severe_pandemic'];
    input.existing = [{ item_id: 'flashlights_headlamps', qty: 4, tested_on: '2026-09-01' }];
    const after = await assess(input);
    expect(after.plan.months).toEqual(before.plan.months);
    expect(after.buckets).toEqual(before.buckets);
  });

  it('spends the rare allowance by family: the radiation meter answers the nuclear family only', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    const meter = async () => (await assess(input)).plan.months.flatMap((m) => m.items).some((i) => i.item_id === 'radiation_meter');
    expect(await meter()).toBe(false);
    input.dials.rare_opt_in = ['severe_pandemic'];
    expect(await meter()).toBe(false);
    input.dials.rare_opt_in = ['nuclear_attack'];
    expect(await meter()).toBe(true);
    input.dials.rare_opt_in = ['all'];
    expect(await meter()).toBe(true);
    input.dials.rare_opt_in = [];
    input.dials.rare_catastrophic_opt_in = true;
    expect(await meter()).toBe(true);
  });
});
