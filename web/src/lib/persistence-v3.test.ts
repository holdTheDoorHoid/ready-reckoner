/**
 * The saved plan, version 2 (DESIGN-DELTA-v3 §2.4, §9.5): what it holds, how a version-1 entry or
 * file becomes version 2 with nothing lost, the maps' pins, and "Forget everything" clearing the
 * maps store too.
 */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { clone, MemoryStorage, savedFor } from '../test/helpers';
import {
  checkMaps,
  checkSavedPlan,
  exportText,
  forgetEverything,
  forgetMaps,
  loadPlan,
  MAPS_DB_NAME,
  type MapsState,
  newPlan,
  OPTIONAL_STEP_IDS,
  parseImport,
  REQUIRED_STEP_IDS,
  SAVED_PLAN_VERSION,
  savePlan,
  STEP_IDS,
  STORAGE_KEY,
} from './persistence';

const philly = FIXTURES['philadelphia-renters-4'];

/** A version-1 saved plan as v0.2.0 wrote it. */
function v1(extra: Record<string, unknown> = {}): Record<string, unknown> {
  const plan = savedFor(philly) as unknown as Record<string, unknown>;
  return { ...plan, version: 1, progress: { completed: ['where', 'who', 'travel', 'money', 'have'], last: '#/plan' }, ...extra };
}

const MAPS: MapsState = {
  home: { lat: 39.9312, lon: -75.1543 },
  meeting_near: { lat: 39.9321, lon: -75.1532 },
  where_go: { lat: 40.2732, lon: -76.8867 },
  routes: [
    [
      { lat: 39.93, lon: -75.15 },
      { lat: 40.0, lon: -75.3 },
    ],
  ],
  layers: { places: true, flood: true, surge: false, wildfire: false },
  fetched_on: '2026-10-02',
};

describe('the steps', () => {
  it('are the five required steps, then the three optional ones', () => {
    expect(REQUIRED_STEP_IDS).toEqual(['where', 'who', 'travel', 'money', 'have']);
    expect(OPTIONAL_STEP_IDS).toEqual(['people', 'places', 'contacts']);
    expect(STEP_IDS).toEqual([...REQUIRED_STEP_IDS, ...OPTIONAL_STEP_IDS]);
  });

  it('keep the optional steps in the progress, and drop steps that do not exist', () => {
    const r = checkSavedPlan({ ...v1(), progress: { completed: ['where', 'people', 'contacts', 'family', 'plan'] } });
    expect(r.ok && r.plan.progress.completed).toEqual(['where', 'people', 'contacts']);
  });
});

describe('version 2', () => {
  it('is what a new plan and every save write', () => {
    expect(SAVED_PLAN_VERSION).toBe(2);
    expect(newPlan(clone(philly)).version).toBe(2);
    expect(JSON.parse(exportText(savedFor(philly))).version).toBe(2);
  });

  it('reads a version-1 file unchanged, as version 2: every field keeps its name and value', () => {
    const old = v1({
      purchases: [{ item_id: 'water_stored', tier: 'h72', qty: 3, paid_usd: 3, date: '2026-10-05' }],
      confidence: { before: 2, after: 4 },
      dismissed_warnings: ['no_renters_insurance'],
      done_dates: { 'check:alarms_test': '2026-11-01' },
      reviewed_on: '2026-10-01',
    });
    const r = parseImport(JSON.stringify(old));
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.plan.version).toBe(2);
    const { version: _new, ...rest } = r.plan;
    const { version: _old, ...before } = old;
    expect(rest).toEqual(before);
    expect(r.plan.maps).toBeUndefined();
  });

  it('migrates a version-1 entry in browser storage, and saves it back as version 2', () => {
    const storage = new MemoryStorage();
    storage.setItem(STORAGE_KEY, JSON.stringify(v1()));
    const loaded = loadPlan(storage);
    expect(loaded.damaged).toBe(false);
    expect(loaded.plan?.version).toBe(2);
    expect(loaded.plan?.input).toEqual(philly);
    savePlan(storage, loaded.plan);
    expect(JSON.parse(storage.getItem(STORAGE_KEY)!).version).toBe(2);
  });

  it('keeps top-level keys it does not know, so opening and saving never loses them', () => {
    const r = checkSavedPlan({ ...v1(), from_a_later_version: { keep: ['me'] }, note: 'x' });
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    const back = JSON.parse(exportText(r.plan)) as Record<string, unknown>;
    expect(back.from_a_later_version).toEqual({ keep: ['me'] });
    expect(back.note).toBe('x');
    expect(back.version).toBe(2);
  });

  it('refuses a version it does not know, with a reason', () => {
    for (const version of [0, '2', undefined, null]) {
      const r = checkSavedPlan({ ...savedFor(philly), version });
      expect(r.ok, String(version)).toBe(false);
      if (!r.ok) expect(r.reason).toMatch(/version/);
    }
    const newer = checkSavedPlan({ ...savedFor(philly), version: 3 });
    expect(!newer.ok && newer.reason).toMatch(/newer version/);
  });

  it('holds the answers of the optional steps inside the household, as they are', () => {
    const plan = savedFor(philly);
    plan.input.people[0]!.profile = { name: 'Ana', medications: [{ name: 'Blood pressure tablet', dose: '10 mg' }] };
    plan.input.family_plan = { home: { address: '12 Sample St' }, documents: { accounts: [{ institution: 'First Sample Bank', last4: '1234' }] } };
    const r = parseImport(exportText(plan));
    expect(r.ok && r.plan.input).toEqual(plan.input);
  });
});

