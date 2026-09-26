import { describe, expect, it } from 'vitest';

import { FIXTURE_NAMES, FIXTURES } from './fixtures';
import { createMockEngine, mockDefaults } from './mock';
import type { EngineError, LocationSuggestions, PlanInput, PlanOutput, Problem } from './types';
import { BUCKET_IDS, ENGINE_API_VERSION, RETURN_PERIODS, TARGET_LADDER_DAYS } from './types';

const engine = createMockEngine();

function clone(input: PlanInput): PlanInput {
  return JSON.parse(JSON.stringify(input)) as PlanInput;
}

async function assess(input: PlanInput): Promise<PlanOutput> {
  const r = await engine.assess(input);
  if (!r.ok) throw new Error(`${r.error.code}: ${JSON.stringify(r.error.details)}`);
  return r.value;
}

async function assessError(input: PlanInput): Promise<EngineError> {
  const r = await engine.assess(input);
  if (r.ok) throw new Error('expected an error');
  return r.error;
}

function problems(e: EngineError): Problem[] {
  return (e.details as { problems: Problem[] }).problems;
}

function days(o: PlanOutput, id: string): number {
  const t = o.buckets.find((b) => b.id === id)!.target;
  if (t.kind !== 'days') throw new Error(`${id} is not a duration bucket`);
  return t.value;
}

function withBudget(input: PlanInput, monthly: number, oneOff: number): PlanInput {
  const next = clone(input);
  next.finances.monthly_budget_usd = monthly;
  next.finances.one_off_budget_usd = oneOff;
  return next;
}

/** Month each not-yet-done item is bought in, keyed `item_id:tier`. */
function purchaseMonths(o: PlanOutput): Map<string, number> {
  const out = new Map<string, number>();
  for (const m of o.plan.months) {
    for (const item of m.items) if (!item.done && item.kind !== 'free_action') out.set(`${item.item_id}:${item.tier}`, m.index);
  }
  return out;
}

describe('mock engine: shape of every fixture', () => {
  for (const name of FIXTURE_NAMES) {
    it(name, async () => {
      const o = await assess(FIXTURES[name]);
      const cat = await engine.catalogue();
      if (!cat.ok) throw new Error('catalogue');
      expect(o.api_version).toBe(ENGINE_API_VERSION);
      expect(o.buckets.map((b) => b.id)).toEqual([...BUCKET_IDS]);
      for (const b of o.buckets) {
        const info = cat.value.buckets.find((x) => x.id === b.id)!;
        expect(b.target.kind).toBe(info.target_kind);
        expect(b.covered.kind).toBe(b.target.kind);
        expect(b.covered_today.kind).toBe(b.target.kind);
        if (b.target.kind === 'days' && b.covered.kind === 'days' && b.covered_today.kind === 'days') {
          expect(TARGET_LADDER_DAYS).toContain(b.target.value);
          expect(b.target.low).toBeLessThanOrEqual(b.target.value);
          expect(b.target.high).toBeGreaterThanOrEqual(b.target.value);
          expect(b.covered.low).toBe(b.covered.value);
          expect(b.covered.high).toBe(b.covered.value);
          expect(b.covered_today.low).toBe(b.covered_today.value);
          expect(b.covered_today.high).toBe(b.covered_today.value);
          // What the household has now never exceeds where the plan takes it, nor the target.
          expect(b.covered_today.value).toBeLessThanOrEqual(b.covered.value);
          expect(b.covered.value).toBeLessThanOrEqual(b.target.value);
        }
        if (b.covered.kind === 'readiness' && b.covered_today.kind === 'readiness') {
          expect(b.covered_today.done).toBeLessThanOrEqual(b.covered.done);
        }
        if (b.relief) expect(b.relief.help_arrives_days).toBeLessThanOrEqual(b.relief.mostly_restored_days);
        const shares = b.contributions.reduce((s, c) => s + c.share, 0);
        if (b.contributions.length) expect(shares).toBeCloseTo(1, 5);
      }
      // Ranked hazards come first, then the rare catastrophic ones.
      const firstRare = o.register.findIndex((h) => h.display === 'rare_catastrophic');
      expect(firstRare).toBeGreaterThan(0);
      expect(o.register.slice(firstRare).every((h) => h.display === 'rare_catastrophic')).toBe(true);
      for (const h of o.register) {
        expect(h.frequency_sentence).toMatch(/of 100 households like yours|Almost all households|Fewer than 1 in 100/);
        expect(h.probability_range[0]).toBeLessThanOrEqual(h.annual_probability);
        expect(h.probability_range[1]).toBeGreaterThanOrEqual(h.annual_probability);
      }
      expect(o.packet_markdown).toContain('# Your preparedness packet');
      for (const section of ['Your risks', 'Your targets', 'Your plan', 'Checklists', 'Family plan', 'Documents and money', 'Special needs', 'Maintenance calendar', 'Sources']) {
        expect(o.packet_markdown).toContain(`## ${section}`);
      }
    });
  }
});

