/**
 * Web Mercator arithmetic for the maps (the projection OpenStreetMap tiles, the Census and FEMA
 * exports and the Forest Service image service all share, EPSG:3857): latitude and longitude to
 * world pixels and tiles and back, the frame a map covers at a zoom and canvas size, which tiles a
 * frame needs, the export box in metres, and the length a scale bar should show.
 *
 * World pixels at zoom z run from 0 to 256·2^z across the world, x east from 180°W and y south from
 * about 85.05°N. A frame is a centre, a zoom and a canvas size in CSS pixels; it always shows whole
 * world pixels at that zoom, one tile pixel to one canvas pixel, so tile labels print at their size.
 */
import type { LatLon } from '../../engine/types';

export const TILE_SIZE = 256;
/** WGS 84 semi-major axis, the sphere Web Mercator uses. */
export const EARTH_RADIUS_M = 6378137;
const CIRCUMFERENCE_M = 2 * Math.PI * EARTH_RADIUS_M;
/** Web Mercator stops here (the latitude that makes the world square). */
export const MAX_LATITUDE = 85.0511287798066;

export interface Point {
  x: number;
  y: number;
}

/** [west, south, east, north] in degrees. */
export type LonLatBox = [number, number, number, number];
/** [xmin, ymin, xmax, ymax] in Web Mercator metres (EPSG:3857), as ArcGIS `bbox` takes it. */
export type MercatorBox = [number, number, number, number];

const clampLat = (lat: number) => Math.max(-MAX_LATITUDE, Math.min(MAX_LATITUDE, lat));

/** The world size in pixels at zoom `z`. */
export function worldSize(z: number): number {
  return TILE_SIZE * 2 ** z;
}

/** Latitude and longitude to world pixels at zoom `z`. */
export function project(p: LatLon, z: number): Point {
  const size = worldSize(z);
  const lat = (clampLat(p.lat) * Math.PI) / 180;
  const x = ((p.lon + 180) / 360) * size;
  const y = (0.5 - Math.log(Math.tan(Math.PI / 4 + lat / 2)) / (2 * Math.PI)) * size;
  return { x, y };
}

/** World pixels at zoom `z` back to latitude and longitude. */
export function unproject(pt: Point, z: number): LatLon {
  const size = worldSize(z);
  const lon = (pt.x / size) * 360 - 180;
  const n = Math.PI - (2 * Math.PI * pt.y) / size;
  const lat = (Math.atan(Math.sinh(n)) * 180) / Math.PI;
  return { lat, lon };
}

/** The tile holding a point: integer x and y at zoom `z`. */
export function tileOf(p: LatLon, z: number): { x: number; y: number; z: number } {
  const { x, y } = project(p, z);
  const n = 2 ** z;
  return { x: Math.min(n - 1, Math.max(0, Math.floor(x / TILE_SIZE))), y: Math.min(n - 1, Math.max(0, Math.floor(y / TILE_SIZE))), z };
}

/** The north-west corner of a tile. */
export function tileCorner(x: number, y: number, z: number): LatLon {
  return unproject({ x: x * TILE_SIZE, y: y * TILE_SIZE }, z);
}

/** Latitude and longitude to Web Mercator metres. */
export function toMercator(p: LatLon): Point {
  const lat = (clampLat(p.lat) * Math.PI) / 180;
  return { x: (p.lon * Math.PI * EARTH_RADIUS_M) / 180, y: Math.log(Math.tan(Math.PI / 4 + lat / 2)) * EARTH_RADIUS_M };
}

/** Ground metres per world pixel at a latitude and zoom (what a scale bar measures). */
export function metresPerPixel(lat: number, z: number): number {
  return (CIRCUMFERENCE_M * Math.cos((clampLat(lat) * Math.PI) / 180)) / worldSize(z);
}

