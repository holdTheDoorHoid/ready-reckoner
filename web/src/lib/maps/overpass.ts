/**
 * Nearby places from OpenStreetMap, through the Overpass API (DESIGN-DELTA-v3 §9.2, §9.3): one
 * query a press covering both maps' boxes (the neighbourhood's everyday places, the area's
 * hospitals, fire stations and police), read into places with a kind, a name, and an address or
 * phone when OpenStreetMap has one, then the nearest few of each kind picked for each map and
 * numbered for its legend.
 *
 * Place names, addresses and phone numbers are OpenStreetMap's data, echoed as tagged (trimmed and
 * capped, never checked or reworded) and credited beside the map. They include shop names: that is
 * what is mapped near the household, not a recommendation.
 */
import type { LatLon } from '../../engine/types';
import { distanceM, type Frame, inFrame, type LonLatBox } from './tiles';

export const PLACE_KINDS = ['pharmacy', 'grocery', 'clinic', 'fuel', 'school', 'childcare', 'fire_station', 'police', 'hospital_er', 'hospital'] as const;
export type PlaceKind = (typeof PLACE_KINDS)[number];

/** What each kind is called in a legend. */
export const KIND_LABELS: Record<PlaceKind, string> = {
  pharmacy: 'Pharmacy',
  grocery: 'Grocery store',
  clinic: 'Clinic or urgent care',
  fuel: 'Fuel',
  school: 'School',
  childcare: 'Child care',
  fire_station: 'Fire station',
  police: 'Police',
  hospital_er: 'Hospital with an emergency room',
  hospital: 'Hospital (emergency room not listed)',
};

export interface Place {
  /** "node/123", "way/456". */
  id: string;
  kind: PlaceKind;
  /** As tagged, trimmed and capped; absent when the place has no name in OpenStreetMap. */
  name?: string;
  at: LatLon;
  address?: string;
  phone?: string;
}

/** How many of each kind a map lists, nearest first, in legend order. */
export interface KindQuota {
  kind: PlaceKind;
  max: number;
  /** Only for a household with children. */
  children?: boolean;
}

/** The neighbourhood map's places (§9.2). */
export const NEIGHBOURHOOD_QUOTAS: readonly KindQuota[] = [
  { kind: 'pharmacy', max: 3 },
  { kind: 'grocery', max: 3 },
  { kind: 'clinic', max: 2 },
  { kind: 'fuel', max: 2 },
  { kind: 'school', max: 2, children: true },
  { kind: 'childcare', max: 2, children: true },
  { kind: 'fire_station', max: 2 },
  { kind: 'police', max: 1 },
];

/** The area map's places (§9.2). A hospital without a listed emergency room fills in only when fewer than three have one. */
export const AREA_QUOTAS: readonly KindQuota[] = [
  { kind: 'hospital_er', max: 6 },
  { kind: 'hospital', max: 2 },
  { kind: 'fire_station', max: 2 },
  { kind: 'police', max: 2 },
];

const box = ([w, s, e, n]: LonLatBox) => [s, w, n, e].map((v) => v.toFixed(5)).join(',');

/**
 * The one query for a press: everyday places in the neighbourhood box, fire stations and police in
 * the `nearby` box (the middle quarter of the area map: the nearest few are all the maps list, and
 * the whole area box held 293 fire stations and 116 police stations around Philadelphia when
 * checked, 170 kB), hospitals in the whole area box. Only the boxes and the kinds asked for are sent.
 */
export function overpassQuery(boxes: { neighbourhood: LonLatBox; nearby: LonLatBox; area: LonLatBox }, options: { children: boolean; timeoutS: number }): string {
  const nb = box(boxes.neighbourhood);
  const near = box(boxes.nearby);
  const ar = box(boxes.area);
  const lines = [
    `nwr["amenity"="pharmacy"](${nb});`,
    `nwr["healthcare"="pharmacy"](${nb});`,
    `nwr["shop"~"^(supermarket|grocery)$"](${nb});`,
    `nwr["amenity"="clinic"](${nb});`,
    `nwr["healthcare"~"^(clinic|urgent_care)$"](${nb});`,
    `nwr["amenity"="fuel"](${nb});`,
    ...(options.children ? [`nwr["amenity"~"^(school|childcare|kindergarten)$"](${nb});`] : []),
    `nwr["amenity"="hospital"]["emergency"!="no"](${ar});`,
    `nwr["amenity"~"^(fire_station|police)$"](${near});`,
  ];
  return `[out:json][timeout:${options.timeoutS}];\n(\n  ${lines.join('\n  ')}\n);\nout center tags qt;`;
}

