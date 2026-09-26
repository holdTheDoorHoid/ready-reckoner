import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';

import type { Engine } from '../engine/index';
import { FIXTURES } from '../engine/fixtures';
import { PackLoader } from '../engine/loader';
import { createMockEngine } from '../engine/mock';
import { gateEngine } from '../engine/wasm';
import type { Envelope, PlanInput, PlanOutput } from '../engine/types';
import { MemoryStorage, savedFor, until } from '../test/helpers';
import { AppState } from './app.svelte';
import { PREFS_KEY, STORAGE_KEY } from './persistence';

/** The mock engine, counting assess calls and optionally holding answers back. */
function countingEngine(hold?: (input: PlanInput) => number): Engine & { calls: PlanInput[] } {
  const inner = createMockEngine();
  const calls: PlanInput[] = [];
  return {
    ...inner,
    calls,
    async assess(input: PlanInput): Promise<Envelope<PlanOutput>> {
      calls.push(input);
      const wait = hold?.(input) ?? 0;
      if (wait) await new Promise((r) => setTimeout(r, wait));
      return inner.assess(input);
    },
  };
}

async function started(plan = savedFor(FIXTURES['philadelphia-renters-4']), engine: Engine = createMockEngine(), delay = 0) {
  const storage = new MemoryStorage();
  storage.setItem(STORAGE_KEY, JSON.stringify(plan));
  const app = new AppState({ storage, engine, delay, saveDelay: 0, today: () => '2026-10-02' });
  await app.init();
  flushSync();
  await until(() => !!app.result.output || !!app.result.error, 'first assessment');
  return { app, storage };
}

