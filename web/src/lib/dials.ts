/**
 * The dials' input model for the v2 settings (docs/ENGINE-API.md "Dials"): which rare families
 * the rare allowance may buy for, bare-minimum mode and the long-horizon section.
 *
 * Rare families. `rare_opt_in` lists family ids (a family's id is the id of the rare hazard that
 * heads it, `RARE_HAZARD_IDS`) or `["all"]`. The v1 switch `rare_catastrophic_opt_in` still
 * parses and means `["all"]`. `rareFamilies` reads both exactly as the engine does
 * (`Dials::rare_families` in rr-types); every change writes the list form and turns the v1
 * switch off, so a saved plan has one source of truth from then on. Unknown ids are dropped when
 * the list is written (the engine would reject them as `unknown_id`).
 *
 * The Risks screen's settings and the rare box (web-risks) both change the allowance through
 * these functions, so the two can never disagree.
 */
import type { Dials, RareHazardId } from '../engine/types';
import { RARE_HAZARD_IDS } from '../engine/types';

/** The `rare_opt_in` entry that means every family, including any added later. */
export const RARE_ALL = 'all';

export function isRareFamily(id: string): id is RareHazardId {
  return (RARE_HAZARD_IDS as readonly string[]).includes(id);
}

/** True when the allowance covers every family: the v1 switch, or `"all"` in the list. */
export function allowsEveryRareFamily(dials: Dials): boolean {
  return dials.rare_catastrophic_opt_in === true || (dials.rare_opt_in ?? []).includes(RARE_ALL);
}

/** The families the rare allowance may buy for, in `RARE_HAZARD_IDS` order, repeats collapsed. */
export function rareFamilies(dials: Dials): RareHazardId[] {
  const list = dials.rare_opt_in ?? [];
  const all = allowsEveryRareFamily(dials);
  return RARE_HAZARD_IDS.filter((f) => all || list.includes(f));
}

/** True when the allowance may buy for this family. */
export function allowsRare(dials: Dials, family: string): boolean {
  return isRareFamily(family) && rareFamilies(dials).includes(family);
}

function write(dials: Dials, list: string[]): void {
  dials.rare_catastrophic_opt_in = false;
  dials.rare_opt_in = list;
}

/** Turn the allowance on or off for one family. */
export function setRareFamily(dials: Dials, family: RareHazardId, on: boolean): void {
  const chosen = new Set<string>(rareFamilies(dials));
  if (on) chosen.add(family);
  else chosen.delete(family);
  write(
    dials,
    RARE_HAZARD_IDS.filter((f) => chosen.has(f)),
  );
}

/** Every family (`["all"]`, which also covers families added later), or none. */
export function setEveryRareFamily(dials: Dials, on: boolean): void {
  write(dials, on ? [RARE_ALL] : []);
}

/** "2 of 9 families", "all of them", "none": for the settings summary. */
export function rareSummary(dials: Dials): string {
  if (allowsEveryRareFamily(dials)) return 'all of them';
  const n = rareFamilies(dials).length;
  if (n === 0) return 'none';
  return `${n} of ${RARE_HAZARD_IDS.length}`;
}

/** Bare-minimum mode as the household set it (the engine may also switch it on for a very long plan). */
export function minimumKit(dials: Dials): boolean {
  return dials.minimum_kit === true;
}

export function setMinimumKit(dials: Dials, on: boolean): void {
  dials.minimum_kit = on;
}

/** Show the long-horizon section even when no target passes 30 days. */
export function longHorizon(dials: Dials): boolean {
  return dials.long_horizon === true;
}

export function setLongHorizon(dials: Dials, on: boolean): void {
  dials.long_horizon = on;
}