/** The kind a set of OpenStreetMap tags describes, or undefined for anything the maps do not show. */
export function kindOf(tags: Record<string, string>): PlaceKind | undefined {
  const amenity = tags.amenity;
  const healthcare = tags.healthcare;
  if (amenity === 'hospital') return tags.emergency === 'no' ? undefined : tags.emergency === 'yes' ? 'hospital_er' : 'hospital';
  if (amenity === 'fire_station') return 'fire_station';
  if (amenity === 'police') return 'police';
  if (amenity === 'pharmacy' || healthcare === 'pharmacy') return 'pharmacy';
  if (amenity === 'clinic' || healthcare === 'clinic' || healthcare === 'urgent_care') return 'clinic';
  if (tags.shop === 'supermarket' || tags.shop === 'grocery') return 'grocery';
  if (amenity === 'fuel') return 'fuel';
  if (amenity === 'school') return 'school';
  if (amenity === 'childcare' || amenity === 'kindergarten') return 'childcare';
  return undefined;
}

const clean = (s: string | undefined, cap: number): string | undefined => {
  const t = s?.replace(/\s+/g, ' ').trim();
  return t ? t.slice(0, cap) : undefined;
};

interface OverpassElement {
  type?: string;
  id?: number;
  lat?: number;
  lon?: number;
  center?: { lat?: number; lon?: number };
  tags?: Record<string, string>;
}

/** Read an Overpass JSON answer into places. Elements without a position or a known kind are skipped. */
export function parseOverpass(json: unknown): Place[] {
  const elements = ((json as { elements?: unknown } | null)?.elements ?? []) as OverpassElement[];
  if (!Array.isArray(elements)) return [];
  const out: Place[] = [];
  for (const el of elements) {
    const tags = el.tags ?? {};
    const kind = kindOf(tags);
    const lat = el.lat ?? el.center?.lat;
    const lon = el.lon ?? el.center?.lon;
    if (!kind || typeof lat !== 'number' || typeof lon !== 'number' || !Number.isFinite(lat) || !Number.isFinite(lon)) continue;
    const street = clean(tags['addr:street'], 100);
    const number = clean(tags['addr:housenumber'], 20);
    const address = street ? (number ? `${number} ${street}` : street) : clean(tags['addr:full'], 120);
    const phone = clean((tags.phone ?? tags['contact:phone'] ?? '').split(';')[0], 40);
    const place: Place = { id: `${el.type ?? 'node'}/${el.id ?? out.length}`, kind, at: { lat, lon } };
    const name = clean(tags.name, 80);
    if (name) place.name = name;
    if (address) place.address = address;
    if (phone) place.phone = phone;
    out.push(place);
  }
  return out;
}

/** The same place mapped twice (a point and a building outline): same kind, same name, within 80 m. */
function duplicate(a: Place, b: Place): boolean {
  return a.kind === b.kind && (a.name ?? '') === (b.name ?? '') && distanceM(a.at, b.at) < 80;
}

/**
 * The places one map lists: those inside its frame (clear of the edge by `margin` pixels), the
 * nearest to `home` first within each kind, named places before unnamed ones, duplicates dropped,
 * at most each quota's `max`, in quota order.
 */
export function pickPlaces(places: readonly Place[], quotas: readonly KindQuota[], frame: Frame, home: LatLon, options: { children: boolean; margin?: number }): Place[] {
  const margin = options.margin ?? 12;
  const out: Place[] = [];
  const erCount = places.filter((p) => p.kind === 'hospital_er' && inFrame(frame, p.at, margin)).length;
  for (const q of quotas) {
    if (q.children && !options.children) continue;
    // Hospitals with no emergency room listed only fill in when fewer than three have one.
    const max = q.kind === 'hospital' ? Math.min(q.max, Math.max(0, 3 - erCount)) : q.max;
    if (max === 0) continue;
    const candidates = places
      .filter((p) => p.kind === q.kind && inFrame(frame, p.at, margin))
      .map((p) => ({ p, d: distanceM(home, p.at) + (p.name ? 0 : 1e7) }))
      .sort((a, b) => a.d - b.d || a.p.id.localeCompare(b.p.id));
    const chosen: Place[] = [];
    for (const { p } of candidates) {
      if (chosen.length >= max) break;
      if (chosen.some((c) => duplicate(c, p))) continue;
      chosen.push(p);
    }
    out.push(...chosen);
  }
  return out;
}
