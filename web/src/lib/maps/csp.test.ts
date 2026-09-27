import { describe, expect, it } from 'vitest';

import { CSP } from '../../../vite-plugins/csp';
import { strategyFor } from '../../pwa/sw-core';
import { OVERPASS, CENSUS_BASE, FLOOD, MAP_ORIGINS, NOMINATIM, OSM_TILES, RECIPIENTS, SEARCH_RECIPIENT, SURGE, WILDFIRE } from './sources';

const directive = (name: string) =>
  CSP.split(';')
    .map((d) => d.trim().split(/\s+/))
    .find(([n]) => n === name)
    ?.slice(1) ?? [];

describe('The content security policy lets the maps reach exactly the services in sources.ts', () => {
  it('lists the seven map origins, no more and no fewer', () => {
    expect([...MAP_ORIGINS].sort()).toEqual([
      'https://hazards.fema.gov',
      'https://imagery.geoplatform.gov',
      'https://nominatim.openstreetmap.org',
      'https://overpass-api.de',
      'https://overpass.kumi.systems',
      'https://tigerweb.geo.census.gov',
      'https://tile.openstreetmap.org',
    ]);
    // Storm surge is not a source (no stable NOAA image service), so NOAA is not an origin.
    expect(SURGE).toBeNull();
    expect(MAP_ORIGINS.some((o) => o.includes('noaa') || o.includes('arcgis.com'))).toBe(false);
  });

  it('puts exactly those origins in connect-src and img-src, and nowhere else', () => {
    expect(directive('connect-src')).toEqual(["'self'", ...MAP_ORIGINS]);
    expect(directive('img-src')).toEqual(["'self'", 'data:', 'blob:', ...MAP_ORIGINS]);
    for (const other of ['default-src', 'script-src', 'style-src', 'font-src', 'worker-src', 'manifest-src']) {
      expect(directive(other).some((v) => v.startsWith('https://')), other).toBe(false);
    }
    expect(directive('script-src')).toEqual(["'self'", "'wasm-unsafe-eval'"]);
  });

  it('builds every endpoint on a listed origin, and names every origin on the consent screen or the address warning', () => {
    for (const url of [OSM_TILES.url.replace(/\{[xyz]\}/g, '1'), CENSUS_BASE.url, ...OVERPASS.urls, FLOOD.url, WILDFIRE.url, NOMINATIM.url]) {
      expect(MAP_ORIGINS).toContain(new URL(url).origin);
    }
    const named = new Set([...RECIPIENTS.flatMap((r) => r.origins), ...SEARCH_RECIPIENT.origins]);
    expect([...named].sort()).toEqual([...MAP_ORIGINS].sort());
  });
});

describe('The service worker never caches a map request', () => {
  const scope = new URL('https://holdthedoorhoid.github.io/ready-reckoner/');
  const precached = new Set(['index.html', 'assets/leaflet-view-abc.js']);

  it('leaves every map service to the network (the browser’s own HTTP cache), for images and fetches alike', () => {
    for (const origin of MAP_ORIGINS) {
      for (const mode of ['cors', 'no-cors', 'navigate']) {
        expect(strategyFor(new URL(`${origin}/15/1/2.png`), scope, precached, mode), `${origin} ${mode}`).toBe('network');
      }
    }
  });

  it('serves the lazy pin-map chunk from the precache, so the panel opens offline', () => {
    expect(strategyFor(new URL('assets/leaflet-view-abc.js', scope), scope, precached, 'cors')).toBe('precache');
  });
});
