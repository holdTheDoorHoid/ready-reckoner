import { readFileSync, existsSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

import { areaView, indexShapes, MAP_HEIGHT, MAP_WIDTH, stateView } from './geo';

/** A tiny state of three square counties in a row, and one far-away county in another state. */
const square = (w: number, s: number, size = 1) => [
  [
    [w, s],
    [w + size, s],
    [w + size, s + size],
    [w, s + size],
    [w, s],
  ],
];
const feature = (id: string, state: string, w: number, s: number, size = 1) => ({
  type: 'Feature',
  id,
  properties: { name: `County ${id}`, state, lat: s + size / 2, lon: w + size / 2 },
  geometry: { type: 'Polygon', coordinates: square(w, s, size) },
});
const SAMPLE = {
  type: 'FeatureCollection',
  features: [
    feature('01001', 'AA', -80, 40),
    feature('01003', 'AA', -79, 40),
    feature('01005', 'AA', -78, 40, 0.05),
    feature('02001', 'BB', -100, 30),
    {
      type: 'Feature',
      id: '02016',
      properties: { name: 'Aleutians West', state: 'AK', lat: 52, lon: 179 },
      geometry: {
        type: 'MultiPolygon',
        coordinates: [
          [
            [
              [178, 51],
              [179.5, 51],
              [179.5, 52],
              [178, 51],
            ],
          ],
          [
            [
              [-179.5, 51],
              [-179, 51],
              [-179, 52],
              [-179.5, 51],
            ],
          ],
        ],
      },
    },
  ],
};

function numbers(d: string): number[] {
  return [...d.matchAll(/-?\d+(?:\.\d+)?/g)].map((m) => Number(m[0]));
}

describe('county shapes for the map', () => {
  const shapes = indexShapes(SAMPLE);

  it('indexes counties by FIPS and by state', () => {
    expect(shapes.byFips.size).toBe(5);
    expect(shapes.byState.get('AA')!.map((c) => c.fips)).toEqual(['01001', '01003', '01005']);
    expect(shapes.byFips.get('01003')!.bbox).toEqual([-79, 40, -78, 41]);
  });

  it("moves Alaska's eastern longitudes west, so the Aleutians do not span the globe", () => {
    const ak = shapes.byFips.get('02016')!;
    expect(ak.bbox[0]).toBe(-182);
    expect(ak.bbox[2]).toBe(-179);
    expect(ak.lon).toBe(-181);
  });

  it('draws a county inside its state, marked, within the drawing box', () => {
    const view = stateView(shapes, '01003')!;
    expect(view.paths.map((p) => [p.fips, p.role])).toEqual([
      ['01001', 'context'],
      ['01005', 'context'],
      ['01003', 'focus'],
    ]);
    for (const p of view.paths) {
      const ns = numbers(p.d);
      for (let i = 0; i < ns.length; i += 2) {
        expect(ns[i]).toBeGreaterThanOrEqual(0);
        expect(ns[i]).toBeLessThanOrEqual(MAP_WIDTH);
        expect(ns[i + 1]).toBeGreaterThanOrEqual(0);
        expect(ns[i + 1]).toBeLessThanOrEqual(MAP_HEIGHT);
      }
    }
    // North is up: the county's top edge (latitude 41) has the smaller y.
    const ys = numbers(view.paths[2]!.d).filter((_, i) => i % 2 === 1);
    expect(Math.min(...ys)).toBeLessThan(Math.max(...ys));
    // A full-size county needs no ring; a tiny one gets one.
    expect(view.markers).toEqual([]);
    const tiny = stateView(shapes, '01005')!;
    expect(tiny.markers).toHaveLength(1);
    expect(tiny.markers[0]!.label).toBeUndefined();
    expect(tiny.markers[0]!.r).toBeGreaterThanOrEqual(8);
  });

  it('shows the counties of an ambiguous ZIP code numbered in order, with their neighbours, the chosen one on top', () => {
    const view = areaView(shapes, ['01003', '01001'], '01001')!;
    expect(view.markers.map((m) => [m.fips, m.label])).toEqual([
      ['01003', '1'],
      ['01001', '2'],
    ]);
    const roles = view.paths.map((p) => [p.fips, p.role]);
    expect(roles.at(-1)).toEqual(['01001', 'focus']);
    expect(roles).toContainEqual(['01003', 'candidate']);
    expect(roles).toContainEqual(['01005', 'context']);
    // Far-away counties are left out.
    expect(roles.some(([f]) => f === '02001')).toBe(false);
  });

  it('says nothing for a county it has no outline for', () => {
    expect(stateView(shapes, '66010')).toBeNull();
    expect(areaView(shapes, ['66010'])).toBeNull();
  });

  const pack = `${process.cwd().replace(/\/web$/, '')}/data/geo/counties.json`;
  it.runIf(existsSync(pack))('draws Philadelphia inside Pennsylvania from the real pack', () => {
    const real = indexShapes(JSON.parse(readFileSync(pack, 'utf8')));
    expect(real.byFips.size).toBeGreaterThan(3200);
    const view = stateView(real, '42101')!;
    expect(view.paths.length).toBe(real.byState.get('PA')!.length);
    expect(view.paths.at(-1)).toMatchObject({ fips: '42101', role: 'focus' });
    // Philadelphia is small at state scale, so it is ringed.
    expect(view.markers).toHaveLength(1);
    // Every Alaska outline stays in the western hemisphere after wrapping.
    for (const c of real.byState.get('AK')!) expect(c.bbox[2]).toBeLessThan(-129);
  });
});
