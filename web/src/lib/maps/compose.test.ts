import { describe, expect, it } from 'vitest';

import { type ComposeInput, composeMaps, COUNTY_CREDIT, framesFor, HOSPITAL_NOTE, longDate, plannedRequests } from './compose';
import fixture from './fixtures/overpass-philadelphia.json';
import { MAP_SLOT_FIXTURES, type MapLocation } from './slots';
import { MAP_ORIGINS, OSM_TILES, SURGE_NOTE } from './sources';
import { emptyMapsState, type MapsState } from './state';
import { fakeEnv, imageResponse, jsonResponse, overlayPixels, type Route } from './testing';
import { tilesFor } from './tiles';

const HOME = { lat: 39.9364, lon: -75.1534 };
const LANCASTER = { lat: 40.0379, lon: -76.3055 };
const PHILLY: MapLocation = { county_fips: '42101', county_name: 'Philadelphia', state_abbr: 'PA', state_name: 'Pennsylvania', centroid: { lat: 40.0094, lon: -75.1333 } };
const OUTLINE = {
  rings: [
    [
      [-75.276, 39.866],
      [-74.963, 39.866],
      [-74.963, 40.138],
      [-75.276, 40.138],
      [-75.276, 39.866],
    ] as [number, number][],
  ],
  bbox: [-75.276, 39.866, -74.963, 40.138] as [number, number, number, number],
};

function household(extra: Partial<MapsState> = {}): MapsState {
  return {
    ...emptyMapsState(),
    home: HOME,
    meeting_near: { lat: 39.9375, lon: -75.15 },
    meeting_far: { lat: 39.9522, lon: -75.1932 },
    where_go: LANCASTER,
    routes: [
      [HOME, { lat: 39.955, lon: -75.19 }, { lat: 40.02, lon: -75.33 }, LANCASTER],
      [HOME, { lat: 39.9, lon: -75.25 }, { lat: 39.96, lon: -75.9 }, LANCASTER],
    ],
    ...extra,
  };
}

function input(extra: Partial<ComposeInput> = {}): ComposeInput {
  return {
    maps: household(),
    location: PHILLY,
    county: OUTLINE,
    layers: { base: true, places: true, flood: true, surge: false, wildfire: false },
    suggested: { flood: true, surge: true, wildfire: false },
    children: true,
    today: '2026-10-01',
    ...extra,
  };
}

/** Everything answers: tiles, places, a flood export with some high-risk zone in it, wildfire, the Census map. */
const allGood: Route = (url) => {
  const host = new URL(url).host;
  if (host === 'tile.openstreetmap.org') return imageResponse('tile');
  if (host === 'overpass-api.de' || host === 'overpass.kumi.systems') return jsonResponse(fixture);
  if (host === 'hazards.fema.gov') return imageResponse('flood');
  if (host === 'imagery.geoplatform.gov') return imageResponse('wildfire');
  if (host === 'tigerweb.geo.census.gov') return imageResponse('census');
  return new Error(`unexpected request to ${url}`);
};

const pixels = (tag: string, w: number, h: number) =>
  tag === 'flood' ? overlayPixels(w, h, 5000, 0) : tag === 'wildfire' ? overlayPixels(w, h, 0, 800) : new Uint8ClampedArray(w * h * 4);

const origin = (url: string) => new URL(url).origin;

