import { describe, expect, it } from 'vitest';

import { DATA_CACHE, precacheList, precacheName, staleCaches, strategyFor } from './sw-core';

describe('service worker rules', () => {
  it('precaches the built app but not itself, source maps, dotfiles or data packs', () => {
    expect(
      precacheList([
        'index.html',
        'sw.js',
        'assets/index-abc.js',
        'assets/index-abc.js.map',
        'data/core/counties.csv',
        'data/manifest.json',
        'icons/icon-192.png',
        '.well-known/x',
        'pkg/rr_wasm_bg.wasm',
      ]),
    ).toEqual(['assets/index-abc.js', 'icons/icon-192.png', 'index.html', 'pkg/rr_wasm_bg.wasm']);
  });

  it('answers each request the right way, under a Pages base path', () => {
    const scope = new URL('https://user.github.io/ready-reckoner/');
    const precached = new Set(['index.html', 'assets/app.js']);
    const at = (path: string, mode = 'cors') => strategyFor(new URL(path, 'https://user.github.io'), scope, precached, mode);
    expect(at('/ready-reckoner/assets/app.js')).toBe('precache');
    expect(at('/ready-reckoner/data/core/counties.csv')).toBe('data');
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
});
