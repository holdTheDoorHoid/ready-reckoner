/**
 * County outlines for the map thumbnail, drawn as plain SVG: no map library, no tiles, nothing
 * fetched from anywhere but this site (`data/geo/counties.json`, Census 2024 cartographic
 * boundaries at 1:20m with 3-decimal coordinates, loaded only when a map is shown).
 *
 * A county is shown inside its state, so the shape is recognisable; a ZIP code that spans
 * several counties is shown as the area around them, each candidate numbered. The projection is
 * a plain longitude-latitude grid with longitudes shrunk by the cosine of the middle latitude,
 * which is close enough for a state-sized thumbnail. Alaska's Aleutians cross the 180th
 * meridian, so eastern longitudes there are moved west by 360°.
 */
import { dataUrl, MAP_FILE } from '../engine/data-files';

type Ring = [number, number][];

export interface CountyShape {
  fips: string;
  name: string;
  state: string;
  /** Interior point (lat, lon) from the pack. */
  lat: number;
  lon: number;
  /** Every ring of every polygon, as [lon, lat] points. */
  rings: Ring[];
  /** [west, south, east, north]. */
  bbox: [number, number, number, number];
}

export interface CountyShapes {
  byFips: Map<string, CountyShape>;
  byState: Map<string, CountyShape[]>;
}

interface Feature {
  id?: string;
  properties?: { name?: string; state?: string; lat?: number; lon?: number };
  geometry?: { type: string; coordinates: unknown };
}

function ringsOf(geometry: Feature['geometry']): Ring[] {
  if (!geometry) return [];
  if (geometry.type === 'Polygon') return geometry.coordinates as Ring[];
  if (geometry.type === 'MultiPolygon') return (geometry.coordinates as Ring[][]).flat();
  return [];
}

/** Index a GeoJSON FeatureCollection of counties (the `geo/counties.json` pack) by FIPS and by state. */
export function indexShapes(json: unknown): CountyShapes {
  const features = ((json as { features?: Feature[] } | null)?.features ?? []) as Feature[];
  const byFips = new Map<string, CountyShape>();
  const byState = new Map<string, CountyShape[]>();
  for (const f of features) {
    const fips = String(f.id ?? '');
    const state = f.properties?.state ?? '';
    if (!/^\d{5}$/.test(fips)) continue;
    const wrap = state === 'AK';
    const rings = ringsOf(f.geometry).map((ring) => ring.map(([lon, lat]) => [wrap && lon > 0 ? lon - 360 : lon, lat] as [number, number]));
    if (rings.length === 0) continue;
    let [w, s, e, n] = [Infinity, Infinity, -Infinity, -Infinity];
    for (const ring of rings) {
      for (const [lon, lat] of ring) {
        if (lon < w) w = lon;
        if (lon > e) e = lon;
        if (lat < s) s = lat;
        if (lat > n) n = lat;
      }
    }
    const lon = f.properties?.lon ?? (w + e) / 2;
    const shape: CountyShape = {
      fips,
      name: f.properties?.name ?? fips,
      state,
      lat: f.properties?.lat ?? (s + n) / 2,
      lon: wrap && lon > 0 ? lon - 360 : lon,
      rings,
      bbox: [w, s, e, n],
    };
    byFips.set(fips, shape);
    const list = byState.get(state) ?? [];
    list.push(shape);
    byState.set(state, list);
  }
  return { byFips, byState };
}

/** Fetch and index the outlines directly (the mock engine has no data loader). Null when the site has none. */
export async function fetchCountyShapes(base: string): Promise<CountyShapes | null> {
  const response = await fetch(dataUrl(base, MAP_FILE));
  const type = response.headers.get('content-type') ?? '';
  if (response.status === 404 || type.includes('text/html')) return null;
  if (!response.ok) throw new Error(`The map did not download (HTTP ${response.status}).`);
  return indexShapes(await response.json());
}

// ---------------------------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------------------------

/** The drawing box every map uses (viewBox units). */
export const MAP_WIDTH = 300;
export const MAP_HEIGHT = 200;
const MARGIN = 8;
/** A county smaller than this (viewBox units) also gets a ring, so it can be found. */
const SMALL = 9;

export type Role = 'focus' | 'candidate' | 'context';

export interface MapPath {
  fips: string;
  name: string;
  role: Role;
  d: string;
}

export interface MapMarker {
  fips: string;
  x: number;
  y: number;
  /** A number shown in the marker (candidates), or none for a plain ring. */
  label?: string;
}

