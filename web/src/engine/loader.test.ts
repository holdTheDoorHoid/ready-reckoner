import { describe, expect, it } from 'vitest';

import type { RawEngine } from './index';
import { coreLoadOrder, type Manifest, startupFiles, ZIP_FILES, zipFiles } from './data-files';
import { PackLoader, type LoaderStatus } from './loader';
import { adaptRawEngine } from './wasm';

const MANIFEST: Manifest = {
  pack_version: 'abc123',
  packs: {
    core: {
      files: [
        { path: 'core/base_rates.toml', bytes: 10 },
        { path: 'core/counties.csv', bytes: 200 },
        { path: 'core/nri_hazards.csv', bytes: 3000 },
        { path: 'core/zip_county.csv', bytes: 50 },
        { path: 'core/zip_centroids.csv', bytes: 40 },
        { path: 'core/zip_facilities.csv', bytes: 30 },
      ],
    },
    geo: { files: [{ path: 'geo/counties.json', bytes: 40 }] },
  },
};

const MAP = { type: 'FeatureCollection', features: [] };

/** A site: path (under /base/) -> body, status and type. Records every request. */
function site(files: Record<string, { body: string; status?: number; type?: string }>) {
  const requested: string[] = [];
  const get = async (url: string): Promise<Response> => {
    requested.push(url);
    const path = url.replace(/^\/base\//, '').replace(/\?.*$/, '');
    const file = files[path];
    if (!file) return new Response('not found', { status: 404 });
    return new Response(file.body, { status: file.status ?? 200, headers: { 'content-type': file.type ?? 'text/plain' } });
  };
  return { get, requested, files };
}

const fullSite = () =>
  site({
    'data/manifest.json': { body: JSON.stringify(MANIFEST), type: 'application/json' },
    'data/core/base_rates.toml': { body: 'rates' },
    'data/core/counties.csv': { body: 'counties' },
    'data/core/nri_hazards.csv': { body: 'nri' },
    'data/core/zip_county.csv': { body: 'zc' },
    'data/core/zip_centroids.csv': { body: 'zp' },
    'data/core/zip_facilities.csv': { body: 'zf' },
    'data/geo/counties.json': { body: JSON.stringify(MAP), type: 'application/json' },
  });

/** A string-level engine that records every load_pack call and can refuse one file. */
function recordingEngine(refuse?: string) {
  const loaded: { name: string; text: string }[] = [];
  const reply = () => '';
  const raw: RawEngine = {
    engine_info: reply,
    county_search: reply,
    resolve_location: reply,
    assess: reply,
    explain: reply,
    catalogue: reply,
    defaults: reply,
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

describe('which files load when', () => {
  it('puts the county list last, and keeps the ZIP tables out of the start-up load', () => {
    expect(coreLoadOrder(MANIFEST)).toEqual([
      'core/base_rates.toml',
      'core/nri_hazards.csv',
      'core/zip_county.csv',
      'core/zip_centroids.csv',
      'core/zip_facilities.csv',
      'core/counties.csv',
    ]);
    expect(startupFiles(MANIFEST)).toEqual(['core/base_rates.toml', 'core/nri_hazards.csv', 'core/counties.csv']);
    expect(zipFiles(MANIFEST)).toEqual([...ZIP_FILES]);
    expect(coreLoadOrder({ packs: {} })).toEqual([]);
  });
});

describe('the pack loader', () => {
  it('loads the manifest first, then the core files without the ZIP tables, versioned, from the same origin', async () => {
    const { get, requested } = fullSite();
    const { engine, loaded } = recordingEngine();
    const loader = new PackLoader(engine, '/base/', { fetch: get });
    const seen: LoaderStatus[] = [];
    loader.subscribe((s) => seen.push(s));
    await loader.core();
    expect(loaded.map((l) => l.name)).toEqual(['manifest.json', 'core/base_rates.toml', 'core/nri_hazards.csv', 'core/counties.csv']);
    expect(loaded[0]!.text).toBe(JSON.stringify(MANIFEST));
    expect(requested[0]).toBe('/base/data/manifest.json');
    expect(requested.slice(1).sort()).toEqual([
      '/base/data/core/base_rates.toml?v=abc123',
      '/base/data/core/counties.csv?v=abc123',
      '/base/data/core/nri_hazards.csv?v=abc123',
    ]);
    // Neither the ZIP tables nor the map are needed to plan for a county.
    expect(requested.some((u) => u.includes('zip_') || u.includes('geo/'))).toBe(false);
    // Progress is counted in the manifest's sizes and ends complete.
    expect(seen.map((s) => s.core.phase)).toContain('loading');
    expect(loader.status.core).toMatchObject({ phase: 'ready', bytesLoaded: 3210, bytesTotal: 3210 });
    expect(loader.status.packVersion).toBe('abc123');
    expect(loader.status.zip.phase).toBe('idle');
    expect(loader.isReady('core')).toBe(true);
    expect(loader.isReady('zip')).toBe(false);
    // A second call is the same load, not another one.
    await loader.core();
    expect(loaded.length).toBe(4);
  });

  it('loads the ZIP tables when asked, after the core files, and the map on its own', async () => {
    const { get, requested } = fullSite();
    const { engine, loaded } = recordingEngine();
    const loader = new PackLoader(engine, '/base/', { fetch: get });
    // Asking for the ZIP tables first still hands the core files over before them.
    await loader.zip();
    expect(loaded.map((l) => l.name)).toEqual([
      'manifest.json',
      'core/base_rates.toml',
      'core/nri_hazards.csv',
      'core/counties.csv',
      'core/zip_county.csv',
      'core/zip_centroids.csv',
      'core/zip_facilities.csv',
    ]);
    expect(loader.status.zip).toMatchObject({ phase: 'ready', bytesLoaded: 120, bytesTotal: 120 });
    expect(await loader.map()).toEqual(MAP);
    expect(loaded.at(-1)!.name).toBe('geo/counties.json');
    expect(requested.filter((u) => u.includes('manifest')).length).toBe(1);
    expect(loader.fetched).toContain('/base/data/geo/counties.json?v=abc123');
  });

  it('resolves at once and loads nothing when the site has no data', async () => {
    for (const noData of [site({}), site({ 'data/manifest.json': { body: '<!doctype html><title>app</title>', type: 'text/html' } })]) {
      const { engine, loaded } = recordingEngine();
      const loader = new PackLoader(engine, '/base/', { fetch: noData.get });
      await loader.core();
      await loader.zip();
      expect(await loader.map()).toBeNull();
      expect(loaded).toEqual([]);
      expect(loader.status.core.phase).toBe('none');
      expect(loader.isReady('core') && loader.isReady('zip')).toBe(true);
    }
  });

  it('fails with a plain reason when the manifest or a file cannot be had, and tries again on retry()', async () => {
    const broken = site({ 'data/manifest.json': { body: 'oops', status: 500 } });
    const a = new PackLoader(recordingEngine().engine, '/base/', { fetch: broken.get });
    await expect(a.core()).rejects.toThrow(/manifest did not download \(HTTP 500\)/);
    expect(a.status.core.phase).toBe('failed');
    expect(a.status.core.error).toMatch(/HTTP 500/);

    const notJson = site({ 'data/manifest.json': { body: '{ nope', type: 'application/json' } });
    await expect(new PackLoader(recordingEngine().engine, '/base/', { fetch: notJson.get }).core()).rejects.toThrow(/not JSON/);

    const offline = new PackLoader(recordingEngine().engine, '/base/', { fetch: () => Promise.reject(new TypeError('Failed to fetch')) });
    await expect(offline.core()).rejects.toThrow('The county data could not be downloaded. Check your internet connection.');

    // A file missing, then back: retry() loads it without starting over.
    const s = fullSite();
    const missing = s.files['data/core/nri_hazards.csv']!;
    delete s.files['data/core/nri_hazards.csv'];
    const { engine, loaded } = recordingEngine();
    const loader = new PackLoader(engine, '/base/', { fetch: s.get });
    await expect(loader.core()).rejects.toThrow('The data file core/nri_hazards.csv did not download (HTTP 404).');
    expect(loader.isReady('core')).toBe(false);
    s.files['data/core/nri_hazards.csv'] = missing;
    await loader.core().catch(() => undefined);
    expect(loader.status.core.phase).toBe('failed');
    loader.retry();
    expect(loader.status.core.phase).toBe('idle');
    await loader.core();
    expect(loader.status.core.phase).toBe('ready');
    expect(loaded.filter((l) => l.name === 'manifest.json').length).toBe(1);
  });

  it('stops with the engine’s own message when it refuses a file', async () => {
    const { engine, loaded } = recordingEngine('core/nri_hazards.csv');
    const loader = new PackLoader(engine, '/base/', { fetch: fullSite().get });
    await expect(loader.core()).rejects.toThrow('The data file core/nri_hazards.csv does not match its checksum in the manifest.');
    expect(loaded.map((l) => l.name)).toEqual(['manifest.json', 'core/base_rates.toml', 'core/nri_hazards.csv']);
    // What depends on the core files fails with it.
    await expect(loader.zip()).rejects.toThrow(/checksum/);
  });
});
