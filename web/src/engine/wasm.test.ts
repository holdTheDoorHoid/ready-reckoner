import { describe, expect, it } from 'vitest';

import type { RawEngine } from './index';
import { FIXTURE_NAMES, FIXTURES } from './fixtures';
import { createMockEngine, mockDefaults } from './mock';
import type { Manifest } from './wasm';
import { adaptRawEngine, coreLoadOrder, loadCorePacks, loadDataFiles } from './wasm';

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
// Loading the data packs
// ---------------------------------------------------------------------------------------------

const MANIFEST: Manifest = {
  pack_version: 'abc123',
  packs: {
    core: {
      files: [
        { path: 'core/base_rates.toml', bytes: 10 },
        { path: 'core/counties.csv', bytes: 200 },
        { path: 'core/nri_hazards.csv', bytes: 3000 },
      ],
    },
    geo: { files: [{ path: 'geo/counties.json', bytes: 40 }] },
  },
};

/** A site: path (under the base) -> body and status. */
function site(files: Record<string, { body: string; status?: number; type?: string }>) {
  const requested: string[] = [];
  const get = async (url: string): Promise<Response> => {
    requested.push(url);
    const path = url.replace(/^\/base\//, '').replace(/\?.*$/, '');
    const file = files[path];
    if (!file) return new Response('not found', { status: 404 });
    return new Response(file.body, { status: file.status ?? 200, headers: { 'content-type': file.type ?? 'text/plain' } });
  };
  return { get, requested };
}

/** A string-level engine that records every load_pack call and can refuse one file. */
function recordingEngine(refuse?: string) {
  const loaded: { name: string; text: string }[] = [];
  const raw: RawEngine = {
    ...rawEngine(new Map()),
    load_pack: (name, bytes) => {
      loaded.push({ name, text: new TextDecoder().decode(bytes) });
      if (name === refuse) {
        return JSON.stringify({ ok: false, error: { code: 'pack_corrupt', message: `The data file ${name} does not match its checksum in the manifest.` } });
      }
      return JSON.stringify({ ok: true, value: { name, version: 'abc123', rows: 1 } });
    },
  };
  return { engine: adaptRawEngine(raw), loaded };
}

const fullSite = () =>
  site({
    'data/manifest.json': { body: JSON.stringify(MANIFEST), type: 'application/json' },
    'data/core/base_rates.toml': { body: 'rates' },
    'data/core/counties.csv': { body: 'counties' },
    'data/core/nri_hazards.csv': { body: 'nri' },
    'data/geo/counties.json': { body: 'map' },
  });

describe('loading the data packs', () => {
  it('orders the core pack with the county list last', () => {
    expect(coreLoadOrder(MANIFEST)).toEqual(['core/base_rates.toml', 'core/nri_hazards.csv', 'core/counties.csv']);
    expect(coreLoadOrder({ packs: {} })).toEqual([]);
  });

  it('loads the manifest first, then every core file, from the same origin with the pack version', async () => {
    const { get, requested } = fullSite();
    const { engine, loaded } = recordingEngine();
    const progress: number[] = [];
    const result = await loadCorePacks(engine, '/base/', { fetch: get, onProgress: (p) => progress.push(p.bytesLoaded) });
    expect(loaded.map((l) => l.name)).toEqual(['manifest.json', 'core/base_rates.toml', 'core/nri_hazards.csv', 'core/counties.csv']);
    expect(loaded[0]!.text).toBe(JSON.stringify(MANIFEST));
    expect(loaded[3]!.text).toBe('counties');
    expect(requested[0]).toBe('/base/data/manifest.json');
    expect(requested.slice(1).sort()).toEqual([
      '/base/data/core/base_rates.toml?v=abc123',
      '/base/data/core/counties.csv?v=abc123',
      '/base/data/core/nri_hazards.csv?v=abc123',
    ]);
    // The map is not needed to plan.
    expect(requested.some((u) => u.includes('geo/'))).toBe(false);
    expect(progress).toEqual([10, 3010, 3210]);
    expect(result).toEqual({ packVersion: 'abc123', files: ['manifest.json', 'core/base_rates.toml', 'core/nri_hazards.csv', 'core/counties.csv'] });
  });

  it('leaves the engine on its sample counties when the site has no data', async () => {
    for (const noData of [site({}), site({ 'data/manifest.json': { body: '<!doctype html><title>app</title>', type: 'text/html' } })]) {
      const { engine, loaded } = recordingEngine();
      expect(await loadCorePacks(engine, '/base/', { fetch: noData.get })).toBeNull();
      expect(loaded).toEqual([]);
    }
  });

  it('throws, naming the problem, when the manifest or a file cannot be had', async () => {
    const broken = site({ 'data/manifest.json': { body: 'oops', status: 500 } });
    await expect(loadCorePacks(recordingEngine().engine, '/base/', { fetch: broken.get })).rejects.toThrow(/manifest did not download \(HTTP 500\)/);

    const notJson = site({ 'data/manifest.json': { body: '{ nope', type: 'application/json' } });
    await expect(loadCorePacks(recordingEngine().engine, '/base/', { fetch: notJson.get })).rejects.toThrow(/not JSON/);

    const missingFile = site({
      'data/manifest.json': { body: JSON.stringify(MANIFEST), type: 'application/json' },
      'data/core/base_rates.toml': { body: 'rates' },
      'data/core/counties.csv': { body: 'counties' },
    });
    await expect(loadCorePacks(recordingEngine().engine, '/base/', { fetch: missingFile.get })).rejects.toThrow(
      'The data file core/nri_hazards.csv did not download (HTTP 404).',
    );
  });

  it('throws with the engine’s own message when it refuses a file, and stops there', async () => {
    const { engine, loaded } = recordingEngine('core/nri_hazards.csv');
    await expect(loadCorePacks(engine, '/base/', { fetch: fullSite().get })).rejects.toThrow(
      'The data file core/nri_hazards.csv does not match its checksum in the manifest.',
    );
    expect(loaded.map((l) => l.name)).toEqual(['manifest.json', 'core/base_rates.toml', 'core/nri_hazards.csv']);
  });

  it('loads other packs on demand, such as the map', async () => {
    const { get, requested } = fullSite();
    const { engine, loaded } = recordingEngine();
    await loadDataFiles(engine, '/base/', MANIFEST, ['geo/counties.json'], { fetch: get });
    expect(loaded.map((l) => l.name)).toEqual(['geo/counties.json']);
    expect(requested).toEqual(['/base/data/geo/counties.json?v=abc123']);
  });
});
