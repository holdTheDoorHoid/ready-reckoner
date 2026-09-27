import { describe, expect, it } from 'vitest';

import type { HazardProfile, PlanOutput } from '../../engine/types';
import { emptyMapsState, type MapsState } from './state';
import { hasChildren, homePoint, type MapLocation, MAP_HEIGHT, MAP_WIDTH, slotFrame, suggestedLayers } from './slots';
import { distanceM, frameBox, inFrame, maxTilesFor, tilesFor } from './tiles';

const PHILLY: MapLocation = {
  county_fips: '42101',
  county_name: 'Philadelphia',
  state_abbr: 'PA',
  state_name: 'Pennsylvania',
  centroid: { lat: 40.0094, lon: -75.1333 },
};
const PHILLY_BOX: [number, number, number, number] = [-75.276, 39.866, -74.963, 40.138];
const FALLS_CHURCH_BOX: [number, number, number, number] = [-77.195, 38.873, -77.15, 38.899];
const HOME = { lat: 39.935, lon: -75.155 };

function withPins(extra: Partial<MapsState> = {}): MapsState {
  return { ...emptyMapsState(), home: HOME, ...extra };
}

describe('Where the maps centre', () => {
  it('uses the home pin, else the ZIP code middle, else the county middle', () => {
    expect(homePoint(withPins(), PHILLY)).toEqual({ at: HOME, basis: 'pin' });
    expect(homePoint(emptyMapsState(), { ...PHILLY, zip_centroid: { lat: 39.9364, lon: -75.1534 } })).toEqual({ at: { lat: 39.9364, lon: -75.1534 }, basis: 'zip' });
    expect(homePoint(undefined, PHILLY)).toEqual({ at: PHILLY.centroid, basis: 'county' });
  });
});

describe('The three frames (§9.2)', () => {
  it('draws the neighbourhood about 1.5 km across: zoom 16 in Philadelphia, zoom 15 in Anchorage', () => {
    const f = slotFrame('neighbourhood', withPins(), PHILLY, PHILLY_BOX);
    expect(f.zoom).toBe(16);
    expect(f.center).toEqual(HOME);
    const box = frameBox(f);
    const across = distanceM({ lat: HOME.lat, lon: box[0] }, { lat: HOME.lat, lon: box[2] });
    expect(Math.abs(across - 1500)).toBeLessThan(100);
    expect(slotFrame('neighbourhood', { ...emptyMapsState(), home: { lat: 61.2181, lon: -149.9003 } }, PHILLY).zoom).toBe(15);
  });

  it('draws the area at zoom 11 when the county is too big for zoom 12, and 12 when it fits', () => {
    expect(slotFrame('area', withPins(), PHILLY, PHILLY_BOX).zoom).toBe(11);
    const fc = { lat: 38.886, lon: -77.172 };
    expect(slotFrame('area', { ...emptyMapsState(), home: fc }, PHILLY, FALLS_CHURCH_BOX).zoom).toBe(12);
    expect(slotFrame('area', withPins(), PHILLY, null).zoom).toBe(11);
  });

  it('draws the region around home alone at zoom 9, and fits home, the destination and the routes', () => {
    expect(slotFrame('region', withPins(), PHILLY).zoom).toBe(9);
    const lancaster = { lat: 40.0379, lon: -76.3055 };
    const route = [HOME, { lat: 40.0, lon: -75.6 }, lancaster];
    const f = slotFrame('region', withPins({ where_go: lancaster, routes: [route] }), PHILLY);
    expect(f.zoom).toBeGreaterThanOrEqual(8);
    expect(f.zoom).toBeLessThanOrEqual(10);
    for (const p of [HOME, lancaster, ...route]) expect(inFrame(f, p, 40)).toBe(true);
  });

  it('keeps a far destination on the region map by zooming out as far as 6', () => {
    const chicago = { lat: 41.8781, lon: -87.6298 };
    const f = slotFrame('region', withPins({ where_go: chicago }), PHILLY);
    expect(f.zoom).toBe(6);
    expect(inFrame(f, chicago)).toBe(true);
  });

  it('needs at most 60 tiles for all three maps: the budget in §9.3 is 250', () => {
    const maps = withPins({ where_go: { lat: 40.0379, lon: -76.3055 } });
    const total = (['neighbourhood', 'area', 'region'] as const).reduce((n, k) => n + tilesFor(slotFrame(k, maps, PHILLY, PHILLY_BOX)).length, 0);
    expect(total).toBeLessThanOrEqual(3 * maxTilesFor(MAP_WIDTH, MAP_HEIGHT));
    expect(3 * maxTilesFor(MAP_WIDTH, MAP_HEIGHT)).toBe(60);
  });
});

function hazard(id: HazardProfile['id'], rate: number, display: HazardProfile['display'] = 'ranked'): HazardProfile {
  return { id, display, rate_per_year: rate, annual_probability: 1 - Math.exp(-rate) } as HazardProfile;
}

function output(register: HazardProfile[], surgeShare?: number): Pick<PlanOutput, 'register' | 'location'> {
  return {
    register,
    location: { ...PHILLY, exposure: surgeShare === undefined ? undefined : { surge_cat3_share: { value: surgeShare, source: 'x' } } } as unknown as PlanOutput['location'],
  };
}

describe('Which layers are suggested (§9.2)', () => {
  it('suggests flood zones when a flood hazard reaches 1 in 100 over the horizon (Philadelphia: river flooding, about 6 in 100 over ten years)', () => {
    // The Philadelphia golden: riverine_flooding 0.00606 a year, coastal 0.00026, hurricane ranked.
    const s = suggestedLayers(output([hazard('riverine_flooding', 0.0060604736544738105), hazard('coastal_flooding', 0.00026329588014981277), hazard('hurricane', 0.04642905402987227)]), 10);
    expect(s).toEqual({ flood: true, surge: true, wildfire: false });
  });

  it('does not suggest flood zones below 1 in 100 over the horizon, and counts only ranked hazards', () => {
    expect(suggestedLayers(output([hazard('riverine_flooding', 0.0009)]), 10).flood).toBe(false);
    expect(suggestedLayers(output([hazard('riverine_flooding', 0.0009)]), 30).flood).toBe(true);
    expect(suggestedLayers(output([hazard('wildfire', 0.5, 'rare_catastrophic')]), 10).wildfire).toBe(false);
  });

  it('suggests wildfire hazard at 1 in 100 or more, and surge for a home in a surge area even without hurricanes ranked', () => {
    expect(suggestedLayers(output([hazard('wildfire', 0.002)]), 10).wildfire).toBe(true);
    expect(suggestedLayers(output([], 0.12), 10).surge).toBe(true);
    expect(suggestedLayers(output([], 0), 10).surge).toBe(false);
    expect(suggestedLayers(null, 10)).toEqual({ flood: false, surge: false, wildfire: false });
  });

  it('shows schools and child care only for a household with children', () => {
    expect(hasChildren([{ age_band: 'adult' }, { age_band: 'teen' }])).toBe(true);
    expect(hasChildren([{ age_band: 'adult' }, { age_band: 'senior' }])).toBe(false);
    expect(hasChildren(undefined)).toBe(false);
  });
});
