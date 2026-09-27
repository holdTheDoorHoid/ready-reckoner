/**
 * The composer (DESIGN-DELTA-v3 §9.2, §9.4): for one press of "Fetch maps", works out the three
 * frames, fetches what the household agreed to (the street map's tiles, the places, the flood
 * and wildfire overlays), draws each map on a canvas and returns it as a JPEG with its legend
 * rows, its pattern keys, a status line for every layer that is missing and the credit lines.
 *
 * Limits, all from `sources.ts`: tiles only inside the three frames (at most 20 a map, 60 a press;
 * never more than `MAX_TILES_PER_PRESS`), no retries, and no more tile requests at all once the
 * tile server refuses one; one Overpass query, the second instance only if the first fails; one
 * flood and one wildfire export. A layer that fails is left out and its map says so ("Flood zones
 * could not be fetched on October 1, 2026"); nothing that failed blocks the rest.
 *
 * The environment (fetch, canvases, image decoding) is passed in, so tests run it with stubs.
 */
import type { IsoDate, LatLon } from '../../engine/types';
import { classCounts, classifyRedBlue, censusExportUrl, floodExportUrl, maskOf, tileUrl, wildfireExportUrl } from './overlays';
import { drawCredit, drawLine, drawMarker, drawNorthArrow, drawPattern, drawScaleBars, INK, layoutMarkers, PATTERN_SIZE, type PatternId, toneBase } from './draw';
import { AREA_QUOTAS, KIND_LABELS, NEIGHBOURHOOD_QUOTAS, overpassQuery, parseOverpass, type Place, pickPlaces } from './overpass';
import { type HomeBasis, homePoint, type MapLocation, type MapSlotKind, SLOT_KINDS, slotFrame, type SuggestedLayers } from './slots';
import { CENSUS_BASE, FLOOD, MAX_TILES_PER_PRESS, OSM_TILES, OVERPASS, SURGE_NOTE, WILDFIRE } from './sources';
import type { MapLayers, MapsState } from './state';

/** What one press of "Fetch maps" asks for: the saved plan's layers, and whether to fetch the street map. */
export type LayerChoice = MapLayers & { base: boolean };
import { type Frame, frameBox, inFrame, scaleBar, TILE_SIZE, type TilePlacement, tilesFor, toCanvas } from './tiles';

// ---------------------------------------------------------------------------------------------
// What comes out
// ---------------------------------------------------------------------------------------------

/** One row of a map's legend table. */
export interface LegendRow {
  /** What the marker shows: "H", "M1", "M2", "D" for the household's own points, "1", "2"… for places. */
  mark: string;
  name: string;
  kind: string;
  address?: string;
  phone?: string;
  /** The household's own point (a square marker). */
  own?: boolean;
  /** Listed, but beyond the edge of this map. */
  offMap?: boolean;
}

/** A pattern or line the legend explains. */
export interface LegendKey {
  pattern: PatternId | 'route-1' | 'route-2' | 'county';
  text: string;
}

export type LayerName = 'base' | 'places' | 'flood' | 'surge' | 'wildfire';

/** A layer that is not on the map as asked, and why, in words for the legend. */
export interface LayerStatus {
  layer: LayerName;
  state: 'fallback' | 'failed' | 'none' | 'off' | 'unavailable';
  text: string;
}

export interface ComposedMap {
  slot: MapSlotKind;
  /** `data:image/jpeg;base64,…` */
  image: string;
  /** CSS pixels (the image itself is `pixelRatio` times larger). */
  width: number;
  height: number;
  /** Bytes of JPEG. */
  bytes: number;
  zoom: number;
  legend: LegendRow[];
  keys: LegendKey[];
  statuses: LayerStatus[];
  /** Short notes printed under the legend (an approximate home, hospital data). */
  notes: string[];
  /** Printed beside the map, one line per source. */
  credits: string[];
  /** "The bars show 500 ft and 200 m." */
  scale: string;
  fetched_on: IsoDate;
}

/** Requests made in one press, by origin. */
export type RequestTally = Record<string, number>;

export interface ComposeResult {
  maps: ComposedMap[];
  requests: RequestTally;
  /** Tiles requested from the tile server. */
  tiles: number;
  /** Requests that brought back something usable. None, with requests made, is a failed press. */
  succeeded: number;
}

