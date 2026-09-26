import { describe, expect, it } from 'vitest';

import type { Catalogue, PlanItem, PlanOutput, RequirementLine } from '../engine/types';
import { itemSourceIds, keyedItems, requirementsFor } from './lookup';

const item = (item_id: string, kind: PlanItem['kind'], tier: PlanItem['tier'] = 'h72'): PlanItem => ({
  item_id,
  name: item_id,
  kind,
  quantity: 1,
  unit: 'unit',
  est_cost_usd: 1,
  price_band: { low: 1, high: 1 },
  buckets: [],
  hazards: [],
  why: '',
  risk_reduction: 0,
  tier,
});

describe('plan item keys', () => {
  it('tells a last deposit from the purchase of the same item in the same month, and never repeats a key', () => {
    const keys = keyedItems([item('cash', 'reserve'), item('cash', 'purchase'), item('cash', 'purchase'), item('cash', 'purchase', 'w2')]).map((k) => k.key);
    expect(keys).toEqual(['reserve:cash:h72', 'purchase:cash:h72', 'purchase:cash:h72#1', 'purchase:cash:w2']);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("the sources behind an item's numbers", () => {
  const catalogue = {
    items: [
      { id: 'water_stored', quantity_rule: 'water_gallons', citations: ['price_obs', 'ready_gov_water'] },
      { id: 'plan_contacts', quantity_rule: '', citations: ['ready_gov_plan'] },
    ],
  } as unknown as Catalogue;
  const line = (id: string, rule: string, citations: string[]) => ({ id, rule, citations }) as unknown as RequirementLine;
  const output = {
    requirements: [
      line('water_out.water_gallons', 'water_gallons', ['ready_gov_water', 'cdc_water_storage']),
      line('water_out.water_gallons.alt', 'water_reused_bottles', ['other']),
    ],
  } as unknown as PlanOutput;

  it("finds the requirement lines behind the quantity by the item's quantity rule", () => {
    expect(requirementsFor(catalogue, output, 'water_stored').map((r) => r.id)).toEqual(['water_out.water_gallons']);
    expect(requirementsFor(catalogue, output, 'plan_contacts')).toEqual([]);
    expect(requirementsFor(catalogue, undefined, 'water_stored')).toEqual([]);
  });

  it('lists the quantity sources first, then the item and price sources, each once', () => {
    expect(itemSourceIds(catalogue, output, 'water_stored')).toEqual(['ready_gov_water', 'cdc_water_storage', 'price_obs']);
    expect(itemSourceIds(catalogue, output, 'plan_contacts')).toEqual(['ready_gov_plan']);
    expect(itemSourceIds(catalogue, output, 'unknown')).toEqual([]);
  });
});
