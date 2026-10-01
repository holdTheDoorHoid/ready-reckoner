import { describe, expect, it } from 'vitest';

import { checkLatLon, checkMapsState, emptyMapsState, hasPins, MAX_ROUTE_POINTS, MAX_ROUTES, mapsHoldLocation } from './state';

describe('MapsState (DESIGN-DELTA-v3 §9.5)', () => {
  it('starts with no pins, no routes, and places on', () => {
    const s = emptyMapsState();
    expect(s).toEqual({ routes: [], layers: { places: true, flood: false, surge: false, wildfire: false } });
    expect(hasPins(s)).toBe(false);
    expect(mapsHoldLocation(s)).toBe(false);
  });

  it('keeps real points, rounded to about a metre, and drops anything else', () => {
    expect(checkLatLon({ lat: 39.9526123456, lon: -75.1652219 })).toEqual({ lat: 39.95261, lon: -75.16522 });
    expect(checkLatLon({ lat: 91, lon: 0 })).toBeUndefined();
    expect(checkLatLon({ lat: 0, lon: -181 })).toBeUndefined();
    expect(checkLatLon({ lat: '39.9', lon: -75 })).toBeUndefined();
    expect(checkLatLon({ lat: Number.NaN, lon: -75 })).toBeUndefined();
    expect(checkLatLon(null)).toBeUndefined();
    expect(checkLatLon([39.9, -75.1])).toBeUndefined();
  });

  it('reads a state written to the §9.5 shape, and drops anything else (an old base-map field)', () => {
    const s = checkMapsState({
      home: { lat: 39.95, lon: -75.16 },
      meeting_far: { lat: 40.0, lon: -75.2 },
      routes: [[{ lat: 39.95, lon: -75.16 }, { lat: 40.1, lon: -75.3 }]],
      layers: { base: false, places: true, flood: true, surge: false, wildfire: false },
      fetched_on: '2026-10-01',
    });
    expect(s).toEqual({
      home: { lat: 39.95, lon: -75.16 },
      meeting_far: { lat: 40, lon: -75.2 },
      routes: [[{ lat: 39.95, lon: -75.16 }, { lat: 40.1, lon: -75.3 }]],
      layers: { places: true, flood: true, surge: false, wildfire: false },
      fetched_on: '2026-10-01',
    });
    expect(hasPins(s)).toBe(true);
    expect(mapsHoldLocation(s)).toBe(true);
  });

  it('drops bad points, single-point routes, extra routes and extra points, and unknown keys', () => {
    const long = Array.from({ length: MAX_ROUTE_POINTS + 10 }, (_, i) => ({ lat: 40 + i / 1000, lon: -75 }));
    const s = checkMapsState({
      home: { lat: 'x', lon: 1 },
      where_go: { lat: 41, lon: -76 },
      routes: [long, [{ lat: 40, lon: -75 }], [{ lat: 40, lon: -75 }, { lat: 99, lon: 0 }], [{ lat: 1, lon: 1 }, { lat: 2, lon: 2 }], 'nonsense', [{ lat: 3, lon: 3 }, { lat: 4, lon: 4 }]],
      layers: { places: 'yes', flood: 1, surge: true },
      fetched_on: 'yesterday',
      password: 'never kept',
    });
    expect(s?.home).toBeUndefined();
    expect(s?.where_go).toEqual({ lat: 41, lon: -76 });
    expect(s?.routes).toHaveLength(MAX_ROUTES);
    expect(s?.routes[0]).toHaveLength(MAX_ROUTE_POINTS);
    expect(s?.routes[1]).toEqual([{ lat: 1, lon: 1 }, { lat: 2, lon: 2 }]);
    // A layer is on only when it is exactly true, except places (on by default), off only when exactly false.
    expect(s?.layers).toEqual({ places: true, flood: false, surge: true, wildfire: false });
    expect(s?.fetched_on).toBeUndefined();
    expect(Object.keys(s ?? {})).not.toContain('password');
  });

  it('is undefined for anything that is not an object', () => {
    expect(checkMapsState(undefined)).toBeUndefined();
    expect(checkMapsState('maps')).toBeUndefined();
    expect(checkMapsState([])).toBeUndefined();
  });

  it('counts a meeting place alone as pins, but not as where the household lives', () => {
    const s = { ...emptyMapsState(), meeting_near: { lat: 40, lon: -75 } };
    expect(hasPins(s)).toBe(true);
    expect(mapsHoldLocation(s)).toBe(false);
  });
});
