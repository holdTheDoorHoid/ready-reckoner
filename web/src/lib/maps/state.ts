/**
 * The household's map choices, as the saved plan keeps them (DESIGN-DELTA-v3 §9.5): the pins, the
 * two ways out, which layers were chosen, and the day the maps were last fetched. Web-only: it lives
 * in `SavedPlan.maps`, outside `input`, and the engine never sees it. The map images themselves are
 * not here (they live in the IndexedDB database `rr-maps`, `store.ts`) and are never in the saved
 * file; a plan imported with pins but no images shows "Maps need refreshing".
 *
 * `persistence.ts` (web-interview3) imports `MapsState` for `SavedPlan.maps` and `checkMapsState`
 * to read it back from storage or a file. The home pin is as sensitive as the street address:
 * `mapsHoldLocation` says whether a saved plan holds one (for DESIGN-DELTA-v3 §7's passphrase rule).
 *
 * Nothing here ever leaves the browser.
 */
import type { IsoDate, LatLon } from '../../engine/types';

/** The pins the household can place, in the order the pin map offers them. */
export const PIN_IDS = ['home', 'meeting_near', 'meeting_far', 'where_go'] as const;
export type PinId = (typeof PIN_IDS)[number];

/** The optional layers, in the order the consent screen lists them. */
export const LAYER_IDS = ['base', 'places', 'flood', 'surge', 'wildfire'] as const;
export type LayerId = (typeof LAYER_IDS)[number];

/**
 * Which layers the household chose on its last "Fetch maps". §9.5 names `places`, `flood`, `surge`
 * and `wildfire`; `base` (the street map itself) is optional and absent means "on", so a plan
 * written to the §9.5 shape reads the same.
 */
export interface MapLayers {
  /** The street map (OpenStreetMap tiles, the Census map as the fallback). Absent: on. */
  base?: boolean;
  places: boolean;
  flood: boolean;
  surge: boolean;
  wildfire: boolean;
}

/** `SavedPlan.maps` (DESIGN-DELTA-v3 §9.5). */
export interface MapsState {
  home?: LatLon;
  meeting_near?: LatLon;
  meeting_far?: LatLon;
  where_go?: LatLon;
  /** The ways out as the household drew them (at most `MAX_ROUTES`, each at most `MAX_ROUTE_POINTS` points). */
  routes: LatLon[][];
  layers: MapLayers;
  /** The day the images now in the maps store were fetched (the viewer's local date). */
  fetched_on?: IsoDate;
}

/** Two ways out (the family plan's "two ways out"). */
export const MAX_ROUTES = 2;
/** Points per drawn route; a click adds one. */
export const MAX_ROUTE_POINTS = 60;
/** Decimal places kept for a pin: five is about a metre, finer than anyone can place a pin. */
const PLACES = 5;

/** A new, empty state: no pins, no routes, the default layers (base map and places on). */
export function emptyMapsState(): MapsState {
  return { routes: [], layers: { base: true, places: true, flood: false, surge: false, wildfire: false } };
}

const isObject = (x: unknown): x is Record<string, unknown> => typeof x === 'object' && x !== null && !Array.isArray(x);
const isDate = (x: unknown): x is IsoDate => typeof x === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(x);

function round(v: number): number {
  const k = 10 ** PLACES;
  return Math.round(v * k) / k;
}

/** A point on the map, rounded to about a metre, or undefined when it is not a real latitude and longitude. */
export function checkLatLon(x: unknown): LatLon | undefined {
  if (!isObject(x)) return undefined;
  const { lat, lon } = x;
  if (typeof lat !== 'number' || typeof lon !== 'number' || !Number.isFinite(lat) || !Number.isFinite(lon)) return undefined;
  if (lat < -90 || lat > 90 || lon < -180 || lon > 180) return undefined;
  return { lat: round(lat), lon: round(lon) };
}

/**
 * Read `SavedPlan.maps` back from storage or an imported file: undefined when it is missing or not
 * an object; otherwise a clean `MapsState` with bad points, extra routes and extra points dropped
 * and each layer choice a boolean (unknown keys are not kept).
 */
export function checkMapsState(x: unknown): MapsState | undefined {
  if (!isObject(x)) return undefined;
  const out = emptyMapsState();
  for (const id of PIN_IDS) {
    const p = checkLatLon(x[id]);
    if (p) out[id] = p;
  }
  if (Array.isArray(x.routes)) {
    out.routes = x.routes
      .filter(Array.isArray)
      .map((r) => (r as unknown[]).map(checkLatLon).filter((p): p is LatLon => !!p).slice(0, MAX_ROUTE_POINTS))
      .filter((r) => r.length >= 2)
      .slice(0, MAX_ROUTES);
  }
  const layers = isObject(x.layers) ? x.layers : {};
  out.layers = {
    base: layers.base !== false,
    places: layers.places !== false,
    flood: layers.flood === true,
    surge: layers.surge === true,
    wildfire: layers.wildfire === true,
  };
  if (isDate(x.fetched_on)) out.fetched_on = x.fetched_on;
  return out;
}

/** Whether any pin or route has been placed. */
export function hasPins(maps: MapsState | undefined): boolean {
  return !!maps && (PIN_IDS.some((id) => maps[id] !== undefined) || maps.routes.length > 0);
}

/**
 * Whether the saved plan's map state holds where the household lives (the home pin, or a route,
 * which usually starts there). It is as sensitive as the street address, so a saved file holding
 * it should be offered the passphrase (DESIGN-DELTA-v3 §7).
 */
export function mapsHoldLocation(maps: MapsState | undefined): boolean {
  return !!maps && (maps.home !== undefined || maps.routes.length > 0);
}
