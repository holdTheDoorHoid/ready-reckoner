import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { clone, MemoryStorage, savedFor } from '../test/helpers';
import {
  checkSavedPlan,
  DEFAULT_PREFS,
  engineInput,
  exportText,
  forgetEverything,
  loadPlan,
  loadPrefs,
  mergeOwned,
  newPlan,
  parseImport,
  PREFS_KEY,
  savePlan,
  savePrefs,
  STORAGE_KEY,
} from './persistence';

const philly = FIXTURES['philadelphia-renters-4'];

describe('the engine view of a saved plan', () => {
  it('merges the starting inventory with check-offs, one entry per item', () => {
    const owned = mergeOwned(
      [{ item_id: 'water_stored', qty: 10 }],
      [
        { item_id: 'water_stored', qty: 3, paid_usd: 3, date: '2026-10-02' },
        { item_id: 'co_alarm', qty: 1, paid_usd: 28, date: '2026-10-02' },
        { item_id: 'co_alarm', qty: 1, paid_usd: 30, date: '2026-10-09' },
      ],
    );
    expect(owned).toEqual([
      // The starting 10 gallons had no price, so no misleading unit price is sent.
      { item_id: 'water_stored', qty: 13 },
      { item_id: 'co_alarm', qty: 2, paid_usd: 58 },
    ]);
  });

  it('keeps earners consistent and attaches the latest confidence, without touching the saved plan', () => {
    const plan = savedFor(philly);
    plan.input.finances.income.earners = 7;
    plan.confidence = { before: 2, after: 4 };
    plan.purchases = [{ item_id: 'alarms_test', tier: 'now', qty: 1, date: '2026-10-02' }];
    const input = engineInput(plan);
    expect(input.finances.income.earners).toBe(2);
    expect(input.confidence_1to5).toBe(4);
    expect(input.existing).toEqual([{ item_id: 'alarms_test', qty: 1 }]);
    expect(plan.input.finances.income.earners).toBe(7);
    expect(plan.input.existing).toEqual([]);
  });
});

describe('checking saved plans and imports', () => {
  it('accepts its own export and round-trips it exactly', () => {
    const plan = savedFor(philly, {
      purchases: [{ item_id: 'water_stored', tier: 'h72', qty: 3, paid_usd: 3, date: '2026-10-05' }],
      confidence: { before: 2 },
      dismissed_warnings: ['no_renters_insurance'],
      done_dates: { 'check:alarms_test': '2026-11-01' },
      reviewed_on: '2026-10-01',
    });
    const text = exportText(plan, new Date('2026-10-06T12:00:00Z'));
    const back = parseImport(text);
    expect(back.ok).toBe(true);
    if (!back.ok) return;
    expect(back.plan).toEqual(plan);
    expect(JSON.parse(text).saved_at).toBe('2026-10-06T12:00:00.000Z');
  });

  it('accepts a bare household file (such as a fixture) as a new plan', () => {
    const back = parseImport(JSON.stringify(philly));
    expect(back.ok && back.plan.input).toEqual(philly);
    expect(back.ok && back.plan.purchases).toEqual([]);
  });

  it('rejects files that are not plans, with a reason a person can act on', () => {
    for (const text of ['not json', '[]', '{"format":"something-else"}', JSON.stringify({ format: 'ready-reckoner-plan', version: 1 })]) {
      const r = parseImport(text);
      expect(r.ok).toBe(false);
      if (!r.ok) expect(r.reason.length).toBeGreaterThan(10);
    }
    const newer = checkSavedPlan({ ...savedFor(philly), version: 2 });
    expect(!newer.ok && newer.reason).toMatch(/newer version/);
    const damaged = checkSavedPlan({ ...savedFor(philly), purchases: [{ item_id: 'x', qty: -1, date: 'soon' }] });
    expect(damaged.ok).toBe(false);
  });

  it('drops unknown bits of progress rather than failing', () => {
    const r = checkSavedPlan({ ...savedFor(philly), progress: { completed: ['where', 'nonsense'] }, confidence: { before: 9 } });
    expect(r.ok && r.plan.progress.completed).toEqual(['where']);
    expect(r.ok && r.plan.confidence).toEqual({});
  });
});

describe('browser storage', () => {
  it('saves and loads the plan under rr.plan.v1', () => {
    const storage = new MemoryStorage();
    const plan = newPlan(clone(philly));
    expect(savePlan(storage, plan)).toBe(true);
    expect(storage.getItem(STORAGE_KEY)).not.toBeNull();
    expect(loadPlan(storage)).toEqual({ plan, damaged: false });
  });

  it('reports damaged storage without throwing, and leaves it alone', () => {
    const storage = new MemoryStorage();
    storage.setItem(STORAGE_KEY, '{broken');
    expect(loadPlan(storage)).toEqual({ plan: null, damaged: true });
    expect(storage.getItem(STORAGE_KEY)).toBe('{broken');
  });

  it('copes with no storage at all, and with writes that fail', () => {
    expect(loadPlan(null)).toEqual({ plan: null, damaged: false });
    expect(savePlan(null, newPlan(clone(philly)))).toBe(false);
    const full = new MemoryStorage();
    full.failWrites = true;
    expect(savePlan(full, newPlan(clone(philly)))).toBe(false);
  });

  it('stores display preferences separately, and stores nothing for the defaults', () => {
    const storage = new MemoryStorage();
    savePrefs(storage, { theme: 'dark', expert: true });
    expect(loadPrefs(storage)).toEqual({ theme: 'dark', expert: true });
    savePrefs(storage, DEFAULT_PREFS);
    expect(storage.getItem(PREFS_KEY)).toBeNull();
    storage.setItem(PREFS_KEY, '{"theme":"neon"}');
    expect(loadPrefs(storage)).toEqual(DEFAULT_PREFS);
  });

  it('forgets everything it wrote, and only that', () => {
    const storage = new MemoryStorage();
    savePlan(storage, newPlan(clone(philly)));
    savePrefs(storage, { theme: 'dark', expert: false });
    storage.setItem('someone-else', 'keep me');
    expect(forgetEverything(storage)).toBe(2);
    expect(storage.getItem(STORAGE_KEY)).toBeNull();
    expect(storage.getItem(PREFS_KEY)).toBeNull();
    expect(storage.getItem('someone-else')).toBe('keep me');
  });
});
