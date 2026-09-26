/** Plain names and catalogue entries by id, from `catalogue()`; falls back to the id itself. */
import type { BucketId, Catalogue, HazardId, Item, PlanItem, PlanOutput, TierId } from '../engine/types';

export function bucketName(cat: Catalogue | null, id: BucketId | string): string {
  return cat?.buckets.find((b) => b.id === id)?.name ?? id;
}

export function hazardName(cat: Catalogue | null, id: HazardId | string): string {
  return cat?.hazards.find((h) => h.id === id)?.name ?? id;
}

export function tierName(cat: Catalogue | null, id: TierId): string {
  return cat?.tiers.find((t) => t.id === id)?.name ?? id;
}

export function catalogueItem(cat: Catalogue | null, id: string): Item | undefined {
  return cat?.items.find((i) => i.id === id);
}

/** Every item in the plan, month by month. */
export function allPlanItems(output: PlanOutput): { item: PlanItem; month: number }[] {
  return output.plan.months.flatMap((m) => m.items.map((item) => ({ item, month: m.index })));
}

/** Lower-case the first letter, for names used inside a sentence. */
export function lowerFirst(s: string): string {
  return s.charAt(0).toLowerCase() + s.slice(1);
}
