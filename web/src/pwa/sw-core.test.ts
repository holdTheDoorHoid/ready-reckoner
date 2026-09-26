import { describe, expect, it } from 'vitest';

import { startupFiles, ZIP_FILES } from '../engine/data-files';
import { DATA_CACHE, dataPrecacheList, precacheList, precacheName, staleCaches, staleDataEntries, strategyFor } from './sw-core';

describe('service worker rules', () => {
  it('precaches the built app and the data manifest, but not itself, source maps, dotfiles or other data', () => {
    expect(
      precacheList([
        'index.html',
        'sw.js',
        'assets/index-abc.js',
        'assets/index-abc.js.map',
        'data/core/counties.csv',
        'data/manifest.json',
        'data/geo/counties.json',
        'icons/icon-192.png',
        '.well-known/x',
        'pkg/rr_wasm_bg.wasm',
      ]),
    ).toEqual(['assets/index-abc.js', 'data/manifest.json', 'icons/icon-192.png', 'index.html', 'pkg/rr_wasm_bg.wasm']);
  });

  it('precaches the start-up data by the versioned address the loader asks for, without the ZIP tables or the map', () => {
    const manifest = {
      pack_version: 'e8b8 cd',
      packs: {
        core: { files: ['core/counties.csv', 'core/nri_hazards.csv', ...ZIP_FILES].map((path) => ({ path })) },
        geo: { files: [{ path: 'geo/counties.json' }] },
      },
    };
    expect(dataPrecacheList(startupFiles(manifest), 'e8b8 cd')).toEqual(['data/core/nri_hazards.csv?v=e8b8%20cd', 'data/core/counties.csv?v=e8b8%20cd']);
    expect(dataPrecacheList(['core/a.csv'], '')).toEqual(['data/core/a.csv']);
  });

  it('answers each request the right way, under a Pages base path', () => {
    const scope = new URL('https://user.github.io/ready-reckoner/');
    const precached = new Set(['index.html', 'assets/app.js', 'data/manifest.json']);
    const at = (path: string, mode = 'cors') => strategyFor(new URL(path, 'https://user.github.io'), scope, precached, mode);
    expect(at('/ready-reckoner/assets/app.js')).toBe('precache');
    expect(at('/ready-reckoner/data/manifest.json')).toBe('precache');
    expect(at('/ready-reckoner/data/core/counties.csv?v=abc')).toBe('data');
    expect(at('/ready-reckoner/data/geo/counties.json?v=abc')).toBe('data');
    expect(at('/ready-reckoner/', 'navigate')).toBe('navigate');
    expect(at('/ready-reckoner/assets/unknown.js')).toBe('network');
    expect(at('/another-project/index.html')).toBe('network');
    expect(strategyFor(new URL('https://example.org/ready-reckoner/assets/app.js'), scope, precached, 'cors')).toBe('network');
  });

  it("deletes only this app's old caches when a new version activates", () => {
    const current = precacheName('new');
    expect(staleCaches([precacheName('old'), current, DATA_CACHE, 'other-app-cache', 'rr-precache-older'], current)).toEqual([
      precacheName('old'),
      'rr-precache-older',
    ]);
  });

  it('drops cached data from other data versions, keeping this version and unversioned files', () => {
    const base = 'https://user.github.io/ready-reckoner/data/';
    expect(
      staleDataEntries([`${base}core/a.csv?v=new`, `${base}core/a.csv?v=old`, `${base}geo/counties.json?v=old`, `${base}geo/counties.json`], 'new'),
    ).toEqual([`${base}core/a.csv?v=old`, `${base}geo/counties.json?v=old`]);
  });
});