describe('mock engine: citations', () => {
  it('marks every citation as a mock placeholder', async () => {
    const cat = await engine.catalogue();
    if (!cat.ok) throw new Error('catalogue');
    for (const c of cat.value.citations) {
      expect(c.id).toMatch(/^mock_/);
      expect(c.title).toMatch(/^Mock: /);
      expect(c.url).toMatch(/^https:\/\/example\.org\//);
    }
    for (const item of cat.value.items) {
      expect(item.citations.length).toBeGreaterThan(0);
      for (const id of item.citations) expect(id).toMatch(/^mock_/);
    }
  });

  it('cites every plan item, hazard, bucket and requirement with sources in provenance', async () => {
    const cat = await engine.catalogue();
    if (!cat.ok) throw new Error('catalogue');
    for (const name of FIXTURE_NAMES) {
      const o = await assess(FIXTURES[name]);
      const known = new Set(o.provenance.map((c) => c.id));
      const all = o.plan.months.flatMap((m) => m.items);
      expect(all.length).toBeGreaterThan(0);
      for (const item of all) {
        const entry = cat.value.items.find((i) => i.id === item.item_id);
        expect(entry, item.item_id).toBeDefined();
        expect(entry!.citations.length).toBeGreaterThan(0);
        for (const id of entry!.citations) expect(known.has(id), `${name}: ${item.item_id} cites ${id}`).toBe(true);
      }
      for (const h of o.register) for (const id of h.sources) expect(known.has(id)).toBe(true);
      for (const b of o.buckets) for (const id of [...b.sources, ...(b.relief?.sources ?? [])]) expect(known.has(id)).toBe(true);
      for (const r of o.requirements) for (const id of r.citations) expect(known.has(id)).toBe(true);
    }
  });
});

describe('mock engine: invariants', () => {
  it('is deterministic', async () => {
    for (const name of FIXTURE_NAMES) {
      const a = await engine.assess(FIXTURES[name]);
      const b = await engine.assess(FIXTURES[name]);
      expect(JSON.stringify(a)).toBe(JSON.stringify(b));
    }
  });

  it('never lowers a target when the return-period dial gets more cautious', async () => {
    for (const name of FIXTURE_NAMES) {
      let previous: PlanOutput | undefined;
      for (const rp of RETURN_PERIODS) {
        const input = clone(FIXTURES[name]);
        input.dials.return_period = rp;
        const o = await assess(input);
        if (previous) {
          for (const b of o.buckets) {
            const before = previous.buckets.find((x) => x.id === b.id)!.target;
            if (b.target.kind === 'days' && before.kind === 'days') {
              expect(b.target.value, `${name} ${b.id} at ${rp}`).toBeGreaterThanOrEqual(before.value);
            }
            if (b.target.kind === 'months' && before.kind === 'months') {
              expect(b.target.value).toBeGreaterThanOrEqual(before.value);
            }
          }
        }
        previous = o;
      }
    }
  });

  it('shows more days at 1-in-500 than at 1-in-10 for every household', async () => {
    for (const name of FIXTURE_NAMES) {
      const low = clone(FIXTURES[name]);
      low.dials.return_period = 'one_in_10';
      const high = clone(FIXTURES[name]);
      high.dials.return_period = 'one_in_500';
      expect(days(await assess(high), 'power')).toBeGreaterThan(days(await assess(low), 'power'));
    }
  });

  it('asks for more water when a person joins, and never fewer days', async () => {
    for (const name of FIXTURE_NAMES) {
      const base = await assess(FIXTURES[name]);
      const bigger = clone(FIXTURES[name]);
      bigger.people.push({
        age_band: 'adult',
        pregnant_or_nursing: false,
        medical: { daily_rx: false, refrigerated_rx: false, powered_device: 'none', mobility: 'none', dietary: [], epinephrine: false },
        earner: false,
      });
      const more = await assess(bigger);
      const water = (o: PlanOutput) => o.requirements.find((r) => r.id === 'water_drinking')!.quantity;
      expect(water(more), name).toBeGreaterThan(water(base));
      for (const b of more.buckets) {
        const before = base.buckets.find((x) => x.id === b.id)!.target;
        if (b.target.kind === 'days' && before.kind === 'days') expect(b.target.value).toBeGreaterThanOrEqual(before.value);
      }
    }
  });

  it('gives only free actions when the budget is zero', async () => {
    for (const name of FIXTURE_NAMES) {
      const o = await assess(withBudget(FIXTURES[name], 0, 0));
      const todo = o.plan.months.flatMap((m) => m.items).filter((i) => !i.done);
      expect(todo.length).toBeGreaterThan(0);
      for (const item of todo) {
        expect(item.kind, `${name}: ${item.item_id}`).toBe('free_action');
        expect(item.est_cost_usd).toBe(0);
      }
      expect(o.warnings.some((w) => w.id === 'zero_budget')).toBe(true);
    }
  });

  it('never spends more than the money available by any month', async () => {
    for (const name of FIXTURE_NAMES) {
      for (const [monthly, oneOff] of [
        [0, 0],
        [0, 300],
        [10, 0],
        [60, 0],
        [60, 500],
        [250, 0],
        [1000, 2000],
      ] as const) {
        const o = await assess(withBudget(FIXTURES[name], monthly, oneOff));
        let spent = 0;
        let lastIndex = -1;
        for (const m of o.plan.months) {
          expect(m.index).toBeGreaterThan(lastIndex);
          lastIndex = m.index;
          expect(m.budget_usd).toBe(m.index === 0 ? monthly + oneOff : monthly);
          spent += m.items.filter((i) => !i.done).reduce((s, i) => s + i.est_cost_usd, 0);
          expect(spent, `${name} at $${monthly}/month + $${oneOff}, month ${m.index}`).toBeLessThanOrEqual(oneOff + monthly * (m.index + 1) + 0.01);
        }
      }
    }
  });

  it('never buys anything later when the budget goes up', async () => {
    for (const name of FIXTURE_NAMES) {
      const budgets = [10, 40, 60, 150, 400];
      let previous: PlanOutput | undefined;
      for (const monthly of budgets) {
        const o = await assess(withBudget(FIXTURES[name], monthly, 0));
        if (previous) {
          const before = purchaseMonths(previous);
          const after = purchaseMonths(o);
          for (const [key, month] of before) {
            expect(after.has(key), `${name}: ${key} vanished at $${monthly}`).toBe(true);
            expect(after.get(key)!, `${name}: ${key} at $${monthly}`).toBeLessThanOrEqual(month);
          }
          if (previous.plan.done_month !== undefined) {
            expect(o.plan.done_month).toBeDefined();
            expect(o.plan.done_month!).toBeLessThanOrEqual(previous.plan.done_month);
          }
        }
        previous = o;
      }
    }
  });

  it('never puts rare-catastrophe items in the plan', async () => {
    for (const name of FIXTURE_NAMES) {
      const o = await assess(withBudget(FIXTURES[name], 5000, 5000));
      expect(o.plan.months.flatMap((m) => m.items).some((i) => i.item_id === 'radiation_meter')).toBe(false);
    }
  });

  it('keeps firearms out of every budget: the only firearm item is free', async () => {
    const cat = await engine.catalogue();
    if (!cat.ok) throw new Error('catalogue');
    const firearm = /firearm|\bgun|ammunition|weapon/i;
    for (const item of cat.value.items) {
      const text = [item.name, item.spec, ...item.look_for, ...item.avoid].join(' ');
      if (firearm.test(text)) {
        expect(item.free, item.id).toBe(true);
        expect(item.price_band_usd.high).toBe(0);
      }
    }
  });

  it('counts a finished plan as covered today', async () => {
    for (const name of ['philadelphia-renters-4', 'coos-bay-well-owner-2'] as const) {
      const base = await assess(withBudget(FIXTURES[name], 500, 0));
      const done = new Map<string, number>();
      for (const i of base.plan.months.flatMap((m) => m.items)) done.set(i.item_id, (done.get(i.item_id) ?? 0) + i.quantity);
      const input = withBudget(FIXTURES[name], 500, 0);
      input.existing = [...done].map(([item_id, qty]) => ({ item_id, qty }));
      const after = await assess(input);
      for (const b of after.buckets) {
        const was = base.buckets.find((x) => x.id === b.id)!.covered;
        if (b.covered_today.kind === 'days' && was.kind === 'days') expect(b.covered_today.value, `${name} ${b.id}`).toBe(was.value);
      }
    }
  });

  it('marks an item done once the household records it, and does not lower coverage', async () => {
    for (const name of FIXTURE_NAMES) {
      const base = await assess(withBudget(FIXTURES[name], 60, 0));
      const first = base.plan.months.flatMap((m) => m.items).find((i) => !i.done && i.kind !== 'free_action')!;
      const input = withBudget(FIXTURES[name], 60, 0);
      input.existing.push({ item_id: first.item_id, qty: first.quantity, paid_usd: 12 });
      const after = await assess(input);
      const same = after.plan.months.flatMap((m) => m.items).find((i) => i.item_id === first.item_id && i.tier === first.tier);
      expect(same?.done, `${name}: ${first.item_id}`).toBe(true);
      expect(same?.paid_usd).toBeGreaterThan(0);
      for (const b of after.buckets) {
        const was = base.buckets.find((x) => x.id === b.id)!;
        const [before, beforeToday] = [was.covered, was.covered_today];
        if (b.covered.kind === 'days' && before.kind === 'days') expect(b.covered.value).toBeGreaterThanOrEqual(before.value);
        if (b.covered_today.kind === 'days' && beforeToday.kind === 'days') expect(b.covered_today.value).toBeGreaterThanOrEqual(beforeToday.value);
      }
    }
  });
});

describe('mock engine: scenarios', () => {
  it('offers the Cascadia scenario in Coos Bay, on by default, with a cliff warning', async () => {
    const o = await assess(FIXTURES['coos-bay-well-owner-2']);
    const s = o.scenarios.find((x) => x.id === 'cascadia_m9')!;
    expect(s.on).toBe(true);
    expect(s.effect_summary).toMatch(/Without it/);
    expect(o.warnings.some((w) => w.id === 'cliff_cascadia_m9' && w.severity === 'warn')).toBe(true);

    const off = clone(FIXTURES['coos-bay-well-owner-2']);
    off.dials.scenario_overrides = [{ id: 'cascadia_m9', on: false }];
    const o2 = await assess(off);
    expect(o2.scenarios.find((x) => x.id === 'cascadia_m9')!.on).toBe(false);
    expect(days(o2, 'water_out')).toBeLessThan(days(o, 'water_out'));
    expect(o2.warnings.some((w) => w.id.startsWith('cliff_'))).toBe(false);
  });

  it('offers no scenarios in Philadelphia, and ignores overrides for scenarios that do not apply', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.dials.scenario_overrides = [{ id: 'cascadia_m9', on: true }];
    const o = await assess(input);
    expect(o.scenarios).toEqual([]);
  });
});

