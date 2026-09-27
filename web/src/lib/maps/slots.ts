/**
 * The three maps (DESIGN-DELTA-v3 §9.2): where each is centred, at what zoom, and which optional
 * layers are suggested for a household.
 *
 * | Slot            | Centre and scale                                              | Layers |
 * | --------------- | ------------------------------------------------------------- | ------ |
 * | `neighbourhood` | the home pin, the zoom (15 or 16) closest to 1.5 km across    | home, meeting place near home, nearby places, FEMA flood zones |
 * | `area`          | the home pin, zoom 12 when the county fits, else 11           | hospitals, fire stations, police, the meeting place outside the neighbourhood, the county outline, wildfire hazard |
 * | `region`        | home, "where we would go" and the drawn routes, fitted (6–10)  | the two ways out, the destination, main roads from the base map |
 *
 * The region map fits zoom 10 down to 6: §9.2 names 8–10, which reaches about 350 km across, and
 * zoom 6 keeps a destination up to about 1,400 km away on the map instead of off its edge.
 *
 * **A map centred on the home is not centred exactly on it.** Every request a map makes (its
 * tiles, the flood and wildfire boxes, the places box) is centred on the frame, so a frame centred
 * on the home pin would tell each recipient where the home is to the metre. Frame centres are
 * snapped to a grid of `SNAP_PX` world pixels at the map's zoom (about 230 m on the neighbourhood
 * map in Philadelphia, 7.5 km on the area map): every home in the same grid cell asks for the
 * same map, and the home is drawn off-centre by at most half a cell (64 pixels of 800).
 */
import type { AgeBand, HazardId, LatLon, LocationResolved, PlanOutput } from '../../engine/types';
import { chanceWithin } from '../format';
import type { MapsState } from './state';
import { type Frame, fitFrame, metresPerPixel, project, unproject } from './tiles';

export const SLOT_KINDS = ['neighbourhood', 'area', 'region'] as const;
export type MapSlotKind = (typeof SLOT_KINDS)[number];

/**
 * A `map_slot` block from the binder (DESIGN-DELTA-v3 §4.1).
 * awaiting: types3 (`web/src/engine/types.ts` gains `Block.map_slot`; this mirrors it until then).
 */
export interface MapSlotBlock {
  id: string;
  kind: MapSlotKind;
  caption: string;
}

/** Stand-ins for the binder's three slots until the binder lands (awaiting: binder). */
export const MAP_SLOT_FIXTURES: readonly MapSlotBlock[] = [
  { id: 'map-neighbourhood', kind: 'neighbourhood', caption: 'Your neighbourhood' },
  { id: 'map-area', kind: 'area', caption: 'Your city or county' },
  { id: 'map-region', kind: 'region', caption: 'Getting out: your region' },
];

/** Every composed map, in CSS pixels (drawn at twice this for print). About 7 by 4.9 inches on paper. */
export const MAP_WIDTH = 800;
export const MAP_HEIGHT = 560;

/** The neighbourhood map aims at about this width on the ground (§9.2). */
const NEIGHBOURHOOD_METRES = 1500;

/** A location as the maps read it: `zip_centroid` arrives with contract v3 (§3.3). */
export type MapLocation = Pick<LocationResolved, 'county_fips' | 'county_name' | 'state_abbr' | 'state_name' | 'centroid'> & {
  /** awaiting: types3 (`LocationResolved.zip_centroid`). */
  zip_centroid?: LatLon;
  exposure?: LocationResolved['exposure'];
};

/** Where the maps centre when there is no home pin. */
export type HomeBasis = 'pin' | 'zip' | 'county';

/** The home pin, else the middle of the ZIP code, else the middle of the county. */
export function homePoint(maps: MapsState | undefined, location: MapLocation): { at: LatLon; basis: HomeBasis } {
  if (maps?.home) return { at: maps.home, basis: 'pin' };
  if (location.zip_centroid) return { at: location.zip_centroid, basis: 'zip' };
  return { at: location.centroid, basis: 'county' };
}

/** A county's box [west, south, east, north], from the outline pack. */
export type CountyBox = [number, number, number, number];

function fits(frame: Frame, box: CountyBox): boolean {
  const c = project(frame.center, frame.zoom);
  const nw = project({ lat: box[3], lon: box[0] }, frame.zoom);
  const se = project({ lat: box[1], lon: box[2] }, frame.zoom);
  return nw.x >= c.x - frame.width / 2 && se.x <= c.x + frame.width / 2 && nw.y >= c.y - frame.height / 2 && se.y <= c.y + frame.height / 2;
}

/** Frame centres snap to this many world pixels, so no request pinpoints the home. */
export const SNAP_PX = 128;

/** The centre of the grid cell a point falls in, at a zoom. */
export function snapCenter(p: LatLon, zoom: number, grid = SNAP_PX): LatLon {
  const w = project(p, zoom);
  return unproject({ x: (Math.floor(w.x / grid) + 0.5) * grid, y: (Math.floor(w.y / grid) + 0.5) * grid }, zoom);
}

/** The frame of one map. */
export function slotFrame(kind: MapSlotKind, maps: MapsState | undefined, location: MapLocation, countyBox?: CountyBox | null): Frame {
  const home = homePoint(maps, location).at;
  const size = { width: MAP_WIDTH, height: MAP_HEIGHT };
  if (kind === 'neighbourhood') {
    const zoom = [16, 15].reduce((best, z) =>
      Math.abs(metresPerPixel(home.lat, z) * MAP_WIDTH - NEIGHBOURHOOD_METRES) < Math.abs(metresPerPixel(home.lat, best) * MAP_WIDTH - NEIGHBOURHOOD_METRES) ? z : best,
    );
    return { center: snapCenter(home, zoom), zoom, ...size };
  }
  if (kind === 'area') {
    const at12: Frame = { center: snapCenter(home, 12), zoom: 12, ...size };
    return countyBox && fits(at12, countyBox) ? at12 : { center: snapCenter(home, 11), zoom: 11, ...size };
  }
  const points: LatLon[] = [home];
  if (maps?.where_go) points.push(maps.where_go);
  for (const route of maps?.routes ?? []) points.push(...route);
  if (points.length === 1) return { center: snapCenter(home, 9), zoom: 9, ...size };
  const fitted = fitFrame(points, MAP_WIDTH, MAP_HEIGHT, 56 + SNAP_PX / 2, 6, 10);
  return { ...fitted, center: snapCenter(fitted.center, fitted.zoom) };
}

/** The middle of a frame, moved by whole pixels (for tests and the pin map). */
export function shiftFrame(frame: Frame, dx: number, dy: number): Frame {
  const c = project(frame.center, frame.zoom);
  return { ...frame, center: unproject({ x: c.x + dx, y: c.y + dy }, frame.zoom) };
}

// ---------------------------------------------------------------------------------------------
// Which optional layers are suggested (§9.2)
// ---------------------------------------------------------------------------------------------

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
