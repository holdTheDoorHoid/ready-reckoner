/**
 * Where the map images live (DESIGN-DELTA-v3 §9.5): the IndexedDB database `rr-maps`, one record
 * per slot with the JPEG, its legend rows, pattern keys, statuses, notes, credits and the day it
 * was fetched. Only on this device: never in the saved file, never sent anywhere. "Forget
 * everything" deletes the whole database (`deleteMapsDatabase`, called from persistence by
 * web-interview3); "Remove maps" clears it.
 *
 * Each record carries the key of the map choices it was drawn for (`mapsKey`: the pins, the
 * routes, the layers, the fetch date and the county). The images are shown only while that key
 * matches the plan's own `SavedPlan.maps`: an imported plan, moved pins or a new county make them
 * "need refreshing" instead of showing another plan's maps.
 */
import type { IsoDate } from '../../engine/types';
import type { ComposedMap, LayerStatus, LegendKey, LegendRow } from './compose';
import type { MapSlotKind } from './slots';
import { type MapsState, PIN_IDS } from './state';

/** The database name (agreed with web-interview3, whose "Forget everything" deletes it). */
export const MAPS_DB = 'rr-maps';
const STORE = 'maps';
const VERSION = 1;

/** One stored map. */
export interface MapRecord {
  slot: MapSlotKind;
  /** `data:image/jpeg;base64,…` */
  image: string;
  width: number;
  height: number;
  bytes: number;
  zoom: number;
  legend: LegendRow[];
  keys: LegendKey[];
  statuses: LayerStatus[];
  notes: string[];
  credits: string[];
  scale: string;
  fetched_on: IsoDate;
  /** `mapsKey` of the choices the map was drawn for. */
  key: string;
}

/** Whether the stored maps can be shown for a plan. */
export type MapsStatus = 'none' | 'ready' | 'stale' | 'unavailable';

/**
 * The key of a plan's map choices: the pins, the routes, the layers, the fetch date and the county,
 * in a fixed order. Two plans with the same key would draw the same maps.
 */
export function mapsKey(maps: MapsState | undefined, countyFips: string): string {
  if (!maps) return '';
  const pins = PIN_IDS.map((id) => (maps[id] ? `${maps[id]!.lat},${maps[id]!.lon}` : '-'));
  const routes = maps.routes.map((r) => r.map((p) => `${p.lat},${p.lon}`).join(' '));
  const l = maps.layers;
  const layers = [l.base !== false, l.places, l.flood, l.surge, l.wildfire].map((b) => (b ? 1 : 0)).join('');
  return JSON.stringify([countyFips, pins, routes, layers, maps.fetched_on ?? '']);
}

/** A composed map as it is stored. */
export function toRecord(map: ComposedMap, key: string): MapRecord {
  return {
    slot: map.slot,
    image: map.image,
    width: map.width,
    height: map.height,
    bytes: map.bytes,
    zoom: map.zoom,
    legend: map.legend,
    keys: map.keys,
    statuses: map.statuses,
    notes: map.notes,
    credits: map.credits,
    scale: map.scale,
    fetched_on: map.fetched_on,
    key,
  };
}

function factory(): IDBFactory | null {
  try {
    return typeof indexedDB === 'undefined' ? null : indexedDB;
  } catch {
    return null;
  }
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error('IndexedDB request failed'));
  });
}

function done(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error ?? new Error('IndexedDB transaction failed'));
    tx.onabort = () => reject(tx.error ?? new Error('IndexedDB transaction aborted'));
  });
}

/** Open the database, or null when this browser keeps none (a private window, blocked storage). */
async function open(): Promise<IDBDatabase | null> {
  const idb = factory();
  if (!idb) return null;
  try {
    const req = idb.open(MAPS_DB, VERSION);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains(STORE)) req.result.createObjectStore(STORE, { keyPath: 'slot' });
    };
    const db = await request(req);
    // Another tab deleting the database ("Forget everything") must not wait on this connection.
    db.onversionchange = () => db.close();
    return db;
  } catch {
    return null;
  }
}

