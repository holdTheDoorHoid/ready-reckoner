import { describe, expect, it } from 'vitest';

import { layoutMarkers, toneBase } from './draw';
import { censusExportUrl, censusTileUrl, classCounts, classifyRedBlue, floodExportUrl, maskOf, tileUrl, wildfireExportUrl } from './overlays';
import { CENSUS_BASE, FLOOD, MAP_ORIGINS, WILDFIRE } from './sources';
import { type Frame, frameMercatorBox } from './tiles';

const frame: Frame = { center: { lat: 39.935, lon: -75.155 }, zoom: 16, width: 800, height: 560 };

describe('The addresses asked for', () => {
  it('builds the OpenStreetMap tile address exactly as the policy gives it', () => {
    expect(tileUrl({ z: 15, x: 9542, y: 12410 })).toBe('https://tile.openstreetmap.org/15/9542/12410.png');
  });

  it('asks FEMA for layer 28 only, in two exact colours, for exactly the frame', () => {
    const url = new URL(floodExportUrl(frame));
    expect(`${url.origin}${url.pathname}`).toBe(FLOOD.url);
    const p = url.searchParams;
    expect(p.get('bbox')).toBe(frameMercatorBox(frame).map((v) => v.toFixed(2)).join(','));
    expect(p.get('size')).toBe('800,560');
    expect(p.get('bboxSR')).toBe('3857');
    expect(p.get('imageSR')).toBe('3857');
    expect(p.get('format')).toBe('png32');
    expect(p.get('transparent')).toBe('true');
    expect(p.get('f')).toBe('image');
    const layers = JSON.parse(p.get('dynamicLayers') ?? '[]');
    expect(layers).toHaveLength(1);
    expect(layers[0].source).toEqual({ type: 'mapLayer', mapLayerId: 28 });
    expect(layers[0].definitionExpression).toBe("SFHA_TF = 'T' OR ZONE_SUBTY LIKE '0.2 P%'");
    const infos = layers[0].drawingInfo.renderer.uniqueValueInfos;
    expect(infos.map((i: { value: string; symbol: { color: number[] } }) => [i.value, i.symbol.color])).toEqual([
      ['T', [255, 0, 0, 255]],
      ['F', [0, 0, 255, 255]],
    ]);
  });

  it('asks the Forest Service for the high and very high classes only, nearest-neighbour, on transparency', () => {
    const url = new URL(wildfireExportUrl(frame));
    expect(`${url.origin}${url.pathname}`).toBe(WILDFIRE.url);
    expect(url.pathname.endsWith('/ImageServer/exportImage')).toBe(true);
    expect(JSON.parse(url.searchParams.get('renderingRule') ?? '{}')).toEqual({
      rasterFunction: 'Colormap',
      rasterFunctionArguments: { Colormap: [[4, 255, 0, 0], [5, 0, 0, 255]] },
      variableName: 'Raster',
    });
    expect(url.searchParams.get('interpolation')).toBe('RSP_NearestNeighbor');
    expect(url.searchParams.get('transparent')).toBe('true');
  });

  it('asks the Census Bureau for roads, rail and water for exactly the frame, or one tile', () => {
    const url = new URL(censusExportUrl(frame));
    expect(`${url.origin}${url.pathname}`).toBe(CENSUS_BASE.url);
    expect(url.searchParams.get('layers')).toBe(CENSUS_BASE.layersNearby);
    expect(new URL(censusExportUrl({ ...frame, zoom: 11 })).searchParams.get('layers')).toBe(CENSUS_BASE.layers);
    const tile = new URL(censusTileUrl({ z: 1, x: 0, y: 0 }));
    const [x0, y0, x1, y1] = (tile.searchParams.get('bbox') ?? '').split(',').map(Number);
    expect(x0).toBeCloseTo(-20037508.34, 1);
    expect(y1).toBeCloseTo(20037508.34, 1);
    expect(x1).toBeCloseTo(0, 1);
    expect(y0).toBeCloseTo(0, 1);
    expect(tile.searchParams.get('size')).toBe('256,256');
  });

  it('only ever builds addresses on origins the content security policy allows', () => {
    for (const url of [tileUrl({ z: 1, x: 0, y: 0 }), floodExportUrl(frame), wildfireExportUrl(frame), censusExportUrl(frame), censusTileUrl({ z: 3, x: 1, y: 2 })]) {
      expect(MAP_ORIGINS).toContain(new URL(url).origin);
    }
  });
});

describe('Reading an overlay back', () => {
  it('sorts pixels into red, blue and nothing, including the blended edges the server draws', () => {
    const px = [
      255, 0, 0, 255, // red: class 1
      0, 0, 255, 255, // blue: class 2
      128, 0, 128, 255, // an edge between the two: class 1
      64, 0, 191, 255, // mostly blue edge: class 2
      0, 0, 255, 128, // half-transparent blue edge: class 2
      0, 0, 255, 60, // nearly transparent: nothing
      0, 0, 0, 0, // transparent
      200, 200, 200, 255, // grey, never asked for: nothing
    ];
    const classes = classifyRedBlue(px);
    expect([...classes]).toEqual([1, 2, 1, 2, 2, 0, 0, 0]);
    expect(classCounts(classes)).toEqual([3, 2, 3]);
  });

  it('makes a white, opaque mask of one class', () => {
    const mask = maskOf(new Uint8Array([0, 1, 2]), 1);
    expect([...mask]).toEqual([0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0]);
  });
});

describe('Drawing helpers', () => {
  it('greys the base map with a gamma of 1.5: white stays white, mid-tones darken', () => {
    const px = new Uint8ClampedArray([255, 255, 255, 255, 242, 239, 233, 255, 170, 211, 223, 255, 0, 0, 0, 255]);
    toneBase(px);
    expect([...px.slice(0, 4)]).toEqual([255, 255, 255, 255]);
    // OSM land (#f2efe9, luminance 239) becomes 231; water (#aad3df, 203) becomes 181; black stays black.
    expect(px[4]).toBe(231);
    expect(px[5]).toBe(231);
    expect(px[8]).toBe(181);
    expect([...px.slice(12, 16)]).toEqual([0, 0, 0, 255]);
  });

  it('moves a marker that would cover another, with a leader back to its point, and never moves fixed ones', () => {
    const placed = layoutMarkers(
      [
        { x: 100, y: 100, fixed: true },
        { x: 104, y: 100 },
        { x: 300, y: 300 },
      ],
      11,
      800,
      560,
    );
    expect(placed[0]).toEqual({ x: 100, y: 100, at: { x: 100, y: 100 }, nudged: false });
    expect(placed[1]!.nudged).toBe(true);
    expect(Math.hypot(placed[1]!.x - 100, placed[1]!.y - 100)).toBeGreaterThanOrEqual(24);
    expect(placed[1]!.at).toEqual({ x: 104, y: 100 });
    expect(placed[2]!.nudged).toBe(false);
  });

  it('keeps nudged markers on the canvas', () => {
    const placed = layoutMarkers(
      [
        { x: 5, y: 5, fixed: true },
        { x: 6, y: 6 },
      ],
      11,
      800,
      560,
    );
    expect(placed[1]!.x).toBeGreaterThanOrEqual(11);
    expect(placed[1]!.y).toBeGreaterThanOrEqual(11);
  });
});