// ---------------------------------------------------------------------------------------------
// What goes in
// ---------------------------------------------------------------------------------------------

/** The county outline, as `lib/geo.ts` reads it (rings of [lon, lat]). */
export interface CountyOutline {
  rings: [number, number][][];
  bbox: [number, number, number, number];
}

export interface ComposeInput {
  maps: MapsState;
  location: MapLocation;
  county: CountyOutline | null;
  /** What the household ticked on the consent screen. */
  layers: LayerChoice;
  /** The §9.2 rule (for the surge note, and to say a suggested layer was left off). */
  suggested: SuggestedLayers;
  children: boolean;
  today: IsoDate;
  /** The state's evacuation-zone page, printed in place of the surge layer (awaiting: binder). */
  zoneLink?: { name: string; url: string } | null;
}

/** An image the canvas can draw. */
export interface Decoded {
  width: number;
  height: number;
  close?: () => void;
}

export interface CanvasLike {
  width: number;
  height: number;
  getContext(kind: '2d', options?: { willReadFrequently?: boolean }): CanvasRenderingContext2D | null;
  toDataURL(type: string, quality?: number): string;
}

export interface ComposeEnv {
  fetch: (url: string, init: RequestInit) => Promise<Response>;
  canvas(width: number, height: number): CanvasLike;
  decode(blob: Blob): Promise<Decoded>;
  /** The RGBA pixels of an image drawn at `width`×`height`. */
  readPixels(image: Decoded, width: number, height: number): ArrayLike<number>;
  /** Device pixels per CSS pixel for the images (2 prints crisply). */
  pixelRatio?: number;
  onProgress?(done: number, total: number): void;
  signal?: AbortSignal;
  /** The clock, for the pause after a busy answer (tests pass their own). */
  now?: () => number;
}

/** The browser's own environment. */
export function browserEnv(): ComposeEnv {
  const canvas = (width: number, height: number): CanvasLike => {
    const c = document.createElement('canvas');
    c.width = width;
    c.height = height;
    return c;
  };
  return {
    fetch: (url, init) => fetch(url, init),
    canvas,
    // Exact pixels for the overlays' two colours; a browser that rejects the options decodes plainly.
    decode: (blob) => createImageBitmap(blob, { colorSpaceConversion: 'none', premultiplyAlpha: 'none' }).catch(() => createImageBitmap(blob)),
    readPixels(image, width, height) {
      const ctx = canvas(width, height).getContext('2d', { willReadFrequently: true })!;
      ctx.drawImage(image as unknown as CanvasImageSource, 0, 0, width, height);
      return ctx.getImageData(0, 0, width, height).data;
    },
  };
}

// ---------------------------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------------------------

const longDateFmt = new Intl.DateTimeFormat('en-US', { month: 'long', day: 'numeric', year: 'numeric', timeZone: 'UTC' });

/** "October 1, 2026". */
export function longDate(iso: IsoDate): string {
  return longDateFmt.format(new Date(`${iso}T00:00:00Z`));
}

/** The household's own points on each map, with their marks. */
const OWN_POINTS: Record<MapSlotKind, { id: 'home' | 'meeting_near' | 'meeting_far' | 'where_go'; mark: string; name: string }[]> = {
  neighbourhood: [
    { id: 'home', mark: 'H', name: 'Home' },
    { id: 'meeting_near', mark: 'M1', name: 'Meeting place near home' },
  ],
  area: [
    { id: 'home', mark: 'H', name: 'Home' },
    { id: 'meeting_far', mark: 'M2', name: 'Meeting place outside the neighbourhood' },
  ],
  region: [
    { id: 'home', mark: 'H', name: 'Home' },
    { id: 'where_go', mark: 'D', name: 'Where we would go' },
  ],
};

const HOME_NOTE: Record<Exclude<HomeBasis, 'pin'>, string> = {
  zip: 'No home pin was placed, so this map is centred on the middle of your ZIP code and your home is not marked.',
  county: 'No home pin was placed, so this map is centred on the middle of your county and your home is not marked.',
};

/** The county outline comes from this app's own data (`data/geo/counties.json`). */
export const COUNTY_CREDIT = 'County line: U.S. Census Bureau cartographic boundary file, 2024 (public domain), from this app’s own data';

