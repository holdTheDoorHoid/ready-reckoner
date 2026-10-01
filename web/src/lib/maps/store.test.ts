import { IDBFactory } from 'fake-indexeddb';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import type { ComposedMap } from './compose';
import { emptyMapsState, type MapsState } from './state';
import {
  clearMaps,
  deleteMapsDatabase,
  getMapImage,
  loadMap,
  loadMaps,
  mapLegend,
  mapRecordsFor,
  MAPS_DB,
  mapsKey,
  mapsStorageAvailable,
  saveMaps,
  toRecord,
} from './store';

const maps: MapsState = { ...emptyMapsState(), home: { lat: 39.9364, lon: -75.1534 }, layers: { places: true, flood: true, surge: false, wildfire: false }, fetched_on: '2026-10-01' };

function composed(slot: ComposedMap['slot']): ComposedMap {
  return {
    slot,
    image: `data:image/jpeg;base64,${slot}`,
    width: 800,
    height: 560,
    bytes: 123456,
    zoom: 16,
    legend: [{ mark: 'H', name: 'Home', kind: 'Your plan', own: true }],
    keys: [{ pattern: 'stripes', text: 'FEMA high-risk flood zone: about a 1 in 100 chance of flooding each year.' }],
    statuses: [],
    notes: [],
    credits: ['Map data © OpenStreetMap contributors, openstreetmap.org/copyright'],
    scale: 'The bars show 500 ft and 200 m.',
    fetched_on: '2026-10-01',
  };
}

const original = globalThis.indexedDB;

beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
});

afterEach(() => {
  globalThis.indexedDB = original;
});

describe('The maps store (IndexedDB "rr-maps")', () => {
  it('keeps one record per slot and gives them back unchanged', async () => {
    const key = mapsKey(maps, '42101');
    const records = (['neighbourhood', 'area', 'region'] as const).map((s) => toRecord(composed(s), key));
    expect(await saveMaps(records)).toBe(true);
    expect(await loadMap('area')).toEqual(records[1]);
    expect((await loadMaps()).map((r) => r.slot).sort()).toEqual(['area', 'neighbourhood', 'region']);
    // A new press replaces every record.
    expect(await saveMaps([toRecord(composed('region'), key)])).toBe(true);
    expect((await loadMaps()).map((r) => r.slot)).toEqual(['region']);
  });

  it('uses the database name "Forget everything" deletes', async () => {
    await saveMaps([toRecord(composed('area'), 'k')]);
    const names = await globalThis.indexedDB.databases();
    expect(names.map((d) => d.name)).toEqual([MAPS_DB]);
    expect(MAPS_DB).toBe('rr-maps');
  });

  it('"Remove maps" clears the records; "Forget everything" deletes the database', async () => {
    await saveMaps([toRecord(composed('area'), 'k')]);
    expect(await clearMaps()).toBe(true);
    expect(await loadMaps()).toEqual([]);
    await deleteMapsDatabase();
    expect(await globalThis.indexedDB.databases()).toEqual([]);
  });

  it('shows a plan only its own maps: moved pins or an imported plan need refreshing', async () => {
    const owner = { maps, countyFips: '42101' };
    expect((await mapRecordsFor(owner)).status).toBe('stale');
    await saveMaps((['neighbourhood', 'area', 'region'] as const).map((s) => toRecord(composed(s), mapsKey(maps, '42101'))));
    const ready = await mapRecordsFor(owner);
    expect(ready.status).toBe('ready');
    expect(Object.keys(ready.records).sort()).toEqual(['area', 'neighbourhood', 'region']);
    // The home pin moved: the images belong to the old choices.
    const moved = { ...maps, home: { lat: 39.95, lon: -75.16 } };
    expect((await mapRecordsFor({ maps: moved, countyFips: '42101' })).status).toBe('stale');
    // Another county, same pins: stale too.
    expect((await mapRecordsFor({ maps, countyFips: '42045' })).status).toBe('stale');
    // A plan that never fetched maps has none to show.
    expect((await mapRecordsFor({ maps: { ...maps, fetched_on: undefined }, countyFips: '42101' })).status).toBe('none');
    expect((await mapRecordsFor({ countyFips: '42101' })).status).toBe('none');
  });

  it('gives web-binder a slot’s image and legend, or null', async () => {
    const owner = { maps, countyFips: '42101' };
    expect(await getMapImage('neighbourhood', owner)).toBeNull();
    await saveMaps([toRecord(composed('neighbourhood'), mapsKey(maps, '42101'))]);
    expect(await getMapImage('neighbourhood', owner)).toEqual({ dataUrl: 'data:image/jpeg;base64,neighbourhood', width: 800, height: 560 });
    const legend = await mapLegend('neighbourhood', owner);
    expect(legend?.legend[0]?.mark).toBe('H');
    expect(legend?.credits[0]).toContain('openstreetmap.org/copyright');
    expect(legend?.fetched_on).toBe('2026-10-01');
    expect(await mapLegend('area', owner)).toBeNull();
  });

  it('says so when the browser keeps no database (a private window)', async () => {
    // @ts-expect-error: a browser with no IndexedDB
    globalThis.indexedDB = undefined;
    expect(await mapsStorageAvailable()).toBe(false);
    expect(await saveMaps([toRecord(composed('area'), 'k')])).toBe(false);
    expect(await loadMaps()).toEqual([]);
    expect((await mapRecordsFor({ maps, countyFips: '42101' })).status).toBe('unavailable');
    await expect(deleteMapsDatabase()).resolves.toBeUndefined();
  });

  it('keys the choices in a fixed order, so the same choices always give the same key', () => {
    const a = mapsKey(maps, '42101');
    expect(mapsKey(JSON.parse(JSON.stringify(maps)) as MapsState, '42101')).toBe(a);
    expect(mapsKey({ ...maps, layers: { ...maps.layers, flood: false } }, '42101')).not.toBe(a);
    expect(mapsKey(undefined, '42101')).toBe('');
  });
});
