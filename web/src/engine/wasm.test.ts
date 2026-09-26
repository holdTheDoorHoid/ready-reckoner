import { describe, expect, it } from 'vitest';

import type { Engine, RawEngine } from './index';
import type { Manifest } from './data-files';
import { FIXTURE_NAMES, FIXTURES } from './fixtures';
import { PackLoader } from './loader';
import { createMockEngine, mockDefaults } from './mock';
import type { Envelope, PlanInput } from './types';
import { adaptRawEngine, gateEngine } from './wasm';

/**
 * A string-level engine shaped exactly like the rr-wasm exports (JSON in, JSON envelope out),
 * answering from a table of prepared replies, so the adapter can be tested before the
 * WebAssembly build exists.
 */
function rawEngine(replies: Map<string, string>): RawEngine {
  const reply = (f: string, arg = '') => replies.get(`${f}:${arg}`) ?? '';
  return {
    engine_info: () => reply('engine_info'),
    load_pack: (name) => reply('load_pack', name),
    county_search: (query) => reply('county_search', query),
    resolve_location: (json) => reply('resolve_location', json),
    assess: (json) => reply('assess', json),
    explain: (json) => reply('explain', json),
    catalogue: () => reply('catalogue'),
    defaults: () => reply('defaults'),
  };
}

describe('the WebAssembly adapter', () => {
  it('passes JSON through unchanged, so the wasm and mock engines are interchangeable', async () => {
    const mock = createMockEngine();
    const replies = new Map<string, string>();
    for (const name of FIXTURE_NAMES) {
      replies.set(`assess:${JSON.stringify(FIXTURES[name])}`, JSON.stringify(await mock.assess(FIXTURES[name])));
    }
    replies.set('catalogue:', JSON.stringify(await mock.catalogue()));
    replies.set('defaults:', JSON.stringify(await mock.defaults()));
    const adapted = adaptRawEngine(rawEngine(replies));
    for (const name of FIXTURE_NAMES) {
      expect(await adapted.assess(FIXTURES[name]), name).toEqual(await mock.assess(FIXTURES[name]));
    }
    expect(await adapted.catalogue()).toEqual(await mock.catalogue());
    const d = await adapted.defaults();
    expect(d.ok && d.value).toEqual(mockDefaults());
  });

  it('turns unreadable output, or a trap, into an "internal" error instead of throwing', async () => {
    const broken: RawEngine = {
      ...rawEngine(new Map([['engine_info:', 'not json']])),
      load_pack: () => {
        throw new Error('unreachable executed');
      },
    };
    const adapted = adaptRawEngine(broken);
    const info = await adapted.engine_info();
    expect(!info.ok && info.error.code).toBe('internal');
    const pack = await adapted.load_pack('core', new Uint8Array([1]));
    expect(!pack.ok && pack.error.code).toBe('internal');
    expect((await adapted.assess(FIXTURES['philadelphia-renters-4'])).ok).toBe(false);
  });
});

// ---------------------------------------------------------------------------------------------
// Waiting for data
// ---------------------------------------------------------------------------------------------

const MANIFEST: Manifest = {
  pack_version: 'v1',
  packs: {
    core: {
      files: [
        { path: 'core/counties.csv', bytes: 10 },
        { path: 'core/zip_county.csv', bytes: 10 },
        { path: 'core/zip_centroids.csv', bytes: 10 },
        { path: 'core/zip_facilities.csv', bytes: 10 },
      ],
    },
  },
};