export const HOSPITAL_NOTE =
  'Hospitals come from OpenStreetMap, which volunteers keep up to date, and a closed hospital can still be listed. Check before you rely on one: call ahead, or use the county hospital list in this binder.';

// ---------------------------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------------------------

const TILE_TIMEOUT_MS = 15_000;
const EXPORT_TIMEOUT_MS = 30_000;

class Fetcher {
  tally: RequestTally = {};
  succeeded = 0;
  constructor(private env: ComposeEnv) {}

  async get(url: string, referrerPolicy: ReferrerPolicy, timeoutMs: number, init: RequestInit = {}): Promise<Response> {
    const origin = new URL(url).origin;
    this.tally[origin] = (this.tally[origin] ?? 0) + 1;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    const outer = this.env.signal;
    const onAbort = () => controller.abort();
    outer?.addEventListener('abort', onAbort);
    try {
      return await this.env.fetch(url, { mode: 'cors', credentials: 'omit', referrerPolicy, cache: 'default', ...init, signal: controller.signal });
    } finally {
      clearTimeout(timer);
      outer?.removeEventListener('abort', onAbort);
    }
  }

  /** An image, or null when it could not be had. */
  async image(url: string, referrerPolicy: ReferrerPolicy, timeoutMs: number): Promise<{ image: Decoded | null; refused: boolean }> {
    try {
      const response = await this.get(url, referrerPolicy, timeoutMs);
      if (!response.ok) return { image: null, refused: [403, 418, 429].includes(response.status) };
      const type = response.headers.get('content-type') ?? '';
      if (!type.startsWith('image/')) return { image: null, refused: false };
      const image = await this.env.decode(await response.blob());
      this.succeeded += 1;
      return { image, refused: false };
    } catch {
      return { image: null, refused: false };
    }
  }
}

/** Run tasks with at most `limit` at once. */
async function pool<T>(items: readonly T[], limit: number, task: (item: T) => Promise<void>): Promise<void> {
  let next = 0;
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (next < items.length) await task(items[next++]!);
  });
  await Promise.all(workers);
}

type Base = { source: 'osm'; tiles: { t: TilePlacement; image: Decoded }[] } | { source: 'census'; image: Decoded } | { source: 'none'; failed?: boolean };

interface Overlay {
  state: 'ok' | 'none' | 'failed';
  classes?: Uint8Array;
  counts?: [number, number, number];
}

async function fetchBases(frames: Record<MapSlotKind, Frame>, fetcher: Fetcher, env: ComposeEnv, progress: () => void): Promise<{ bases: Record<MapSlotKind, Base>; tiles: number }> {
  const plans = SLOT_KINDS.map((slot) => ({ slot, tiles: tilesFor(frames[slot]) }));
  let budget = MAX_TILES_PER_PRESS;
  let refused = false;
  let requested = 0;
  const got = new Map<string, Decoded | null>();
  const all: TilePlacement[] = [];
  for (const plan of plans) {
    for (const t of plan.tiles) {
      const key = `${t.z}/${t.x}/${t.y}`;
      if (got.has(key)) continue;
      got.set(key, null);
      if (budget-- > 0) all.push(t);
    }
  }
  await pool(all, 4, async (t) => {
    if (refused || env.signal?.aborted) return;
    requested += 1;
    const r = await fetcher.image(tileUrl(t), OSM_TILES.referrerPolicy, TILE_TIMEOUT_MS);
    if (r.refused) refused = true;
    got.set(`${t.z}/${t.x}/${t.y}`, r.image);
    progress();
  });
  const bases = {} as Record<MapSlotKind, Base>;
  for (const plan of plans) {
    const images = plan.tiles.map((t) => ({ t, image: got.get(`${t.z}/${t.x}/${t.y}`) ?? null }));
    if (images.every((i) => i.image)) {
      bases[plan.slot] = { source: 'osm', tiles: images as { t: TilePlacement; image: Decoded }[] };
      continue;
    }
    // A missing tile: the whole map comes from the Census export instead (one request).
    const census = await fetcher.image(censusExportUrl(frames[plan.slot]), CENSUS_BASE.referrerPolicy, EXPORT_TIMEOUT_MS);
    progress();
    bases[plan.slot] = census.image ? { source: 'census', image: census.image } : { source: 'none', failed: true };
  }
  return { bases, tiles: requested };
}

