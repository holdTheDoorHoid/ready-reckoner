/**
 * The addresses the composer asks for (built from the settings in `sources.ts`) and how an
 * overlay image is read back: the FEMA flood export and the Forest Service wildfire export are
 * both asked to draw in two exact colours, pure red for the first class and pure blue for the
 * second, on transparency, so reading a pixel's class never depends on either agency's own colours
 * or on the browser's colour management.
 *
 * | Overlay  | Red (class 1)                                          | Blue (class 2)                         |
 * | -------- | ------------------------------------------------------ | -------------------------------------- |
 * | flood    | Special Flood Hazard Area (`SFHA_TF = 'T'`): 1% a year | 0.2% annual chance (`ZONE_SUBTY` 0.2…) |
 * | wildfire | Wildfire Hazard Potential class 4, high                | class 5, very high                     |
 */
import { CENSUS_BASE, FLOOD, OSM_TILES, WILDFIRE } from './sources';
import { type Frame, frameMercatorBox, type TilePlacement } from './tiles';

/** A tile's address from the tile source's template. */
export function tileUrl(t: Pick<TilePlacement, 'x' | 'y' | 'z'>, template: string = OSM_TILES.url): string {
  return template.replace('{z}', String(t.z)).replace('{x}', String(t.x)).replace('{y}', String(t.y));
}

function exportParams(frame: Frame): Record<string, string> {
  return {
    bbox: frameMercatorBox(frame)
      .map((v) => v.toFixed(2))
      .join(','),
    bboxSR: '3857',
    imageSR: '3857',
    size: `${frame.width},${frame.height}`,
  };
}

const query = (params: Record<string, string>) => new URLSearchParams(params).toString();

/** The Census base map for exactly the frame (D5 fallback). */
export function censusExportUrl(frame: Frame): string {
  return `${CENSUS_BASE.url}?${query({
    ...exportParams(frame),
    dpi: '96',
    format: 'png',
    transparent: 'false',
    layers: frame.zoom >= 14 ? CENSUS_BASE.layersNearby : CENSUS_BASE.layers,
    f: 'image',
  })}`;
}

/** A Census tile for the pin map's fallback layer: the export for one tile's box. */
export function censusTileUrl(t: Pick<TilePlacement, 'x' | 'y' | 'z'>): string {
  const n = 2 ** t.z;
  const half = Math.PI * 6378137;
  const size = (2 * half) / n;
  const x0 = t.x * size - half;
  const y1 = half - t.y * size;
  return `${CENSUS_BASE.url}?${query({
    bbox: [x0, y1 - size, x0 + size, y1].map((v) => v.toFixed(2)).join(','),
    bboxSR: '3857',
    imageSR: '3857',
    size: '256,256',
    dpi: '96',
    format: 'png',
    transparent: 'false',
    layers: t.z >= 14 ? CENSUS_BASE.layersNearby : CENSUS_BASE.layers,
    f: 'image',
  })}`;
}

const NO_OUTLINE = { type: 'esriSLS', style: 'esriSLSNull', color: [0, 0, 0, 0], width: 0 };

/** FEMA's flood zones for exactly the frame, drawn red (1% a year) and blue (0.2% a year) on transparency. */
export function floodExportUrl(frame: Frame): string {
  const dynamicLayers = [
    {
      id: FLOOD.layer,
      source: { type: 'mapLayer', mapLayerId: FLOOD.layer },
      definitionExpression: "SFHA_TF = 'T' OR ZONE_SUBTY LIKE '0.2 P%'",
      drawingInfo: {
        renderer: {
          type: 'uniqueValue',
          field1: 'SFHA_TF',
          uniqueValueInfos: [
            { value: 'T', symbol: { type: 'esriSFS', style: 'esriSFSSolid', color: [255, 0, 0, 255], outline: NO_OUTLINE } },
            { value: 'F', symbol: { type: 'esriSFS', style: 'esriSFSSolid', color: [0, 0, 255, 255], outline: NO_OUTLINE } },
          ],
        },
        transparency: 0,
      },
    },
  ];
  return `${FLOOD.url}?${query({
    ...exportParams(frame),
    dpi: '96',
    format: 'png32',
    transparent: 'true',
    dynamicLayers: JSON.stringify(dynamicLayers),
    f: 'image',
  })}`;
}

/** The Forest Service's high (red) and very high (blue) wildfire hazard for exactly the frame, on transparency. */
export function wildfireExportUrl(frame: Frame): string {
  const renderingRule = {
    rasterFunction: 'Colormap',
    rasterFunctionArguments: {
      Colormap: [
        [WILDFIRE.high, 255, 0, 0],
        [WILDFIRE.veryHigh, 0, 0, 255],
      ],
    },
    variableName: 'Raster',
  };
  return `${WILDFIRE.url}?${query({
    ...exportParams(frame),
    format: 'png32',
    transparent: 'true',
    interpolation: 'RSP_NearestNeighbor',
    renderingRule: JSON.stringify(renderingRule),
    f: 'image',
  })}`;
}

/** Each pixel's class: 0 nothing, 1 red (the first class), 2 blue (the second). */
export function classifyRedBlue(rgba: ArrayLike<number>): Uint8Array {
  const out = new Uint8Array(Math.floor(rgba.length / 4));
  for (let i = 0; i < out.length; i++) {
    const r = rgba[i * 4]!;
    const g = rgba[i * 4 + 1]!;
    const b = rgba[i * 4 + 2]!;
    const a = rgba[i * 4 + 3]!;
    // Only red, blue and their blends (purple) are asked for: green stays low in all of them.
    if (a < 96 || g >= 96) continue;
    if (r >= 128 && r >= b) out[i] = 1;
    else if (b >= 128) out[i] = 2;
  }
  return out;
}

/** How many pixels of each class: [nothing, class 1, class 2]. */
export function classCounts(classes: Uint8Array): [number, number, number] {
  const n: [number, number, number] = [0, 0, 0];
  for (const c of classes) n[c as 0 | 1 | 2] += 1;
  return n;
}

/** An RGBA mask (white, opaque where the pixel is of `cls`), for drawing a hatch pattern through. */
export function maskOf(classes: Uint8Array, cls: 1 | 2): Uint8ClampedArray {
  const out = new Uint8ClampedArray(classes.length * 4);
  for (let i = 0; i < classes.length; i++) {
    if (classes[i] !== cls) continue;
    out[i * 4] = 255;
    out[i * 4 + 1] = 255;
    out[i * 4 + 2] = 255;
    out[i * 4 + 3] = 255;
  }
  return out;
}
