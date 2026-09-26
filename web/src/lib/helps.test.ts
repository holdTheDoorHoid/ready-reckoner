/**
 * "What helps" on the hazard cards (round-2 walkthrough W4, W5, W6), checked against the real
 * engine's Philadelphia plan (fixtures/golden) and the real item catalogue (content/items).
 */
import { beforeAll, describe, expect, it } from 'vitest';

import type { Catalogue, PlanItem, PlanOutput } from '../engine/types';
import { golden, realCatalogue, realItems } from '../test/real';
import { helpsFor, LIFE_SAVING } from './helps';
import { catalogueItem } from './lookup';

const COLD = ['cold_wave', 'winter_weather', 'ice_storm'];

let cat: Catalogue;
let philly: PlanOutput;
beforeAll(async () => {
  cat = await realCatalogue();
  philly = golden('philadelphia-renters-4');
});

const ids = (items: PlanItem[]) => items.map((i) => i.item_id);
const extras = (id: string) => catalogueItem(cat, id)?.hazard_extras ?? [];

describe('the real catalogue as the tests read it', () => {
  it('has every item with its id, buckets and flags', () => {
    const items = realItems();
    expect(items.length).toBeGreaterThan(90);
    expect(new Set(items.map((i) => i.id)).size).toBe(items.length);
    for (const i of items) {
      expect(i.id, 'id').toMatch(/^[a-z0-9_]+$/);
      expect(i.buckets.length, i.id).toBeGreaterThan(0);
      expect(Array.isArray(i.hazard_extras), i.id).toBe(true);
    }
    expect(catalogueItem(cat, 'thermal_warm_room_plan')?.hazard_extras).toEqual(COLD);
    expect(catalogueItem(cat, 'fire_smoke_alarm')?.life_safety).toBe(true);
    // Every id the web treats as life-saving is still in the catalogue.
    for (const id of LIFE_SAVING) expect(catalogueItem(cat, id), id).toBeDefined();
  });
});

describe('what helps on a hazard card', () => {
  it('heat wave (W4): heat items, never the warm room or other winter cover', () => {
    const helps = helpsFor(philly, cat, 'heat_wave');
    expect(ids(helps)).not.toContain('thermal_warm_room_plan');
    for (const i of helps) expect(extras(i.item_id).some((h) => COLD.includes(h)), i.item_id).toBe(false);
    const top = ids(helps.slice(0, 3));
    expect(top).toContain('thermal_cool_room_plan');
    expect(top).toContain('thermal_battery_fan');
  });

  it('cold wave: the warm room first, never the fan, the cool room or cooling towels', () => {
    const helps = helpsFor(philly, cat, 'cold_wave');
    expect(helps[0]?.item_id).toBe('thermal_warm_room_plan');
    for (const id of ['thermal_battery_fan', 'thermal_cool_room_plan', 'thermal_cooling_towel']) expect(ids(helps)).not.toContain(id);
    for (const i of helps) expect(extras(i.item_id).includes('heat_wave'), i.item_id).toBe(false);
  });

  it('medical emergency (W5): the first-aid kit and bleeding-control kit before 988 and thermometers; no wildfire masks', () => {
    const helps = helpsFor(philly, cat, 'medical_emergency');
    expect(ids(helps.slice(0, 2))).toEqual(['med_first_aid_kit', 'med_bleeding_control_kit']);
    const at = (id: string) => ids(helps).indexOf(id);
    expect(at('med_988_saved')).toBeGreaterThan(1);
    expect(at('med_thermometer')).toBeGreaterThan(1);
    expect(ids(helps)).not.toContain('med_n95_respirators');
  });

  it('house fire: life-saving items first (alarms, the extinguisher)', () => {
    const helps = helpsFor(philly, cat, 'house_fire');
    const lifeSaving = (i: PlanItem) => !!catalogueItem(cat, i.item_id)?.life_safety || LIFE_SAVING.has(i.item_id);
    const firstOther = helps.findIndex((i) => !lifeSaving(i));
    expect(firstOther).toBeGreaterThanOrEqual(2);
    expect(helps.slice(firstOther).some(lifeSaving)).toBe(false);
    expect(ids(helps.slice(0, 3))).toEqual(expect.arrayContaining(['fire_test_alarms', 'fire_co_alarm']));
  });

  it('never offers a savings deposit, never twice the same item, and puts done items last, on every card', () => {
    for (const name of ['philadelphia-renters-4', 'coos-bay-well-owner-2', 'miami-condo-retiree-1', 'hays-kansas-farm-5', 'phoenix-apartment-cpap-1']) {
      const out = golden(name);
      for (const h of out.register.filter((x) => x.display === 'ranked')) {
        const helps = helpsFor(out, cat, h.id);
        expect(helps.some((i) => i.kind === 'reserve'), `${name} ${h.id}`).toBe(false);
        expect(new Set(ids(helps)).size, `${name} ${h.id}`).toBe(helps.length);
        const firstDone = helps.findIndex((i) => i.done);
        if (firstDone >= 0) expect(helps.slice(firstDone).every((i) => i.done), `${name} ${h.id}`).toBe(true);
        // Nothing made only for other hazards.
        for (const i of helps) {
          const e = extras(i.item_id);
          expect(e.length === 0 || e.includes(h.id), `${name} ${h.id}: ${i.item_id}`).toBe(true);
        }
      }
    }
  });

  it('leaves out an item whose requirement class is the other half of heat and cold, even without hazard_extras', () => {
    const out: PlanOutput = JSON.parse(JSON.stringify(philly));
    // A made-up winter item with no hazard_extras, sized by the real `blankets` rule (thermal_cold),
    // which the engine links to both heat and cold (both drive the thermal bucket).
    const item: PlanItem = { ...out.plan.months[0]!.items[0]!, item_id: 'test_winter_throw', name: 'Winter throw', kind: 'purchase', buckets: ['thermal'], hazards: ['heat_wave', 'cold_wave'], done: false };
    out.plan.months[0]!.items.push(item);
    const withItem: Catalogue = { ...cat, items: [...cat.items, { ...catalogueItem(cat, 'thermal_blankets')!, id: 'test_winter_throw', hazard_extras: [], quantity_rule: 'blankets' }] };
    expect(out.requirements.some((r) => r.rule === 'blankets' && r.item_class === 'thermal_cold')).toBe(true);
    expect(ids(helpsFor(out, withItem, 'heat_wave'))).not.toContain('test_winter_throw');
    expect(ids(helpsFor(out, withItem, 'cold_wave'))).toContain('test_winter_throw');
  });
});