/** Overpass servers that answered "busy" (429, 406, 504), and until when they are left alone. */
const overpassBusyUntil = new Map<string, number>();

/** Forget every "busy" answer (tests). */
export function resetOverpassPauses(): void {
  overpassBusyUntil.clear();
}

type PlacesResult = { places: Place[] } | { failed: 'busy' | 'error' };

async function fetchPlaces(frames: Record<MapSlotKind, Frame>, children: boolean, fetcher: Fetcher, now: () => number): Promise<PlacesResult> {
  const area = frames.area;
  const query = overpassQuery(
    { neighbourhood: frameBox(frames.neighbourhood), nearby: frameBox({ ...area, zoom: area.zoom + 1 }), area: frameBox(area) },
    { children, timeoutS: OVERPASS.timeoutS },
  );
  let busy = false;
  for (const url of OVERPASS.urls.slice(0, OVERPASS.maxQueries)) {
    // The main instance asks for a 30-second pause after a 429 or 406: a server that said it was
    // busy is not asked again until then (the next one is tried instead).
    if ((overpassBusyUntil.get(url) ?? 0) > now()) {
      busy = true;
      continue;
    }
    try {
      const response = await fetcher.get(url, OVERPASS.referrerPolicy, OVERPASS.clientTimeoutMs, {
        method: 'POST',
        body: new URLSearchParams({ data: query }),
      });
      if ([429, 406, 504].includes(response.status)) {
        overpassBusyUntil.set(url, now() + OVERPASS.busyPauseMs);
        busy = true;
        continue;
      }
      if (!response.ok) continue;
      const json = (await response.json()) as { elements?: unknown; remark?: string };
      // A timed-out or refused query still answers 200, with a remark instead of the data.
      if (!Array.isArray(json.elements) || /error|timed out/i.test(json.remark ?? '')) continue;
      fetcher.succeeded += 1;
      return { places: parseOverpass(json) };
    } catch {
      // Unreachable or too slow: try the next instance, if the press allows one more query.
    }
  }
  return { failed: busy ? 'busy' : 'error' };
}

async function fetchOverlay(url: string, referrerPolicy: ReferrerPolicy, frame: Frame, fetcher: Fetcher, env: ComposeEnv): Promise<Overlay> {
  const { image } = await fetcher.image(url, referrerPolicy, EXPORT_TIMEOUT_MS);
  if (!image) return { state: 'failed' };
  const classes = classifyRedBlue(env.readPixels(image, frame.width, frame.height));
  image.close?.();
  const counts = classCounts(classes);
  return counts[1] + counts[2] === 0 ? { state: 'none', counts } : { state: 'ok', classes, counts };
}

// ---------------------------------------------------------------------------------------------
// Drawing one map
// ---------------------------------------------------------------------------------------------

const TINT: Record<PatternId, string> = {
  stripes: 'rgba(21,67,125,0.22)',
  dots: 'rgba(21,67,125,0.10)',
  'stripes-back': 'rgba(122,37,8,0.16)',
  cross: 'rgba(122,37,8,0.26)',
};

function drawOverlay(ctx: CanvasRenderingContext2D, classes: Uint8Array, cls: 1 | 2, pattern: PatternId, ink: string, frame: Frame, ratio: number, env: ComposeEnv): void {
  const { width: w, height: h } = frame;
  const mask = env.canvas(w, h);
  const mctx = mask.getContext('2d')!;
  const data = mctx.createImageData(w, h);
  data.data.set(maskOf(classes, cls));
  mctx.putImageData(data, 0, 0);
  const layer = env.canvas(w * ratio, h * ratio);
  const lctx = layer.getContext('2d')!;
  lctx.fillStyle = TINT[pattern];
  lctx.fillRect(0, 0, w * ratio, h * ratio);
  const tile = env.canvas(PATTERN_SIZE * ratio, PATTERN_SIZE * ratio);
  drawPattern(tile.getContext('2d')!, pattern, ink, ratio);
  const fill = lctx.createPattern(tile as unknown as CanvasImageSource, 'repeat');
  if (fill) {
    lctx.fillStyle = fill;
    lctx.fillRect(0, 0, w * ratio, h * ratio);
  }
  lctx.globalCompositeOperation = 'destination-in';
  lctx.drawImage(mask as unknown as CanvasImageSource, 0, 0, w * ratio, h * ratio);
  ctx.drawImage(layer as unknown as CanvasImageSource, 0, 0);
}