describe('mock engine: validation and places', () => {
  it('reports bad input with field paths, in a fixed order', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']);
    input.location.zip = '191';
    input.finances.monthly_budget_usd = -5;
    input.dials.horizon_years = 0;
    input.finances.income.earners = 3;
    const e = await assessError(input);
    expect(e.code).toBe('bad_input');
    expect(problems(e).map((p) => [p.code, p.field])).toEqual([
      ['zip_format', 'location.zip'],
      ['negative_value', 'finances.monthly_budget_usd'],
      ['out_of_range', 'dials.horizon_years'],
      ['earners_mismatch', 'finances.income.earners'],
    ]);
    for (const p of problems(e)) expect(p.message.length).toBeGreaterThan(10);
  });

  it('rejects unknown fields and wrong types as schema problems', async () => {
    const input = clone(FIXTURES['philadelphia-renters-4']) as PlanInput & { surprise?: boolean };
    input.surprise = true;
    (input.housing as unknown as Record<string, unknown>).floor = 'ground';
    const e = await assessError(input);
    expect(problems(e).map((p) => p.field).sort()).toEqual(['housing.floor', 'surprise']);
    expect(problems(e).every((p) => p.code === 'schema')).toBe(true);
  });

  it('flags a scenario toggled twice', async () => {
    const input = clone(FIXTURES['coos-bay-well-owner-2']);
    input.dials.scenario_overrides = [
      { id: 'cascadia_m9', on: true },
      { id: 'cascadia_m9', on: false },
    ];
    expect(problems(await assessError(input))[0]!.code).toBe('duplicate_id');
  });

  it('fails on the placeholder ZIP code from defaults()', async () => {
    const d = await engine.defaults();
    if (!d.ok) throw new Error('defaults');
    expect(d.value).toEqual(mockDefaults());
    expect((await assessError(d.value)).code).toBe('unknown_zip');
    const fixed = clone(d.value);
    fixed.location.zip = '19147';
    await assess(fixed);
  });

  it('asks which county when a ZIP code spans several, largest share first', async () => {
    const r = await engine.resolve_location({ country: 'US', zip: '19087', setting: 'suburban' });
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.code).toBe('ambiguous_zip');
    const s = (r.error.details as LocationSuggestions).suggestions;
    expect(s.map((x) => x.county_name)).toEqual(['Delaware County', 'Chester County', 'Montgomery County']);
    expect(s[0]!.zip_county_share).toBeGreaterThan(s[1]!.zip_county_share!);

    // The pick is recorded as county_fips alongside the ZIP code, and the county decides.
    const picked = await engine.resolve_location({ country: 'US', zip: '19087', county_fips: '42029', setting: 'suburban' });
    expect(picked.ok && picked.value.county_name).toBe('Chester County');
  });

  it('resolves a ZIP code that is mostly in one county without asking', async () => {
    const r = await engine.resolve_location({ country: 'US', zip: '19010', setting: 'suburban' });
    expect(r.ok && r.value.county_fips).toBe('42091');
    expect(r.ok && r.value.zip_county_share).toBe(0.86);
  });

  it('plans for an unlisted ZIP code with its state and says it is sample data', async () => {
    const r = await engine.resolve_location({ country: 'US', zip: '98101', setting: 'urban' });
    expect(r.ok && r.value.state_abbr).toBe('WA');
    expect(r.ok && r.value.data_note).toMatch(/Sample data/);
  });

  it('searches counties by name, code, and "name, state"', async () => {
    const byName = await engine.county_search('phila');
    expect(byName.ok && byName.value[0]!.county_name).toBe('Philadelphia County');
    const byCode = await engine.county_search('42101');
    expect(byCode.ok && byCode.value[0]!.county_fips).toBe('42101');
    const byState = await engine.county_search('Cook, IL');
    expect(byState.ok && byState.value.map((c) => c.county_name)).toEqual(['Cook County']);
    const nothing = await engine.county_search('   ');
    expect(nothing.ok && nothing.value).toEqual([]);
  });
});