async function withDb<T>(fallback: T, run: (db: IDBDatabase) => Promise<T>): Promise<T> {
  const db = await open();
  if (!db) return fallback;
  try {
    return await run(db);
  } catch {
    return fallback;
  } finally {
    db.close();
  }
}

/** Whether this browser can keep the maps at all. */
export async function mapsStorageAvailable(): Promise<boolean> {
  return withDb(false, async () => true);
}

/** Replace every stored map with these, in one transaction. False when nothing could be kept. */
export function saveMaps(records: readonly MapRecord[]): Promise<boolean> {
  return withDb(false, async (db) => {
    const tx = db.transaction(STORE, 'readwrite');
    const store = tx.objectStore(STORE);
    store.clear();
    for (const r of records) store.put(r);
    await done(tx);
    return true;
  });
}

/** Every stored map. */
export function loadMaps(): Promise<MapRecord[]> {
  return withDb([] as MapRecord[], async (db) => request(db.transaction(STORE, 'readonly').objectStore(STORE).getAll() as IDBRequest<MapRecord[]>));
}

/** One stored map, or null. */
export function loadMap(slot: MapSlotKind): Promise<MapRecord | null> {
  return withDb(null as MapRecord | null, async (db) => (await request(db.transaction(STORE, 'readonly').objectStore(STORE).get(slot) as IDBRequest<MapRecord | undefined>)) ?? null);
}

/** "Remove maps": the images go; the database stays. */
export function clearMaps(): Promise<boolean> {
  return withDb(false, async (db) => {
    const tx = db.transaction(STORE, 'readwrite');
    tx.objectStore(STORE).clear();
    await done(tx);
    return true;
  });
}

/** "Forget everything": delete the database itself. Resolves even when there was none. */
export function deleteMapsDatabase(): Promise<void> {
  const idb = factory();
  if (!idb) return Promise.resolve();
  return new Promise((resolve) => {
    try {
      const req = idb.deleteDatabase(MAPS_DB);
      req.onsuccess = () => resolve();
      req.onerror = () => resolve();
      req.onblocked = () => resolve();
    } catch {
      resolve();
    }
  });
}

// ---------------------------------------------------------------------------------------------
// What the binder asks for (web-binder)
// ---------------------------------------------------------------------------------------------

/** The plan whose maps are wanted: its map choices and its county. */
export interface MapsOwner {
  maps?: MapsState;
  countyFips: string;
}

/**
 * The stored maps for a plan, and whether they can be shown: `none` (no maps were fetched for it),
 * `ready`, `stale` (pins but no matching images: "Maps need refreshing") or `unavailable` (this
 * browser keeps no maps).
 */
export async function mapRecordsFor(owner: MapsOwner): Promise<{ status: MapsStatus; records: Partial<Record<MapSlotKind, MapRecord>> }> {
  if (!(await mapsStorageAvailable())) return { status: 'unavailable', records: {} };
  if (!owner.maps?.fetched_on) return { status: 'none', records: {} };
  const key = mapsKey(owner.maps, owner.countyFips);
  const all = await loadMaps();
  const records: Partial<Record<MapSlotKind, MapRecord>> = {};
  for (const r of all) if (r.key === key) records[r.slot] = r;
  return { status: Object.keys(records).length > 0 ? 'ready' : 'stale', records };
}

/** A slot's image for the PDF, or null when there is none for this plan. */
export async function getMapImage(slot: MapSlotKind, owner: MapsOwner): Promise<{ dataUrl: string; width: number; height: number } | null> {
  const r = (await mapRecordsFor(owner)).records[slot];
  return r ? { dataUrl: r.image, width: r.width, height: r.height } : null;
}

/** A slot's legend for the PDF: rows, keys, statuses, notes, credits, the scale and the date. */
export async function mapLegend(
  slot: MapSlotKind,
  owner: MapsOwner,
): Promise<Pick<MapRecord, 'legend' | 'keys' | 'statuses' | 'notes' | 'credits' | 'scale' | 'fetched_on'> | null> {
  const r = (await mapRecordsFor(owner)).records[slot];
  return r ? { legend: r.legend, keys: r.keys, statuses: r.statuses, notes: r.notes, credits: r.credits, scale: r.scale, fetched_on: r.fetched_on } : null;
}