interface DrawInput {
  slot: MapSlotKind;
  frame: Frame;
  base: Base;
  places: PlacesResult | null;
  flood: Overlay | null;
  wildfire: Overlay | null;
  input: ComposeInput;
}

function drawMap(d: DrawInput, env: ComposeEnv, ratio: number): ComposedMap {
  const { slot, frame, base, input } = d;
  const { width: w, height: h } = frame;
  const date = longDate(input.today);
  const canvas = env.canvas(w * ratio, h * ratio);
  const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
  const legend: LegendRow[] = [];
  const keys: LegendKey[] = [];
  const statuses: LayerStatus[] = [];
  const notes: string[] = [];
  const credits: string[] = [];

  // 1. The base map, greyed.
  ctx.fillStyle = '#f2f2f2';
  ctx.fillRect(0, 0, w * ratio, h * ratio);
  if (base.source === 'osm') {
    for (const { t, image } of base.tiles) ctx.drawImage(image as unknown as CanvasImageSource, t.left * ratio, t.top * ratio, TILE_SIZE * ratio, TILE_SIZE * ratio);
    credits.push(OSM_TILES.creditLong);
  } else if (base.source === 'census') {
    ctx.drawImage(base.image as unknown as CanvasImageSource, 0, 0, w * ratio, h * ratio);
    credits.push(`${CENSUS_BASE.credit}, retrieved ${date} (public domain)`);
    statuses.push({ layer: 'base', state: 'fallback', text: 'The OpenStreetMap map did not answer, so this map uses the U.S. Census Bureau’s street map.' });
  } else if (base.failed) {
    statuses.push({ layer: 'base', state: 'failed', text: `The street map could not be fetched on ${date}.` });
  } else {
    statuses.push({ layer: 'base', state: 'off', text: 'No street map: you left it off.' });
  }
  if (base.source !== 'none') {
    const pixels = ctx.getImageData(0, 0, w * ratio, h * ratio);
    toneBase(pixels.data);
    ctx.putImageData(pixels, 0, 0);
  }

  // 2. Overlays, hatched.
  if (slot === 'neighbourhood') {
    if (input.layers.flood && d.flood) {
      if (d.flood.state === 'ok' && d.flood.classes) {
        if (d.flood.counts![1] > 0) {
          drawOverlay(ctx, d.flood.classes, 1, 'stripes', INK.flood, frame, ratio, env);
          keys.push({ pattern: 'stripes', text: 'FEMA high-risk flood zone: about a 1 in 100 chance of flooding each year.' });
        }
        if (d.flood.counts![2] > 0) {
          drawOverlay(ctx, d.flood.classes, 2, 'dots', INK.flood, frame, ratio, env);
          keys.push({ pattern: 'dots', text: 'FEMA moderate flood zone: about a 1 in 500 chance each year.' });
        }
        credits.push(`${FLOOD.credit}, retrieved ${date}`);
      } else if (d.flood.state === 'none') {
        statuses.push({ layer: 'flood', state: 'none', text: 'No FEMA flood zone is mapped in this area.' });
        credits.push(`${FLOOD.credit}, retrieved ${date}`);
      } else {
        statuses.push({ layer: 'flood', state: 'failed', text: `Flood zones could not be fetched on ${date}.` });
      }
    } else if (input.suggested.flood) {
      statuses.push({ layer: 'flood', state: 'off', text: 'Flood zones were left off this map.' });
    }
  }
  if (slot === 'area') {
    if (input.layers.wildfire && d.wildfire) {
      if (d.wildfire.state === 'ok' && d.wildfire.classes) {
        if (d.wildfire.counts![1] > 0) {
          drawOverlay(ctx, d.wildfire.classes, 1, 'stripes-back', INK.fire, frame, ratio, env);
          keys.push({ pattern: 'stripes-back', text: 'High wildfire hazard (U.S. Forest Service).' });
        }
        if (d.wildfire.counts![2] > 0) {
          drawOverlay(ctx, d.wildfire.classes, 2, 'cross', INK.fire, frame, ratio, env);
          keys.push({ pattern: 'cross', text: 'Very high wildfire hazard (U.S. Forest Service).' });
        }
        credits.push(`${WILDFIRE.credit}, retrieved ${date}`);
      } else if (d.wildfire.state === 'none') {
        statuses.push({ layer: 'wildfire', state: 'none', text: 'No high wildfire hazard is mapped in this area.' });
        credits.push(`${WILDFIRE.credit}, retrieved ${date}`);
      } else {
        statuses.push({ layer: 'wildfire', state: 'failed', text: `Wildfire hazard could not be fetched on ${date}.` });
      }
    } else if (input.suggested.wildfire) {
      statuses.push({ layer: 'wildfire', state: 'off', text: 'Wildfire hazard was left off this map.' });
    }
  }
  if (input.suggested.surge && slot !== 'region') {
    const where = input.zoneLink ? ` Find your evacuation zone: ${input.zoneLink.name}, ${input.zoneLink.url}` : ' Your state or county emergency office has the evacuation-zone map.';
    statuses.push({ layer: 'surge', state: 'unavailable', text: `${SURGE_NOTE}${where}` });
  }

  // From here on, CSS pixels.
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);

  // 3. The county outline and the drawn routes.
  if (slot !== 'neighbourhood' && input.county) {
    for (const ring of input.county.rings) {
      drawLine(
        ctx,
        ring.map(([lon, lat]) => toCanvas(frame, { lat, lon })),
        { width: 2.2, dash: [9, 5], ink: INK.county, casing: 3 },
      );
    }
    keys.push({ pattern: 'county', text: `County line: ${input.location.county_name}, ${input.location.state_abbr}.` });
    credits.push(COUNTY_CREDIT);
  }
  if (slot !== 'neighbourhood') {
    input.maps.routes.forEach((route, i) => {
      const pts = route.map((p) => toCanvas(frame, p));
      if (!route.some((p) => inFrame(frame, p))) return;
      drawLine(ctx, pts, { width: 4, dash: i === 0 ? [] : [11, 7], ink: INK.route, casing: 4 });
      keys.push({ pattern: i === 0 ? 'route-1' : 'route-2', text: i === 0 ? 'Way out 1, as you drew it.' : 'Way out 2, as you drew it.' });
    });
  }

  // 4. Markers: the household's own points first (fixed), then the places, numbered.
  const marks: { at: LatLon; label: string; own: boolean }[] = [];
  const home = homePoint(input.maps, input.location);
  for (const own of OWN_POINTS[slot]) {
    const at = input.maps[own.id];
    if (!at) continue;
    const onMap = inFrame(frame, at, 4);
    legend.push({ mark: own.mark, name: own.name, kind: 'Your plan', own: true, ...(onMap ? {} : { offMap: true }) });
    if (onMap) marks.push({ at, label: own.mark, own: true });
  }
  if (home.basis !== 'pin' && slot !== 'region') notes.push(HOME_NOTE[home.basis]);
  if (slot !== 'region') {
    if (!input.layers.places || d.places === null) statuses.push({ layer: 'places', state: 'off', text: 'Nearby places were left off.' });
    else if ('failed' in d.places) {
      const why = d.places.failed === 'busy' ? ': the places service was busy. Refresh the maps later to add them' : '';
      statuses.push({ layer: 'places', state: 'failed', text: `Nearby places could not be fetched on ${date}${why}.` });
    } else {
      const picked = pickPlaces(d.places.places, slot === 'neighbourhood' ? NEIGHBOURHOOD_QUOTAS : AREA_QUOTAS, frame, home.at, { children: input.children });
      picked.forEach((p, i) => {
        const row: LegendRow = { mark: String(i + 1), name: p.name ?? `${KIND_LABELS[p.kind]} (no name listed)`, kind: KIND_LABELS[p.kind] };
        if (p.address) row.address = p.address;
        if (p.phone) row.phone = p.phone;
        legend.push(row);
        marks.push({ at: p.at, label: String(i + 1), own: false });
      });
      if (picked.length === 0) statuses.push({ layer: 'places', state: 'none', text: 'No places of these kinds are mapped here in OpenStreetMap.' });
      if (picked.some((p) => p.kind === 'hospital_er' || p.kind === 'hospital')) notes.push(HOSPITAL_NOTE);
      credits.push(`${OVERPASS.credit}, retrieved ${date}`);
    }
  }
  const placed = layoutMarkers(
    marks.map((m) => ({ ...toCanvas(frame, m.at), fixed: m.own })),
    11,
    w,
    h,
  );
  // Places first, then the household's own points on top.
  marks.forEach((m, i) => !m.own && drawMarker(ctx, placed[i]!, m.label, false));
  marks.forEach((m, i) => m.own && drawMarker(ctx, placed[i]!, m.label, true));

  // 5. Scale, north, credit.
  const bars = [scaleBar(frame, 150, 'us'), scaleBar(frame, 150, 'metric')];
  drawScaleBars(ctx, bars, h);
  drawNorthArrow(ctx, w);
  if (base.source !== 'none') drawCredit(ctx, base.source === 'osm' ? OSM_TILES.credit : 'U.S. Census Bureau', w, h);

  const image = canvas.toDataURL('image/jpeg', 0.8);
  const comma = image.indexOf(',');
  return {
    slot,
    image,
    width: w,
    height: h,
    bytes: Math.floor(((image.length - comma - 1) * 3) / 4),
    zoom: frame.zoom,
    legend,
    keys,
    statuses,
    notes,
    credits,
    scale: `The bars show ${bars[0]!.label} and ${bars[1]!.label}.`,
    fetched_on: input.today,
  };
}

