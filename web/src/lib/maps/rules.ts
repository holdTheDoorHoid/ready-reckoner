/**
 * Which optional map layers are suggested for a household (DESIGN-DELTA-v3 §9.2), and whether its
 * maps show schools and child care. Kept apart from the map arithmetic in `slots.ts` so a screen
 * can ask without loading it.
 */
import type { AgeBand, HazardId, PlanOutput } from '../../engine/types';
import { chanceWithin } from '../format';

/** The flood hazards whose chance decides the flood-zone layer. */
export const FLOOD_HAZARDS: readonly HazardId[] = ['riverine_flooding', 'coastal_flooding'];

/** "1 in 100 or more": the matrix's chance over the household's horizon. */
export const SUGGEST_AT = 0.01;

export interface SuggestedLayers {
  flood: boolean;
  surge: boolean;
  wildfire: boolean;
}

/**
 * Flood zones when a flood hazard is in the ranked matrix at 1 in 100 or more over the horizon;
 * storm surge when the home is in a surge area or hurricane is ranked (not drawn: see
 * `SURGE_NOTE`); wildfire hazard when wildfire is ranked at 1 in 100 or more.
 */
export function suggestedLayers(output: Pick<PlanOutput, 'register' | 'location'> | null | undefined, years: number): SuggestedLayers {
  const ranked = (output?.register ?? []).filter((h) => h.display === 'ranked');
  const likely = (ids: readonly HazardId[]) => ranked.some((h) => ids.includes(h.id) && chanceWithin(h.rate_per_year, years) >= SUGGEST_AT);
  const surgeShare = output?.location.exposure?.surge_cat3_share?.value ?? 0;
  return {
    flood: likely(FLOOD_HAZARDS),
    surge: surgeShare > 0 || ranked.some((h) => h.id === 'hurricane'),
    wildfire: likely(['wildfire']),
  };
}

const CHILD_BANDS: readonly AgeBand[] = ['infant', 'toddler', 'child', 'teen'];

/** Schools and child care are shown when the household has children. */
export function hasChildren(people: readonly { age_band: AgeBand }[] | undefined): boolean {
  return (people ?? []).some((p) => CHILD_BANDS.includes(p.age_band));
}