/** A site whose files arrive only when `release(path)` is called (or at once when `open`). */
function slowSite(open = false) {
  const waiting = new Map<string, () => void>();
  const requested: string[] = [];
  let fail = false;
  const get = (url: string): Promise<Response> => {
    requested.push(url);
    const path = url.replace(/^\/s\/data\//, '').replace(/\?.*$/, '');
    if (fail) return Promise.reject(new TypeError('Failed to fetch'));
    const body = path === 'manifest.json' ? JSON.stringify(MANIFEST) : path;
    const respond = () => new Response(body, { headers: { 'content-type': path.endsWith('.json') ? 'application/json' : 'text/plain' } });
    if (open || path === 'manifest.json') return Promise.resolve(respond());
    return new Promise((resolve) => waiting.set(path, () => resolve(respond())));
  };
  return {
    get,
    requested,
    release: (path: string) => waiting.get(path)?.(),
    releaseAll: () => [...waiting.values()].forEach((r) => r()),
    breakNetwork: () => (fail = true),
  };
}

/** An engine that answers bad_input for an input with no people, otherwise ok, and records calls. */
function answeringEngine() {
  const calls: string[] = [];
  const envelope = (value: unknown) => JSON.stringify({ ok: true, value });
  const raw: RawEngine = {
    engine_info: () => envelope({}),
    load_pack: (name) => {
      calls.push(`load ${name}`);
      return envelope({ name, version: 'v1', rows: 1 });
    },
    county_search: (q) => {
      calls.push(`search ${q}`);
      return envelope([]);
    },
    resolve_location: (json) => {
      calls.push(`resolve ${JSON.parse(json).zip ?? JSON.parse(json).county_fips}`);
      return envelope({});
    },
    assess: (json) => {
      const input = JSON.parse(json) as PlanInput;
      calls.push(`assess ${input.location.zip ?? input.location.county_fips}`);
      if (input.people.length === 0) {
        return JSON.stringify({ ok: false, error: { code: 'bad_input', message: 'Some answers need another look.', details: { problems: [] } } });
      }
      return envelope({ plan: true });
    },
    explain: () => envelope({}),
    catalogue: () => envelope({ items: [] }),
    defaults: () => envelope({}),
  };
  return { engine: adaptRawEngine(raw), calls };
}

const tick = () => new Promise((r) => setTimeout(r, 0));
const withPlace = (location: PlanInput['location'], people = FIXTURES['philadelphia-renters-4'].people): PlanInput => ({
  ...FIXTURES['philadelphia-renters-4'],
  location,
  people,
});

describe('calls that need data wait for it', () => {
  function setup(open = false) {
    const s = slowSite(open);
    const { engine, calls } = answeringEngine();
    const loader = new PackLoader(engine, '/s/', { fetch: s.get });
    const gated: Engine = gateEngine(engine, loader);
    return { s, calls, loader, gated };
  }

  it('answers a county plan once the county data is in, without fetching the ZIP tables', async () => {
    const { s, calls, gated, loader } = setup();
    let answer: Envelope<unknown> | undefined;
    void gated.assess(withPlace({ country: 'US', county_fips: '42101', setting: 'urban' })).then((r) => (answer = r));
    await tick();
    expect(answer).toBeUndefined();
    s.release('core/counties.csv');
    await loader.core();
    await tick();
    expect(answer).toEqual({ ok: true, value: { plan: true } });
    expect(s.requested.some((u) => u.includes('zip_'))).toBe(false);
    // The probe while loading plus the real call once the data was in.
    expect(calls.filter((c) => c.startsWith('assess')).length).toBe(2);
  });

  it('makes a ZIP code wait for the ZIP tables, and starts fetching them', async () => {
    const { s, gated, loader } = setup();
    let answer: Envelope<unknown> | undefined;
    void gated.resolve_location({ country: 'US', zip: '19147', setting: 'urban' }).then((r) => (answer = r));
    await tick();
    s.release('core/counties.csv');
    await loader.core();
    await tick();
    expect(answer).toBeUndefined();
    expect(s.requested.filter((u) => u.includes('zip_')).length).toBe(3);
    s.releaseAll();
    await loader.zip();
    await tick();
    expect(answer?.ok).toBe(true);
  });

  it('never fetches the ZIP tables for the new-plan placeholder ZIP code', async () => {
    const { s, gated } = setup(true);
    await gated.assess(withPlace({ country: 'US', zip: '00000', setting: 'urban' }));
    expect(s.requested.some((u) => u.includes('zip_'))).toBe(false);
  });

  it('reports a problem with the answers at once, without waiting for data', async () => {
    const { gated } = setup();
    const answer = await gated.assess(withPlace({ country: 'US', zip: '19147', setting: 'urban' }, []));
    expect(!answer.ok && answer.error.code).toBe('bad_input');
  });

  it('answers pack_missing with the reason when the data cannot be had', async () => {
    const { s, gated } = setup();
    s.breakNetwork();
    const answer = await gated.county_search('phila');
    expect(answer.ok).toBe(false);
    expect(!answer.ok && answer.error).toEqual({ code: 'pack_missing', message: 'The county data could not be downloaded. Check your internet connection.' });
  });

  it('passes the catalogue and the defaults straight through', async () => {
    const { gated, s } = setup();
    expect(await gated.catalogue()).toEqual({ ok: true, value: { items: [] } });
    expect(s.requested.filter((u) => !u.includes('manifest')).length).toBe(0);
  });
});