// ---------------------------------------------------------------------------------------------
// One press
// ---------------------------------------------------------------------------------------------

/** How many requests a press can make at most, for the progress bar. */
export function plannedRequests(input: ComposeInput): number {
  const frames = framesFor(input);
  const tiles = input.layers.base ? new Set(SLOT_KINDS.flatMap((s) => tilesFor(frames[s]).map((t) => `${t.z}/${t.x}/${t.y}`))).size : 0;
  return tiles + (input.layers.places ? 1 : 0) + (input.layers.flood ? 1 : 0) + (input.layers.wildfire ? 1 : 0);
}

/** The three frames for this household. */
export function framesFor(input: Pick<ComposeInput, 'maps' | 'location' | 'county'>): Record<MapSlotKind, Frame> {
  const box = input.county?.bbox ?? null;
  return {
    neighbourhood: slotFrame('neighbourhood', input.maps, input.location, box),
    area: slotFrame('area', input.maps, input.location, box),
    region: slotFrame('region', input.maps, input.location, box),
  };
}

/** Fetch and draw the three maps. Never throws for a failed source: its map says so instead. */
export async function composeMaps(input: ComposeInput, env: ComposeEnv): Promise<ComposeResult> {
  const ratio = env.pixelRatio ?? 2;
  const frames = framesFor(input);
  const fetcher = new Fetcher(env);
  const total = plannedRequests(input);
  let done = 0;
  // A Census fallback adds requests the plan did not count: the bar stops at full, never past it.
  const progress = () => env.onProgress?.(Math.min(++done, total), total);

  const [base, places, flood, wildfire] = await Promise.all([
    input.layers.base
      ? fetchBases(frames, fetcher, env, progress)
      : Promise.resolve({ bases: { neighbourhood: { source: 'none' }, area: { source: 'none' }, region: { source: 'none' } } as Record<MapSlotKind, Base>, tiles: 0 }),
    input.layers.places ? fetchPlaces(frames, input.children, fetcher, env.now ?? Date.now).finally(progress) : Promise.resolve(null),
    input.layers.flood ? fetchOverlay(floodExportUrl(frames.neighbourhood), FLOOD.referrerPolicy, frames.neighbourhood, fetcher, env).finally(progress) : Promise.resolve(null),
    input.layers.wildfire ? fetchOverlay(wildfireExportUrl(frames.area), WILDFIRE.referrerPolicy, frames.area, fetcher, env).finally(progress) : Promise.resolve(null),
  ]);

  const maps = SLOT_KINDS.map((slot) =>
    drawMap({ slot, frame: frames[slot], base: base.bases[slot]!, places, flood, wildfire, input }, env, ratio),
  );
  for (const b of Object.values(base.bases)) {
    if (b.source === 'osm') for (const { image } of b.tiles) image.close?.();
    else if (b.source === 'census') b.image.close?.();
  }
  return { maps, requests: fetcher.tally, tiles: base.tiles, succeeded: fetcher.succeeded };
}
