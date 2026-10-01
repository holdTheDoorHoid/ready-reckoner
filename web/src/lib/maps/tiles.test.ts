import { describe, expect, it } from 'vitest';

import {
  distanceM,
  exportScale,
  fitFrame,
  type Frame,
  frameBox,
  frameMercatorBox,
  fromCanvas,
  maxTilesFor,
  metresPerPixel,
  project,
  scaleBar,
  tileCorner,
  tileOf,
  tilesFor,
  toCanvas,
  toMercator,
  unproject,
} from './tiles';

// Reference values computed separately (Python, the OSM wiki's "Slippy map tilenames" formulas).
const CITY_HALL = { lat: 39.9526, lon: -75.1652 };

describe('Web Mercator: points, tiles and metres', () => {
  it('puts the equator and prime meridian in the middle of the world', () => {
    expect(project({ lat: 0, lon: 0 }, 0)).toEqual({ x: 128, y: 128 });
    expect(project({ lat: 0, lon: -180 }, 1)).toEqual({ x: 0, y: 256 });
  });

  it('finds the tile Philadelphia City Hall is on (z15: 9542, 12410)', () => {
    expect(tileOf(CITY_HALL, 15)).toEqual({ x: 9542, y: 12410, z: 15 });
    const px = project(CITY_HALL, 15);
    expect(px.x).toBeCloseTo(2442827.894, 2);
    expect(px.y).toBeCloseTo(3177193.474, 2);
  });

  it('round-trips a point through world pixels at every zoom', () => {
    for (const z of [0, 5, 11, 16, 19]) {
      const back = unproject(project(CITY_HALL, z), z);
      expect(back.lat).toBeCloseTo(CITY_HALL.lat, 9);
      expect(back.lon).toBeCloseTo(CITY_HALL.lon, 9);
    }
  });

  it('gives each tile corner as a point', () => {
    const c = tileCorner(9542, 12410, 15);
    expect(c.lon).toBeLessThanOrEqual(CITY_HALL.lon);
    expect(c.lat).toBeGreaterThanOrEqual(CITY_HALL.lat);
    expect(tileCorner(0, 0, 0)).toEqual({ lat: expect.closeTo(85.0511, 3), lon: -180 });
  });

  it('knows the ground size of a pixel (1.83 m at latitude 40, zoom 16)', () => {
    expect(metresPerPixel(0, 0)).toBeCloseTo(156543.034, 2);
    expect(metresPerPixel(40, 16)).toBeCloseTo(1.8298175, 6);
  });

  it('converts to Web Mercator metres (EPSG:3857)', () => {
    const m = toMercator({ lat: 39.88, lon: -75.265 });
    expect(m.x).toBeCloseTo(-8378461.5, 0);
    expect(m.y).toBeCloseTo(4848519.5, 0);
  });

  it('measures great-circle distance (City Hall to Independence Hall is about 1.4 km)', () => {
    expect(distanceM(CITY_HALL, { lat: 39.9489, lon: -75.15 })).toBeGreaterThan(1300);
    expect(distanceM(CITY_HALL, { lat: 39.9489, lon: -75.15 })).toBeLessThan(1450);
  });
});

