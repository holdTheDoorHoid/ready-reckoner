/**
 * Where the site keeps the engine and its data, and which data files load when. Shared by the
 * loader (`loader.ts`), the service worker build (`vite-plugins/pwa.ts`) and the tests, so the
 * three always agree. No imports: the build plugin reads this file in Node.
 *
 * - At start: `manifest.json`, then every file of the `core` pack except the ZIP tables, with the
 *   county list last. That is everything the engine needs to plan for a county.
 * - When a ZIP code is typed (or a saved plan has one): the ZIP tables. Only ZIP lookups read
 *   them; `crates/rr-wasm/src/source.rs` (`ZIP_FILES`) answers `pack_missing` for a location with a
 *   ZIP code until they are in, so a plan is never made from half of the data.
 * - When a map is shown: `geo/counties.json`.
 */

/** Where build-web.sh puts the engine, relative to the site root. */
export const PKG_MODULE = 'pkg/rr_wasm.js';
export const PKG_WASM = 'pkg/rr_wasm_bg.wasm';
/** Where build-web.sh puts the data, relative to the site root. */
export const DATA_DIR = 'data/';
export const MANIFEST_PATH = 'manifest.json';
/** The pack the engine needs to plan. */
export const CORE_PACK = 'core';
/** Loaded last so county records are assembled once. */
export const COUNTY_LIST = 'core/counties.csv';
/** The core files only ZIP-code lookups read; loaded when a ZIP code is needed. Same list as `ZIP_FILES` in crates/rr-wasm/src/source.rs. */
export const ZIP_FILES: readonly string[] = ['core/zip_county.csv', 'core/zip_facilities.csv'];
/** County outlines for the map thumbnail; loaded when a map is shown. */
export const MAP_FILE = 'geo/counties.json';
/** `defaults()` puts this well-formed but unreal ZIP code in a new plan; the app must replace it. */
export const PLACEHOLDER_ZIP = '00000';

/** The parts of `data/manifest.json` the app reads. */
export interface Manifest {
  pack_version?: string;
  generated?: string;
  packs: Record<string, { description?: string; files: ManifestFile[] }>;
  attributions?: unknown[];
}

export interface ManifestFile {
  path: string;
  bytes?: number;
  rows?: number;
  job?: string;
}

/** The core pack's files in the order the loader hands them over: the county list last. */
export function coreLoadOrder(manifest: Manifest): string[] {
  const paths = (manifest.packs[CORE_PACK]?.files ?? []).map((f) => f.path);
  return [...paths.filter((p) => p !== COUNTY_LIST), ...paths.filter((p) => p === COUNTY_LIST)];
}

/** What loads at start: the core pack without the ZIP tables, county list last. */
export function startupFiles(manifest: Manifest): string[] {
  return coreLoadOrder(manifest).filter((p) => !ZIP_FILES.includes(p));
}

/** The ZIP tables the manifest lists (normally all three). */
export function zipFiles(manifest: Manifest): string[] {
  return coreLoadOrder(manifest).filter((p) => ZIP_FILES.includes(p));
}

/** A data file's URL under the site root, versioned so a cached manifest always pairs with its files. */
export function dataUrl(base: string, path: string, packVersion?: string): string {
  return `${base}${DATA_DIR}${path}${packVersion ? `?v=${encodeURIComponent(packVersion)}` : ''}`;
}

/** Does this location need the ZIP tables? A real, well-formed ZIP code does; the placeholder does not. */
export function needsZipTables(location: { zip?: string } | undefined): boolean {
  const zip = location?.zip;
  return zip !== undefined && /^\d{5}$/.test(zip) && zip !== PLACEHOLDER_ZIP;
}

/** Does the location name a place at all (a real ZIP code or a county)? */
export function hasPlace(location: { zip?: string; county_fips?: string } | undefined): boolean {
  if (!location) return false;
  if (location.county_fips !== undefined && location.county_fips !== '') return true;
  return location.zip !== undefined && location.zip !== '' && location.zip !== PLACEHOLDER_ZIP;
}