export interface MapView {
  paths: MapPath[];
  markers: MapMarker[];
}

function union(boxes: [number, number, number, number][]): [number, number, number, number] {
  return boxes.reduce((a, b) => [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(a[2], b[2]), Math.max(a[3], b[3])]);
}

function intersects(a: [number, number, number, number], b: [number, number, number, number]): boolean {
  return a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3];
}

/** A projection fitting `frame` ([w, s, e, n]) into the drawing box, centred. */
function projector(frame: [number, number, number, number]): (lon: number, lat: number) => [number, number] {
  const [w, s, e, n] = frame;
  const k = Math.cos((((s + n) / 2) * Math.PI) / 180);
  const fw = Math.max((e - w) * k, 1e-6);
  const fh = Math.max(n - s, 1e-6);
  const scale = Math.min((MAP_WIDTH - 2 * MARGIN) / fw, (MAP_HEIGHT - 2 * MARGIN) / fh);
  const ox = (MAP_WIDTH - fw * scale) / 2;
  const oy = (MAP_HEIGHT - fh * scale) / 2;
  return (lon, lat) => [ox + (lon - w) * k * scale, oy + (n - lat) * scale];
}

function pathOf(shape: CountyShape, project: (lon: number, lat: number) => [number, number]): string {
  const parts: string[] = [];
  for (const ring of shape.rings) {
    let last = '';
    const points: string[] = [];
    for (const [lon, lat] of ring) {
      const [x, y] = project(lon, lat);
      const p = `${x.toFixed(1)},${y.toFixed(1)}`;
      if (p !== last) points.push(p);
      last = p;
    }
    if (points.length >= 3) parts.push(`M${points.join('L')}Z`);
  }
  return parts.join('');
}

function extent(shape: CountyShape, project: (lon: number, lat: number) => [number, number]): number {
  const [x0, y0] = project(shape.bbox[0], shape.bbox[3]);
  const [x1, y1] = project(shape.bbox[2], shape.bbox[1]);
  return Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0));
}

/** One county inside its state: the state's counties outlined, the county marked. Null if unknown. */
export function stateView(shapes: CountyShapes, fips: string): MapView | null {
  const focus = shapes.byFips.get(fips);
  if (!focus) return null;
  const others = shapes.byState.get(focus.state) ?? [focus];
  const project = projector(union(others.map((c) => c.bbox)));
  const paths: MapPath[] = others
    .filter((c) => c.fips !== fips)
    .map((c) => ({ fips: c.fips, name: c.name, role: 'context' as const, d: pathOf(c, project) }));
  paths.push({ fips, name: focus.name, role: 'focus', d: pathOf(focus, project) });
  const markers: MapMarker[] = [];
  if (extent(focus, project) < SMALL) {
    const [x, y] = project(focus.lon, focus.lat);
    markers.push({ fips, x, y });
  }
  return { paths, markers };
}

/**
 * Several counties (the ones a ZIP code spans) and the area around them, each numbered in the
 * order given; `selected` is drawn as the focus. Counties from neighbouring states are included.
 */
export function areaView(shapes: CountyShapes, fipsList: readonly string[], selected?: string): MapView | null {
  const chosen = fipsList.map((f) => shapes.byFips.get(f)).filter((c): c is CountyShape => !!c);
  if (chosen.length === 0) return null;
  const [w, s, e, n] = union(chosen.map((c) => c.bbox));
  const padX = Math.max((e - w) * 0.2, 0.12);
  const padY = Math.max((n - s) * 0.2, 0.1);
  const frame: [number, number, number, number] = [w - padX, s - padY, e + padX, n + padY];
  const project = projector(frame);
  const ids = new Set(chosen.map((c) => c.fips));
  const paths: MapPath[] = [];
  for (const c of shapes.byFips.values()) {
    if (ids.has(c.fips) || !intersects(c.bbox, frame)) continue;
    paths.push({ fips: c.fips, name: c.name, role: 'context', d: pathOf(c, project) });
  }
  // The chosen county last, so its outline sits on top.
  for (const c of [...chosen].sort((a, b) => Number(a.fips === selected) - Number(b.fips === selected))) {
    paths.push({ fips: c.fips, name: c.name, role: c.fips === selected ? 'focus' : 'candidate', d: pathOf(c, project) });
  }
  const markers = chosen.map((c, i) => {
    const [x, y] = project(c.lon, c.lat);
    return { fips: c.fips, x, y, label: String(i + 1) };
  });
  return { paths, markers };
}