describe('mock engine: other functions', () => {
  it('reports versions and attributions, and a data pack version once a pack is loaded', async () => {
    const fresh = createMockEngine();
    const before = await fresh.engine_info();
    if (!before.ok) throw new Error('engine_info');
    expect(before.value.api_version).toBe(ENGINE_API_VERSION);
    expect(before.value.engine_version).toMatch(/^mock/);
    expect(before.value.data_pack_version).toBeUndefined();
    expect(before.value.attributions.some((a) => /not endorsed by FEMA/.test(a.text))).toBe(true);
    const pack = await fresh.load_pack('core/counties.csv', new TextEncoder().encode('fips,name\n42101,Philadelphia\n'));
    expect(pack.ok && pack.value.rows).toBe(2);
    const after = await fresh.engine_info();
    expect(after.ok && after.value.data_pack_version).toBeDefined();
    expect(after.ok && after.value.packs_loaded).toEqual(['core/counties.csv']);
    expect((await fresh.load_pack('empty', new Uint8Array())).ok).toBe(false);
  });

  it('explains a hazard, a bucket, an item, a requirement and a warning', async () => {
    const input = FIXTURES['philadelphia-renters-4'];
    const o = await assess(input);
    const requests = [
      { kind: 'hazard' as const, id: o.register[0]!.id },
      { kind: 'bucket' as const, id: 'power' },
      { kind: 'bucket' as const, id: 'income' },
      { kind: 'item' as const, id: 'water_stored' },
      { kind: 'requirement' as const, id: 'water_drinking' },
      { kind: 'warning' as const, id: o.warnings[0]!.id },
    ];
    for (const req of requests) {
      const r = await engine.explain({ ...req, input });
      expect(r.ok, `${req.kind} ${req.id}`).toBe(true);
      if (!r.ok) continue;
      expect(r.value.title.length).toBeGreaterThan(5);
      expect(r.value.plain.length).toBeGreaterThan(0);
    }
    const missing = await engine.explain({ kind: 'item', id: 'no_such_item', input });
    expect(missing.ok).toBe(false);
  });
});