/** Great-circle distance in metres (for "nearest first"). */
export function distanceM(a: LatLon, b: LatLon): number {
  const r = (d: number) => (d * Math.PI) / 180;
  const dLat = r(b.lat - a.lat);
  const dLon = r(b.lon - a.lon);
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(r(a.lat)) * Math.cos(r(b.lat)) * Math.sin(dLon / 2) ** 2;
  return 2 * 6371008.8 * Math.asin(Math.min(1, Math.sqrt(h)));
}

// ---------------------------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------------------------

/** What one map shows: a centre, a whole-number zoom and a canvas size in CSS pixels. */
export interface Frame {
  center: LatLon;
  zoom: number;
  width: number;
  height: number;
}

/** The frame's top-left corner in world pixels (whole pixels, so tiles land on pixel boundaries). */
export function frameOrigin(frame: Frame): Point {
  const c = project(frame.center, frame.zoom);
  return { x: Math.round(c.x - frame.width / 2), y: Math.round(c.y - frame.height / 2) };
}

/** Where a point falls on the frame's canvas, in CSS pixels (may be outside it). */
export function toCanvas(frame: Frame, p: LatLon): Point {
  const o = frameOrigin(frame);
  const w = project(p, frame.zoom);
  return { x: w.x - o.x, y: w.y - o.y };
}

/** The point under a canvas pixel. */
export function fromCanvas(frame: Frame, pt: Point): LatLon {
  const o = frameOrigin(frame);
  return unproject({ x: o.x + pt.x, y: o.y + pt.y }, frame.zoom);
}

/** Whether a point falls inside the frame, less `margin` pixels on each side. */
export function inFrame(frame: Frame, p: LatLon, margin = 0): boolean {
  const c = toCanvas(frame, p);
  return c.x >= margin && c.y >= margin && c.x <= frame.width - margin && c.y <= frame.height - margin;
}

/** The frame's box in degrees. */
export function frameBox(frame: Frame): LonLatBox {
  const o = frameOrigin(frame);
  const nw = unproject(o, frame.zoom);
  const se = unproject({ x: o.x + frame.width, y: o.y + frame.height }, frame.zoom);
  return [nw.lon, se.lat, se.lon, nw.lat];
}

/** The frame's box in Web Mercator metres, for an ArcGIS export of exactly the canvas. */
export function frameMercatorBox(frame: Frame): MercatorBox {
  const o = frameOrigin(frame);
  const m = CIRCUMFERENCE_M / worldSize(frame.zoom);
  const half = CIRCUMFERENCE_M / 2;
  return [o.x * m - half, half - (o.y + frame.height) * m, (o.x + frame.width) * m - half, half - o.y * m];
}

/** The map scale an ArcGIS server computes for the export (metres per pixel at 96 dpi, in its own units). */
export function exportScale(frame: Frame): number {
  return (CIRCUMFERENCE_M / worldSize(frame.zoom)) * (96 / 0.0254);
}

/** One tile a frame needs, and where its top-left corner lands on the canvas. */
export interface TilePlacement {
  z: number;
  /** Tile column, wrapped into the world (0 … 2^z − 1). */
  x: number;
  y: number;
  /** Canvas position of the tile's top-left corner, CSS pixels. */
  left: number;
  top: number;
}

/**
 * Every tile the frame touches, left to right, top to bottom. Columns wrap around the 180th
 * meridian; rows beyond the poles are left out (there is nothing there).
 */
export function tilesFor(frame: Frame): TilePlacement[] {
  const o = frameOrigin(frame);
  const n = 2 ** frame.zoom;
  const x0 = Math.floor(o.x / TILE_SIZE);
  const x1 = Math.floor((o.x + frame.width - 1) / TILE_SIZE);
  const y0 = Math.max(0, Math.floor(o.y / TILE_SIZE));
  const y1 = Math.min(n - 1, Math.floor((o.y + frame.height - 1) / TILE_SIZE));
  const out: TilePlacement[] = [];
  for (let ty = y0; ty <= y1; ty++) {
    for (let tx = x0; tx <= x1; tx++) {
      out.push({ z: frame.zoom, x: ((tx % n) + n) % n, y: ty, left: tx * TILE_SIZE - o.x, top: ty * TILE_SIZE - o.y });
    }
  }
  return out;
}