describe('Composing the three maps for a Philadelphia household', () => {
  it('asks only the recipients the household agreed to, within the limits, and says how many', async () => {
    const { env, requests } = fakeEnv(allGood, pixels);
    const result = await composeMaps(input(), env);
    const frames = framesFor(input());
    const expectedTiles = tilesFor(frames.neighbourhood).length + tilesFor(frames.area).length + tilesFor(frames.region).length;
    expect(result.tiles).toBe(expectedTiles);
    expect(result.tiles).toBeLessThanOrEqual(60);
    expect(result.succeeded).toBe(expectedTiles + 2);
    expect(result.requests).toEqual({
      'https://tile.openstreetmap.org': expectedTiles,
      'https://overpass-api.de': 1,
      'https://hazards.fema.gov': 1,
    });
    // Every request goes to an origin in sources.ts, with no cookies.
    for (const r of requests) {
      expect(MAP_ORIGINS).toContain(origin(r.url));
      expect(r.init.credentials).toBe('omit');
      expect(r.init.mode).toBe('cors');
      expect(r.init.cache).toBe('default');
    }
  });

  it('sends the site’s address only to the tile server (its policy requires a Referer), and no one else', async () => {
    const { env, requests } = fakeEnv(allGood, pixels);
    await composeMaps(input({ layers: { base: true, places: true, flood: true, surge: false, wildfire: true } }), env);
    for (const r of requests) {
      const expected = origin(r.url) === 'https://tile.openstreetmap.org' ? 'strict-origin-when-cross-origin' : 'no-referrer';
      expect(r.init.referrerPolicy, r.url).toBe(expected);
    }
    const overpass = requests.find((r) => r.url.includes('overpass'));
    expect(overpass?.init.method).toBe('POST');
    expect(String(overpass?.init.body)).toMatch(/^data=%5Bout%3Ajson%5D%5Btimeout%3A25%5D/);
  });

  it('draws every tile of each map on its canvas and encodes a JPEG at quality 0.8', async () => {
    const { env, canvases } = fakeEnv(allGood, pixels);
    const result = await composeMaps(input(), env);
    const frames = framesFor(input());
    const mains = canvases.filter((c) => c.width === 800 && c.height === 560 && c.encoded);
    expect(mains).toHaveLength(3);
    mains.forEach((c, i) => {
      const slot = (['neighbourhood', 'area', 'region'] as const)[i]!;
      expect(c.ctx.ops.filter((op) => op.startsWith('drawImage tile'))).toHaveLength(tilesFor(frames[slot]).length);
      expect(c.encoded).toEqual({ type: 'image/jpeg', quality: 0.8 });
      // The base map is greyed: its pixels are read back and written once.
      expect(c.ctx.ops.filter((op) => op === 'putImageData')).toHaveLength(1);
      // The OpenStreetMap credit is drawn on the image itself.
      expect(c.ctx.texts).toContain(OSM_TILES.credit);
    });
    for (const map of result.maps) {
      expect(map.image.startsWith('data:image/jpeg;base64,')).toBe(true);
      expect(map.bytes).toBe(3000);
      expect(map.width).toBe(800);
      expect(map.fetched_on).toBe('2026-10-01');
    }
    expect(result.maps.map((m) => m.slot)).toEqual(MAP_SLOT_FIXTURES.map((s) => s.kind));
  });

  it('numbers the legend: the household’s own points, then the places nearest first', async () => {
    const { env } = fakeEnv(allGood, pixels);
    const [nb, area, region] = (await composeMaps(input(), env)).maps;
    expect(nb!.legend.slice(0, 3)).toEqual([
      { mark: 'H', name: 'Home', kind: 'Your plan', own: true },
      { mark: 'M1', name: 'Meeting place near home', kind: 'Your plan', own: true },
      { mark: '1', name: 'Rite Aid', kind: 'Pharmacy', address: '704 East Passyunk Avenue', phone: '+1 215-627-3151' },
    ]);
    expect(nb!.legend.map((r) => r.mark)).toEqual(['H', 'M1', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
    expect(nb!.legend.find((r) => r.mark === '3')?.name).toBe('Pharmacy (no name listed)');
    expect(area!.legend[1]).toEqual({ mark: 'M2', name: 'Meeting place outside the neighbourhood', kind: 'Your plan', own: true });
    expect(area!.legend.filter((r) => r.kind === 'Hospital with an emergency room').map((r) => r.name)).toContain('Pennsylvania Hospital');
    expect(area!.notes).toContain(HOSPITAL_NOTE);
    expect(region!.legend).toEqual([
      { mark: 'H', name: 'Home', kind: 'Your plan', own: true },
      { mark: 'D', name: 'Where we would go', kind: 'Your plan', own: true },
    ]);
  });

  it('hatches the flood zones, draws the county line and both ways out, and credits every source with its date', async () => {
    const { env } = fakeEnv(allGood, pixels);
    const [nb, area, region] = (await composeMaps(input(), env)).maps;
    expect(nb!.keys).toEqual([{ pattern: 'stripes', text: 'FEMA high-risk flood zone: about a 1 in 100 chance of flooding each year.' }]);
    expect(nb!.credits).toEqual([
      'Map data © OpenStreetMap contributors, openstreetmap.org/copyright',
      'Flood zones: FEMA National Flood Hazard Layer, retrieved October 1, 2026',
      'Places © OpenStreetMap contributors, openstreetmap.org/copyright, retrieved October 1, 2026',
    ]);
    expect(area!.keys.map((k) => k.pattern)).toEqual(['county', 'route-1', 'route-2']);
    expect(area!.credits).toContain(COUNTY_CREDIT);
    expect(region!.keys.map((k) => k.pattern)).toEqual(['county', 'route-1', 'route-2']);
    expect(region!.scale).toMatch(/^The bars show .+ mi and .+ km\.$/);
  });

  it('says on the neighbourhood and area maps that storm surge is not drawn, and points to the state’s zone page', async () => {
    const { env } = fakeEnv(allGood, pixels);
    const zoneLink = { name: 'Know Your Zone', url: 'https://example.org/know-your-zone' };
    const [nb, area, region] = (await composeMaps(input({ zoneLink }), env)).maps;
    for (const map of [nb!, area!]) {
      const surge = map.statuses.find((s) => s.layer === 'surge');
      expect(surge?.state).toBe('unavailable');
      expect(surge?.text).toBe(`${SURGE_NOTE} Find your evacuation zone: Know Your Zone, https://example.org/know-your-zone`);
    }
    expect(region!.statuses.some((s) => s.layer === 'surge')).toBe(false);
  });

  it('reports progress up to the planned number of requests', async () => {
    const { env, progress } = fakeEnv(allGood, pixels);
    await composeMaps(input(), env);
    const total = plannedRequests(input());
    expect(progress.at(-1)).toEqual([total, total]);
  });
});

describe('When a source fails, its layer is left out and the map says so', () => {
  it('flood zones fail: the neighbourhood map says so, with the date; nothing else changes', async () => {
    const { env } = fakeEnv((url, init) => (url.includes('hazards.fema.gov') ? new Error('network down') : allGood(url, init)), pixels);
    const [nb, area] = (await composeMaps(input(), env)).maps;
    expect(nb!.statuses).toContainEqual({ layer: 'flood', state: 'failed', text: 'Flood zones could not be fetched on October 1, 2026.' });
    expect(nb!.keys).toEqual([]);
    expect(nb!.credits.some((c) => c.includes('FEMA'))).toBe(false);
    expect(area!.legend.length).toBeGreaterThan(2);
  });

  it('a flood export with no zone in it says there is none, rather than failing', async () => {
    const { env } = fakeEnv(allGood, () => new Uint8ClampedArray(800 * 560 * 4));
    const [nb] = (await composeMaps(input(), env)).maps;
    expect(nb!.statuses).toContainEqual({ layer: 'flood', state: 'none', text: 'No FEMA flood zone is mapped in this area.' });
  });

  it('a busy Overpass server: the second instance answers, and that is the last query of the press', async () => {
    const { env } = fakeEnv((url, init) => (url.startsWith('https://overpass-api.de') ? jsonResponse({ remark: 'busy' }, 504) : allGood(url, init)), pixels);
    const result = await composeMaps(input(), env);
    expect(result.requests['https://overpass-api.de']).toBe(1);
    expect(result.requests['https://overpass.kumi.systems']).toBe(1);
    expect(result.maps[0]!.legend.length).toBe(11);
  });

  it('both Overpass servers fail (one times out inside a 200 answer): no third try, and the maps say so', async () => {
    const { env } = fakeEnv((url, init) => {
      if (url.startsWith('https://overpass-api.de')) return jsonResponse({ elements: [], remark: 'runtime error: Query timed out in "query" at line 3 after 25 seconds.' });
      if (url.startsWith('https://overpass.kumi.systems')) return new Error('offline');
      return allGood(url, init);
    }, pixels);
    const result = await composeMaps(input(), env);
    expect((result.requests['https://overpass-api.de'] ?? 0) + (result.requests['https://overpass.kumi.systems'] ?? 0)).toBe(2);
    const [nb, area] = result.maps;
    for (const map of [nb!, area!]) {
      expect(map.statuses).toContainEqual({ layer: 'places', state: 'failed', text: 'Nearby places could not be fetched on October 1, 2026.' });
      expect(map.legend.every((r) => r.own)).toBe(true);
    }
  });

  it('the tile server refuses: no more tile requests after the first refusals, and each map uses the Census map instead', async () => {
    const { env } = fakeEnv((url, init) => (url.startsWith('https://tile.openstreetmap.org') ? imageResponse('refused', 429) : allGood(url, init)), pixels);
    const result = await composeMaps(input(), env);
    expect(result.requests['https://tile.openstreetmap.org']).toBeLessThanOrEqual(4);
    expect(result.requests['https://tigerweb.geo.census.gov']).toBe(3);
    for (const map of result.maps) {
      expect(map.statuses).toContainEqual({ layer: 'base', state: 'fallback', text: 'The OpenStreetMap map did not answer, so this map uses the U.S. Census Bureau’s street map.' });
      expect(map.credits[0]).toBe('Base map: U.S. Census Bureau TIGERweb, retrieved October 1, 2026 (public domain)');
    }
  });

  it('the street map fails everywhere: the maps still come, with markers and the county line, saying so', async () => {
    const { env } = fakeEnv((url, init) => (/tile\.openstreetmap|tigerweb/.test(url) ? new Error('offline') : allGood(url, init)), pixels);
    const result = await composeMaps(input(), env);
    for (const map of result.maps) expect(map.statuses).toContainEqual({ layer: 'base', state: 'failed', text: 'The street map could not be fetched on October 1, 2026.' });
    expect(result.succeeded).toBe(2);
    // Nothing at all answers: a failed press.
    const dead = fakeEnv(() => new Error('offline'), pixels);
    expect((await composeMaps(input(), dead.env)).succeeded).toBe(0);
    expect(result.maps[1]!.keys.some((k) => k.pattern === 'county')).toBe(true);
  });
});

describe('What the household chose', () => {
  it('no street map and no places: nothing is asked of those recipients, and the maps say what was left off', async () => {
    const { env, requests } = fakeEnv(allGood, pixels);
    const result = await composeMaps(input({ layers: { base: false, places: false, flood: false, surge: false, wildfire: false } }), env);
    expect(requests).toEqual([]);
    const [nb] = result.maps;
    expect(nb!.statuses).toEqual([
      { layer: 'base', state: 'off', text: 'No street map: you left it off.' },
      { layer: 'flood', state: 'off', text: 'Flood zones were left off this map.' },
      { layer: 'surge', state: 'unavailable', text: `${SURGE_NOTE} Your state or county emergency office has the evacuation-zone map.` },
      { layer: 'places', state: 'off', text: 'Nearby places were left off.' },
    ]);
  });

  it('wildfire hazard: very high cross-hatched on the area map, credited with its date', async () => {
    const { env, requests } = fakeEnv(allGood, pixels);
    const result = await composeMaps(input({ layers: { base: true, places: true, flood: false, surge: false, wildfire: true }, suggested: { flood: false, surge: false, wildfire: true } }), env);
    expect(requests.filter((r) => r.url.includes('imagery.geoplatform.gov'))).toHaveLength(1);
    const area = result.maps[1]!;
    expect(area.keys[0]).toEqual({ pattern: 'cross', text: 'Very high wildfire hazard (U.S. Forest Service).' });
    expect(area.credits).toContain('Wildfire hazard: USDA Forest Service Wildfire Hazard Potential (2023), retrieved October 1, 2026');
  });

  it('without a home pin: centred on the ZIP code, the home not marked, and the map says so', async () => {
    const { env } = fakeEnv(allGood, pixels);
    const result = await composeMaps(input({ maps: { ...emptyMapsState() }, location: { ...PHILLY, zip_centroid: HOME } }), env);
    const [nb] = result.maps;
    expect(nb!.legend.some((r) => r.mark === 'H')).toBe(false);
    expect(nb!.notes[0]).toBe('No home pin was placed, so this map is centred on the middle of your ZIP code and your home is not marked.');
  });

  it('a meeting place beyond the edge of its map is listed, not drawn', async () => {
    const { env } = fakeEnv(allGood, pixels);
    const result = await composeMaps(input({ maps: household({ meeting_near: { lat: 40.1, lon: -75.0 } }) }), env);
    expect(result.maps[0]!.legend[1]).toEqual({ mark: 'M1', name: 'Meeting place near home', kind: 'Your plan', own: true, offMap: true });
  });
});

describe('Dates', () => {
  it('writes the fetch date as people read it', () => {
    expect(longDate('2026-10-01')).toBe('October 1, 2026');
  });
});