describe('Frames: what a map covers', () => {
  const frame: Frame = { center: { lat: 39.935, lon: -75.155 }, zoom: 16, width: 800, height: 560 };

  it('puts the centre in the middle of the canvas and maps canvas pixels back to points', () => {
    const c = toCanvas(frame, frame.center);
    expect(c.x).toBeCloseTo(400, 0);
    expect(c.y).toBeCloseTo(280, 0);
    const p = fromCanvas(frame, { x: 0, y: 0 });
    const box = frameBox(frame);
    expect(p.lon).toBeCloseTo(box[0], 9);
    expect(p.lat).toBeCloseTo(box[3], 9);
  });

  it('covers about 1.46 km across at zoom 16 in Philadelphia', () => {
    const box = frameBox(frame);
    const width = distanceM({ lat: frame.center.lat, lon: box[0] }, { lat: frame.center.lat, lon: box[2] });
    expect(width).toBeGreaterThan(1440);
    expect(width).toBeLessThan(1490);
  });

  it('gives the export box in metres, exactly the canvas at 96 dpi', () => {
    const [x0, y0, x1, y1] = frameMercatorBox(frame);
    const mpp = (2 * Math.PI * 6378137) / (256 * 2 ** 16);
    expect((x1 - x0) / mpp).toBeCloseTo(800, 6);
    expect((y1 - y0) / mpp).toBeCloseTo(560, 6);
    // FEMA's flood zones draw at 1:36,112 and closer: zoom 15 and 16 qualify, zoom 14 does not.
    expect(exportScale(frame)).toBeCloseTo(9027.98, 1);
    expect(exportScale({ ...frame, zoom: 15 })).toBeLessThan(36111.9);
    expect(exportScale({ ...frame, zoom: 14 })).toBeGreaterThan(36111.9);
  });

  it('lists each tile once with its canvas position, and never more than 20 for an 800×560 map', () => {
    const tiles = tilesFor(frame);
    expect(tiles.length).toBeGreaterThanOrEqual(12);
    expect(tiles.length).toBeLessThanOrEqual(20);
    expect(new Set(tiles.map((t) => `${t.x}/${t.y}`)).size).toBe(tiles.length);
    // Tiles tile the canvas: the first starts at or left of 0, the last ends at or past the edge.
    expect(tiles[0]!.left).toBeLessThanOrEqual(0);
    expect(tiles[0]!.top).toBeLessThanOrEqual(0);
    const last = tiles[tiles.length - 1]!;
    expect(last.left + 256).toBeGreaterThanOrEqual(800);
    expect(last.top + 256).toBeGreaterThanOrEqual(560);
    expect(maxTilesFor(800, 560)).toBe(20);
  });

  it('counts exactly 4 × 2 tiles when the frame sits on tile boundaries, and one more row and column when it does not', () => {
    // Centred on a tile corner, a 1024 × 512 canvas starts 2 tiles left and 1 tile up: whole tiles only.
    const corner = tileCorner(1193, 1551, 12);
    const aligned: Frame = { center: corner, zoom: 12, width: 1024, height: 512 };
    const tiles = tilesFor(aligned);
    expect(tiles.length).toBe(8);
    expect(tiles.map((t) => [t.x, t.y, t.left, t.top])).toContainEqual([1191, 1550, 0, 0]);
    // A 768 px height centred on the corner starts half a tile down: four rows.
    expect(tilesFor({ ...aligned, height: 768 }).length).toBe(16);
  });

  it('wraps tile columns across the 180th meridian', () => {
    const tiles = tilesFor({ center: { lat: 52, lon: 179.99 }, zoom: 3, width: 512, height: 256 });
    const xs = new Set(tiles.map((t) => t.x));
    expect(xs.has(7)).toBe(true);
    expect(xs.has(0)).toBe(true);
    expect([...xs].every((x) => x >= 0 && x < 8)).toBe(true);
  });

  it('fits several points at the highest zoom that holds them all', () => {
    const home = { lat: 39.935, lon: -75.155 };
    const lancaster = { lat: 40.0379, lon: -76.3055 };
    const f = fitFrame([home, lancaster], 800, 560, 56, 6, 10);
    expect(f.zoom).toBe(9);
    for (const p of [home, lancaster]) {
      const c = toCanvas(f, p);
      expect(c.x).toBeGreaterThanOrEqual(56 - 1);
      expect(c.x).toBeLessThanOrEqual(800 - 56 + 1);
    }
    const pittsburgh = { lat: 40.4406, lon: -79.9959 };
    expect(fitFrame([home, pittsburgh], 800, 560, 56, 6, 10).zoom).toBe(7);
    expect(fitFrame([home], 800, 560, 56, 6, 10).zoom).toBe(10);
  });
});

describe('Scale bar', () => {
  const frame: Frame = { center: { lat: 40, lon: -75 }, zoom: 16, width: 800, height: 560 };

  it('picks a round metric length that fits (200 m in 160 px at zoom 16, latitude 40)', () => {
    const bar = scaleBar(frame, 160, 'metric');
    expect(bar.label).toBe('200 m');
    expect(bar.pixels).toBeCloseTo(200 / 1.8298175, 3);
  });

  it('picks feet for short US bars and miles once a quarter mile fits', () => {
    expect(scaleBar(frame, 160, 'us').label).toBe('500 ft');
    expect(scaleBar({ ...frame, zoom: 12 }, 160, 'us').label).toBe('2 mi');
    expect(scaleBar({ ...frame, zoom: 14 }, 160, 'us').label).toBe('½ mi');
    expect(scaleBar({ ...frame, zoom: 9 }, 160, 'metric').label).toBe('20 km');
  });

  it('never draws a bar longer than it was given', () => {
    for (const z of [6, 9, 11, 12, 15, 16]) {
      for (const system of ['metric', 'us'] as const) expect(scaleBar({ ...frame, zoom: z }, 150, system).pixels).toBeLessThanOrEqual(150);
    }
  });
});