/** The most tiles a frame of this size can ever need, wherever it sits on the tile grid. */
export function maxTilesFor(width: number, height: number): number {
  return (Math.ceil((width - 1) / TILE_SIZE) + 1) * (Math.ceil((height - 1) / TILE_SIZE) + 1);
}

/**
 * The highest zoom in [`minZoom`, `maxZoom`] at which every point fits inside a `width`×`height`
 * canvas with `padding` pixels to spare on each side, and the centre that does it (the middle of
 * the points' box). With one point, `maxZoom` around it.
 */
export function fitFrame(points: readonly LatLon[], width: number, height: number, padding: number, minZoom: number, maxZoom: number): Frame {
  if (points.length === 0) throw new Error('fitFrame needs at least one point');
  for (let z = maxZoom; z >= minZoom; z--) {
    const pts = points.map((p) => project(p, z));
    const xs = pts.map((p) => p.x);
    const ys = pts.map((p) => p.y);
    const [minX, maxX, minY, maxY] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
    if (maxX - minX <= width - 2 * padding && maxY - minY <= height - 2 * padding) {
      return { center: unproject({ x: (minX + maxX) / 2, y: (minY + maxY) / 2 }, z), zoom: z, width, height };
    }
  }
  const pts = points.map((p) => project(p, minZoom));
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  return {
    center: unproject({ x: (Math.min(...xs) + Math.max(...xs)) / 2, y: (Math.min(...ys) + Math.max(...ys)) / 2 }, minZoom),
    zoom: minZoom,
    width,
    height,
  };
}

// ---------------------------------------------------------------------------------------------
// Scale bar
// ---------------------------------------------------------------------------------------------

export interface ScaleBar {
  /** Length on the ground, in the bar's unit. */
  value: number;
  unit: 'm' | 'km' | 'ft' | 'mi';
  /** The bar's length on the canvas, CSS pixels. */
  pixels: number;
  /** "500 m", "2 km", "1,000 ft", "½ mi". */
  label: string;
}

const NICE = [1, 2, 5];

function niceBelow(x: number): number {
  const p = 10 ** Math.floor(Math.log10(x));
  let best = p;
  for (const n of NICE) if (n * p <= x) best = n * p;
  return best;
}

/**
 * The longest round length that fits in `maxPixels` at the frame's centre, in metric (m, km) or US
 * (ft, mi) units. Miles use ¼, ½, 1, 2, 5… so short bars stay in miles once they pass 1,000 ft.
 */
export function scaleBar(frame: Frame, maxPixels: number, system: 'metric' | 'us'): ScaleBar {
  const mpp = metresPerPixel(frame.center.lat, frame.zoom);
  const maxM = mpp * maxPixels;
  if (system === 'metric') {
    const m = niceBelow(maxM);
    if (m >= 1000) return { value: m / 1000, unit: 'km', pixels: m / mpp, label: `${(m / 1000).toLocaleString('en-US')} km` };
    return { value: m, unit: 'm', pixels: m / mpp, label: `${m.toLocaleString('en-US')} m` };
  }
  const FT = 0.3048;
  const MI = 1609.344;
  const maxMi = maxM / MI;
  if (maxMi >= 0.25) {
    const quarters = [0.25, 0.5];
    const mi = maxMi >= 1 ? niceBelow(maxMi) : quarters.filter((q) => q <= maxMi).pop()!;
    const label = mi === 0.25 ? '¼ mi' : mi === 0.5 ? '½ mi' : `${mi.toLocaleString('en-US')} mi`;
    return { value: mi, unit: 'mi', pixels: (mi * MI) / mpp, label };
  }
  const ft = niceBelow(maxM / FT);
  return { value: ft, unit: 'ft', pixels: (ft * FT) / mpp, label: `${ft.toLocaleString('en-US')} ft` };
}