describe('AppState', () => {
  it('loads the saved plan and assesses it', async () => {
    const { app } = await started();
    expect(app.plan?.input.location.zip).toBe('19147');
    expect(app.result.output?.location.county_name).toBe('Philadelphia County');
    expect(app.source?.kind).toBe('mock');
    app.destroy();
  });

  it('re-runs the assessment once for a burst of changes (100 ms pause)', async () => {
    const engine = countingEngine();
    const { app } = await started(undefined, engine, 100);
    const before = engine.calls.length;
    for (const rp of ['one_in_10', 'one_in_50', 'one_in_500'] as const) {
      app.plan!.input.dials.return_period = rp;
      flushSync();
      await new Promise((r) => setTimeout(r, 10));
    }
    await until(() => !app.pending && app.result.output?.buckets[0]?.target.kind === 'days', 'reassessment');
    await new Promise((r) => setTimeout(r, 150));
    expect(engine.calls.length - before).toBe(1);
    expect(engine.calls.at(-1)?.dials.return_period).toBe('one_in_500');
    app.destroy();
  });

  it('drops an answer that arrives after a newer one', async () => {
    // The first change is slow to answer; the second is quick. The quick one must win.
    const engine = countingEngine((input) => (input.dials.return_period === 'one_in_10' ? 80 : 0));
    const { app } = await started(undefined, engine, 0);
    app.plan!.input.dials.return_period = 'one_in_10';
    flushSync();
    await new Promise((r) => setTimeout(r, 5));
    app.plan!.input.dials.return_period = 'one_in_500';
    flushSync();
    await new Promise((r) => setTimeout(r, 150));
    const power = app.result.output!.buckets.find((b) => b.id === 'power')!.target;
    expect(power.kind === 'days' && power.value).toBe(10);
    app.destroy();
  });

  it('records a check-off with the date and what was paid, and the plan follows', async () => {
    const { app } = await started();
    const item = app.result.output!.plan.months.flatMap((m) => m.items).find((i) => i.kind === 'purchase' && !i.done)!;
    app.record(item);
    app.setPaid(item.item_id, item.tier, 12.5);
    flushSync();
    expect(app.plan!.purchases).toEqual([{ item_id: item.item_id, tier: item.tier, qty: item.quantity, date: '2026-10-02', paid_usd: 12.5 }]);
    await until(() => app.result.output!.plan.months.flatMap((m) => m.items).some((i) => i.item_id === item.item_id && i.done === true), 'the item to show as done');
    app.unrecord(item.item_id, item.tier);
    flushSync();
    expect(app.plan!.purchases).toEqual([]);
    app.destroy();
  });

  it('saves changes to storage, and forgets everything on request', async () => {
    const { app, storage } = await started();
    app.plan!.input.finances.monthly_budget_usd = 75;
    app.prefs.theme = 'dark';
    flushSync();
    await new Promise((r) => setTimeout(r, 20));
    expect(JSON.parse(storage.getItem(STORAGE_KEY)!).input.finances.monthly_budget_usd).toBe(75);
    expect(JSON.parse(storage.getItem(PREFS_KEY)!).theme).toBe('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    app.forget();
    flushSync();
    await new Promise((r) => setTimeout(r, 20));
    expect(storage.length).toBe(0);
    expect(app.plan).toBeNull();
    expect(app.result).toEqual({});
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
    app.destroy();
  });

  it('starts a new plan from the engine defaults, dated today', async () => {
    const app = new AppState({ storage: new MemoryStorage(), engine: createMockEngine(), delay: 0, saveDelay: 0, today: () => '2026-11-15' });
    await app.init();
    await app.startNew();
    expect(app.plan?.input.planning_date).toBe('2026-11-15');
    expect(app.plan?.input.location.zip).toBe('00000');
    flushSync();
    // The placeholder ZIP code is not a place: the plan asks for one instead of looking it up.
    await until(() => !!app.result.error, 'the plan to ask for a place');
    expect(app.result.error?.code).toBe('bad_input');
    expect((app.result.error?.details as { problems: { code: string; field: string }[] }).problems[0]).toMatchObject({ code: 'location_missing', field: 'location' });
    app.destroy();
  });

  it('keeps working when the browser allows no storage', async () => {
    const app = new AppState({ storage: null, engine: createMockEngine(), delay: 0, saveDelay: 0 });
    expect(app.storageAvailable).toBe(false);
    await app.init();
    await app.startNew();
    app.saveNow();
    expect(app.saveFailed).toBe(false);
    app.destroy();
  });
});

describe('AppState with the data loading behind it', () => {
  /** A site whose manifest and one core file download on request, or fail. */
  function site() {
    let failing = true;
    const manifest = JSON.stringify({ pack_version: 'v9', packs: { core: { files: [{ path: 'core/counties.csv', bytes: 10 }] } } });
    const get = async (url: string): Promise<Response> => {
      if (url.includes('manifest.json')) return new Response(manifest, { headers: { 'content-type': 'application/json' } });
      if (failing) throw new TypeError('Failed to fetch');
      return new Response('counties', { headers: { 'content-type': 'text/plain' } });
    };
    return { get, heal: () => (failing = false) };
  }

  it('is ready before the data, reports a failed download as pack_missing, and plans after "Try again"', async () => {
    const s = site();
    const mock = createMockEngine();
    const loader = new PackLoader(mock, '/s/', { fetch: s.get });
    const engine = gateEngine(mock, loader);
    const storage = new MemoryStorage();
    storage.setItem(STORAGE_KEY, JSON.stringify(savedFor(FIXTURES['philadelphia-renters-4'])));
    const app = new AppState({ storage, engine, loader, delay: 0, saveDelay: 0, today: () => '2026-10-02' });
    void loader.core().catch(() => undefined);
    await app.init();
    flushSync();
    expect(app.ready).toBe(true);
    await until(() => !!app.result.error, 'the failed data to be reported');
    expect(app.result.error?.code).toBe('pack_missing');
    expect(app.result.error?.message).toMatch(/could not be downloaded/);
    expect(app.data?.core.phase).toBe('failed');
    await app.dataSettled();

    s.heal();
    app.retryData();
    flushSync();
    await until(() => !!app.result.output, 'the plan after trying again', 5000);
    expect(app.data?.core.phase).toBe('ready');
    expect(app.data?.packVersion).toBe('v9');
    expect(app.manifest?.pack_version).toBe('v9');
    expect(app.dataUrlsFetched()).toContain('/s/data/core/counties.csv?v=v9');
    app.destroy();
  });
});
