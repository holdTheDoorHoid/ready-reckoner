/** Contract v2 answers in the saved plan: they live in `input`, and tested-on dates travel with the items. */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import type { PlanInput } from '../engine/types';
import detroitJson from '../../../fixtures/households/pending/detroit-snap-3.json';
import minotJson from '../../../fixtures/households/pending/minot-missile-field-3.json';
import { MemoryStorage, savedFor } from '../test/helpers';
import { checkSavedPlan, engineInput, exportText, heldQuantity, loadPlan, mergeOwned, parseImport, savePlan, setTestedOn, testedOn } from './persistence';

const detroit = detroitJson as unknown as PlanInput;
const minot = minotJson as unknown as PlanInput;

describe('tested-on dates', () => {
  it('go to the engine with the item, the latest date wherever it was recorded', () => {
    const owned = mergeOwned(
      [{ item_id: 'generator_portable', qty: 1, tested_on: '2026-08-01' }],
      [
        { item_id: 'flashlights_headlamps', qty: 2, date: '2026-10-02', tested_on: '2026-10-02' },
        { item_id: 'flashlights_headlamps', qty: 2, date: '2026-10-09' },
        { item_id: 'generator_portable', qty: 1, date: '2026-10-05', tested_on: '2026-10-20' },
      ],
    );
    expect(owned).toEqual([
      { item_id: 'generator_portable', qty: 2, tested_on: '2026-10-20' },
      { item_id: 'flashlights_headlamps', qty: 4, tested_on: '2026-10-02' },
    ]);
  });

  it('are kept on what the household had before the plan, or else on its latest check-off', () => {
    const plan = savedFor(FIXTURES['philadelphia-renters-4']);
    plan.input.existing = [{ item_id: 'generator_portable', qty: 1 }];
    plan.purchases = [
      { item_id: 'flashlights_headlamps', tier: 'h72', qty: 4, date: '2026-10-02' },
      { item_id: 'flashlights_headlamps', tier: 'w2', qty: 1, date: '2026-11-02' },
    ];
    expect(heldQuantity(plan, 'flashlights_headlamps')).toBe(5);
    expect(setTestedOn(plan, 'generator_portable', '2026-10-03')).toBe(true);
    expect(plan.input.existing[0]!.tested_on).toBe('2026-10-03');
    expect(setTestedOn(plan, 'flashlights_headlamps', '2026-11-05')).toBe(true);
    expect(plan.purchases[1]!.tested_on).toBe('2026-11-05');
    expect(plan.purchases[0]!.tested_on).toBeUndefined();
    expect(testedOn(plan, 'flashlights_headlamps')).toBe('2026-11-05');
    expect(engineInput(plan).existing.find((o) => o.item_id === 'flashlights_headlamps')?.tested_on).toBe('2026-11-05');
    // Clearing removes every copy; an item the household does not have takes no date.
    expect(setTestedOn(plan, 'flashlights_headlamps', undefined)).toBe(true);
    expect(testedOn(plan, 'flashlights_headlamps')).toBeUndefined();
    expect(setTestedOn(plan, 'power_station', '2026-11-05')).toBe(false);
  });

  it('survive the round trip through storage and a saved file; a bad date on a check-off is refused', () => {
    const plan = savedFor(minot, { purchases: [{ item_id: 'power_station', tier: 'm1', qty: 1, date: '2026-10-05', tested_on: '2026-10-06' }] });
    const storage = new MemoryStorage();
    expect(savePlan(storage, plan)).toBe(true);
    const back = loadPlan(storage).plan!;
    expect(back.purchases[0]!.tested_on).toBe('2026-10-06');
    expect(back.input.existing.filter((o) => o.tested_on).map((o) => o.item_id)).toEqual(['power_headlamp', 'power_lantern']);
    const file = parseImport(exportText(plan));
    expect(file.ok && file.plan.purchases[0]!.tested_on).toBe('2026-10-06');
    const bad = JSON.parse(exportText(plan)) as { purchases: { tested_on: string }[] };
    bad.purchases[0]!.tested_on = 'last week';
    expect(checkSavedPlan(bad).ok).toBe(false);
  });
});

describe('the other v2 answers', () => {
  it('are part of the household, so they are saved, exported and imported as they are', () => {
    const plan = savedFor(detroit);
    const storage = new MemoryStorage();
    savePlan(storage, plan);
    const back = loadPlan(storage).plan!;
    expect(back.input.family_plan).toEqual(detroit.family_plan);
    expect(back.input.people[2]!.access_needs).toEqual(['hearing']);
    expect(back.input.housing).toMatchObject({ below_grade_bedroom: true, cooking: 'gas', water_system_record: 'occasional_notices' });
    expect(back.input.finances.benefits).toEqual(['snap_wic']);
    expect(back.input.dials.minimum_kit).toBe(true);
    const file = parseImport(exportText(plan));
    expect(file.ok && file.plan.input).toEqual(detroit);
    // The engine sees them unchanged (the family plan too: the engine tidies its own copy).
    expect(engineInput(plan).family_plan).toEqual(detroit.family_plan);
    // A bare household file (a fixture) imports as a new plan.
    const bare = parseImport(JSON.stringify(minot));
    expect(bare.ok && bare.plan.input.dials.rare_opt_in).toEqual(['nuclear_attack', 'geomagnetic_storm']);
  });
});