describe('the maps (web-only, outside the household)', () => {
  it('survive storage and the saved file', () => {
    const plan = savedFor(philly, { maps: clone(MAPS) });
    const storage = new MemoryStorage();
    savePlan(storage, plan);
    expect(loadPlan(storage).plan?.maps).toEqual(MAPS);
    const r = parseImport(exportText(plan));
    expect(r.ok && r.plan.maps).toEqual(MAPS);
    // The engine never sees them.
    expect(JSON.stringify(plan.input)).not.toContain('meeting_near');
  });

  it('keep what is well formed and drop the rest, never the whole plan', () => {
    const maps = checkMaps({
      home: { lat: 39.93, lon: -75.15 },
      meeting_near: { lat: 'north', lon: -75.15 },
      meeting_far: { lat: 95, lon: 0 },
      where_go: null,
      routes: [[{ lat: 39.9, lon: -75.1 }, { lat: 'x' }, { lat: 40, lon: -75.2 }], 'not a route'],
      layers: { places: true, flood: 'yes', wildfire: true },
      fetched_on: 'yesterday',
      extra: 1,
    });
    expect(maps).toEqual({
      home: { lat: 39.93, lon: -75.15 },
      routes: [
        [
          { lat: 39.9, lon: -75.1 },
          { lat: 40, lon: -75.2 },
        ],
      ],
      layers: { places: true, flood: false, surge: false, wildfire: true },
    });
    expect(checkMaps('pins')).toBeUndefined();
    const r = checkSavedPlan({ ...v1(), maps: 'damaged' });
    expect(r.ok && r.plan.maps).toBeUndefined();
  });
});

/** A stand-in for the browser's IndexedDB that records what was deleted. */
function fakeIndexedDB(outcome: 'success' | 'error' | 'blocked' = 'success') {
  const deleted: string[] = [];
  const factory = {
    deleteDatabase(name: string) {
      deleted.push(name);
      const request = {} as IDBOpenDBRequest;
      setTimeout(() => {
        if (outcome === 'success') request.onsuccess?.(new Event('success'));
        else if (outcome === 'error') request.onerror?.(new Event('error'));
        else request.onblocked?.(new Event('blocked') as IDBVersionChangeEvent);
      }, 0);
      return request;
    },
  } as unknown as IDBFactory;
  return { factory, deleted };
}

describe('forgetting everything', () => {
  it('deletes the maps store (rr-maps) along with every rr. key', async () => {
    const storage = new MemoryStorage();
    savePlan(storage, savedFor(philly, { maps: clone(MAPS) }));
    storage.setItem('someone-else', 'keep me');
    const idb = fakeIndexedDB();
    expect(forgetEverything(storage, idb.factory)).toBe(1);
    expect(storage.getItem(STORAGE_KEY)).toBeNull();
    expect(storage.getItem('someone-else')).toBe('keep me');
    expect(idb.deleted).toEqual([MAPS_DB_NAME]);
    expect(MAPS_DB_NAME).toBe('rr-maps');
  });

  it('says whether the maps store went, and copes with no IndexedDB at all', async () => {
    expect(await forgetMaps(fakeIndexedDB('success').factory)).toBe(true);
    expect(await forgetMaps(fakeIndexedDB('blocked').factory)).toBe(true);
    expect(await forgetMaps(fakeIndexedDB('error').factory)).toBe(false);
    expect(await forgetMaps(null)).toBe(false);
    const throwing = { deleteDatabase: () => { throw new DOMException('denied', 'SecurityError'); } } as unknown as IDBFactory;
    expect(await forgetMaps(throwing)).toBe(false);
    expect(forgetEverything(new MemoryStorage(), null)).toBe(0);
  });

  it('still forgets the plan in a browser that throws when IndexedDB is merely looked at', () => {
    const storage = new MemoryStorage();
    savePlan(storage, savedFor(philly));
    Object.defineProperty(globalThis, 'indexedDB', {
      configurable: true,
      get() {
        throw new DOMException('denied', 'SecurityError');
      },
    });
    try {
      expect(forgetEverything(storage)).toBe(1);
      expect(storage.getItem(STORAGE_KEY)).toBeNull();
    } finally {
      delete (globalThis as { indexedDB?: unknown }).indexedDB;
    }
  });
});
