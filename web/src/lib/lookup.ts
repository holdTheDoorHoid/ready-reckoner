/** Plain names and catalogue entries by id, from `catalogue()`; falls back to the id itself. */
import type { BucketId, Catalogue, HazardId, Item, PlanItem, PlanOutput, RequirementLine, TierId } from '../engine/types';

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

/**
 * The requirement lines behind an item's quantity: the lines worked out by the item's quantity
 * rule (`Item.quantity_rule` = `RequirementLine.rule`). Empty for items without one (free actions).
 */
export function requirementsFor(cat: Catalogue | null, output: PlanOutput | undefined, itemId: string): RequirementLine[] {
  const rule = catalogueItem(cat, itemId)?.quantity_rule;
  if (!rule || !output) return [];
  return output.requirements.filter((r) => r.rule === rule);
}

/**
 * Every source behind the numbers shown for a plan item: the quantity (its requirement lines)
 * first, then the item itself (what to look for, the price band).
 */
export function itemSourceIds(cat: Catalogue | null, output: PlanOutput | undefined, itemId: string): string[] {
  const quantity = requirementsFor(cat, output, itemId).flatMap((r) => r.citations);
  const item = catalogueItem(cat, itemId)?.citations ?? [];
  return [...new Set([...quantity, ...item])];
}

/**
 * Plan items with a stable key each. A month can hold the same item twice (the last deposit toward
 * it and the purchase itself), so the key names the kind too, and any repeat gets a number: a
 * keyed list must never see the same key twice.
 */
export function keyedItems<T extends PlanItem>(items: readonly T[]): { key: string; item: T }[] {
  const seen = new Map<string, number>();
  return items.map((item) => {
    const base = `${item.kind}:${item.item_id}:${item.tier}`;
    const n = seen.get(base) ?? 0;
    seen.set(base, n + 1);
    return { key: n === 0 ? base : `${base}#${n}`, item };
  });
}
