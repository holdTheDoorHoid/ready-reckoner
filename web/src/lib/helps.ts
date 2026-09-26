/**
 * "What helps" on a hazard card: the plan items that answer one hazard, most direct first.
 *
 * 1. Candidates are the free steps and purchases (never a savings deposit) that the engine links
 *    to the hazard (`PlanItem.hazards`, the main hazards behind the item's consequences) or that
 *    the catalogue says are made for it (`Item.hazard_extras`).
 * 2. An item made for other hazards is left out: a heat-wave card never offers the warm-room step
 *    for a winter outage, a cold-wave card never offers the fan, and a medical-emergency card never
 *    offers wildfire masks. The engine splits heat from cold cover by requirement class
 *    (`RequirementLine.item_class` `thermal_heat` / `thermal_cold`), but the table that joins an
 *    item to its line is internal to rr-plan, so the item's own `hazard_extras` (the signal the
 *    engine falls back on) decides first, and the requirement class decides for any item whose
 *    quantity rule is a thermal line.
 * 3. For hazards whose main consequence is a readiness checklist (a medical emergency, a fire, a
 *    break-in, being stranded, having to leave), life-saving items come first: the catalogue's
 *    `life_safety` flag plus `LIFE_SAVING` below.
 * 4. Then made-for items, then items whose main consequence is the hazard's, then any overlap;
 *    free and cheap first on ties; anything already done goes last.
 */
import type { Catalogue, HazardId, PlanItem, PlanOutput } from '../engine/types';
import { allPlanItems, catalogueItem, requirementsFor } from './lookup';

/**
 * Equipment that saves a life in the moment which the catalogue does not flag `life_safety` (the
 * flag drives purchase order, and these three are not first purchases for every home). Keep in
 * step with `content/items`; a catalogue flag for "life-saving in use" would replace this list.
 */
export const LIFE_SAVING: ReadonlySet<string> = new Set(['med_first_aid_kit', 'med_bleeding_control_kit', 'fire_extinguisher']);

/** The heat and cold parts of the thermal bucket, as rr-plan splits them (HEAT and COLD in crates/rr-plan/src/coverage.rs). */
const THERMAL_PART: Partial<Record<HazardId, 'thermal_heat' | 'thermal_cold'>> = {
  heat_wave: 'thermal_heat',
  cold_wave: 'thermal_cold',
  winter_weather: 'thermal_cold',
  ice_storm: 'thermal_cold',
};

export function helpsFor(output: PlanOutput, cat: Catalogue | null, hazardId: string): PlanItem[] {
  const hazard = output.register.find((h) => h.id === hazardId);
  if (!hazard) return [];
  const main = hazard.buckets[0];
  const readinessHazard = main !== undefined && cat?.buckets.find((b) => b.id === main)?.kind === 'readiness';
  const part = THERMAL_PART[hazard.id];

  const extras = (i: PlanItem): readonly string[] => catalogueItem(cat, i.item_id)?.hazard_extras ?? [];
  const madeFor = (i: PlanItem) => extras(i).includes(hazard.id);
  const madeForOthers = (i: PlanItem) => extras(i).length > 0 && !madeFor(i);
  const otherPart = (i: PlanItem) => {
    if (!part) return false;
    const classes = new Set(requirementsFor(cat, output, i.item_id).map((r) => r.item_class).filter((c) => c === 'thermal_heat' || c === 'thermal_cold'));
    return classes.size > 0 && !classes.has(part);
  };
  const lifeSaving = (i: PlanItem) => !!catalogueItem(cat, i.item_id)?.life_safety || LIFE_SAVING.has(i.item_id);
  const score = (i: PlanItem) => {
    const made = madeFor(i) ? 4 : 0;
    const primary = i.buckets[0] === main ? 3 : i.buckets[0] && hazard.buckets.includes(i.buckets[0]) ? 1 : 0;
    return made + primary + i.buckets.filter((b) => hazard.buckets.includes(b)).length * 0.5;
  };

  const seen = new Set<string>();
  return allPlanItems(output)
    .map((x) => x.item)
    .filter((i) => i.kind !== 'reserve')
    .filter((i) => (i.hazards.includes(hazard.id) || madeFor(i)) && score(i) >= 1)
    .filter((i) => !madeForOthers(i) && !otherPart(i))
    .filter((i) => (seen.has(i.item_id) ? false : (seen.add(i.item_id), true)))
    .sort(
      (a, b) =>
        Number(!!a.done) - Number(!!b.done) ||
        (readinessHazard ? Number(lifeSaving(b)) - Number(lifeSaving(a)) : 0) ||
        score(b) - score(a) ||
        a.est_cost_usd - b.est_cost_usd,
    );
}
