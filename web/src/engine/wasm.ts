/**
 * The WebAssembly engine behind `Engine`.
 *
 * `crates/rr-wasm/build-web.sh` writes the wasm-bindgen output (`--target web`) to
 * `web/public/pkg/` and copies the data packs to `web/public/data/`. `loadWasmEngine` loads the
 * module from the site's own origin, hands the data to it (`loadCorePacks`), and `adaptRawEngine`
 * turns each JSON envelope into the `Engine` the app talks to.
 *
 * Loading, in order (docs/ENGINE-API.md, "Loading"):
 * 1. `data/manifest.json` is fetched and loaded first: the engine checks every later file against
 *    the sha256 the manifest records.
 * 2. Every file of the `core` pack is fetched at once and loaded one by one, `core/counties.csv`
 *    last (the engine assembles county records after each file, which is only real work once the
 *    county list is in). File URLs carry `?v=<pack_version>`, so a cached manifest is always paired
 *    with the files it describes.
 * 3. `geo/counties.json` (the map) is not needed to plan; load it later with `loadDataFiles`.
 *
 * No manifest (HTTP 404, or the dev server's HTML page in its place) means the site was built
 * without data: the engine then plans with its seven built-in sample counties and `engine_info()`
 * says so (`packs_loaded` empty, no `data_pack_version`). Any other failure throws, and
 * `getEngine()` falls back to the mock engine and records why.
 */
import type { Engine, RawEngine } from './index';
import type { Envelope } from './types';
import { ENGINE_API_VERSION } from './types';

/** Where build-web.sh puts the engine, relative to the site root. */
export const PKG_MODULE = 'pkg/rr_wasm.js';
export const PKG_WASM = 'pkg/rr_wasm_bg.wasm';
/** Where build-web.sh puts the data, relative to the site root. */
export const DATA_DIR = 'data/';
export const MANIFEST_PATH = 'manifest.json';
/** The pack the engine needs to plan; loaded at start. */
export const CORE_PACK = 'core';
/** Loaded last so county records are assembled once. */
export const COUNTY_LIST = 'core/counties.csv';

function parse<T>(json: () => string): Promise<Envelope<T>> {
  try {
    return Promise.resolve(JSON.parse(json()) as Envelope<T>);
  } catch (e) {
    return Promise.resolve({
      ok: false,
      error: { code: 'internal', message: 'The planner returned something it could not read.', details: { reason: String(e) } },
    });
  }
}

/** Wrap the string-level WebAssembly exports as an `Engine`. */
export function adaptRawEngine(raw: RawEngine): Engine {
  return {
    engine_info: () => parse(() => raw.engine_info()),
    load_pack: (name, bytes) => parse(() => raw.load_pack(name, bytes)),
    county_search: (query) => parse(() => raw.county_search(query)),
    resolve_location: (input) => parse(() => raw.resolve_location(JSON.stringify(input))),
    assess: (input) => parse(() => raw.assess(JSON.stringify(input))),
    explain: (request) => parse(() => raw.explain(JSON.stringify(request))),
    catalogue: () => parse(() => raw.catalogue()),
    defaults: () => parse(() => raw.defaults()),
  };
}

/** The wasm-bindgen module: the `RawEngine` exports plus its two initialisers. */
export interface WasmModule extends RawEngine {
  /** Fetches and instantiates the .wasm (by default from next to the module). */
  default(init?: { module_or_path: string | URL | Response | BufferSource | WebAssembly.Module }): Promise<unknown>;
  /** Instantiates from bytes already in hand (tests in Node). */
  initSync(init: { module: BufferSource | WebAssembly.Module }): unknown;
}

/** The parts of `data/manifest.json` the loader reads. */
export interface Manifest {
  pack_version?: string;
  packs: Record<string, { files: { path: string; bytes?: number }[] }>;
}

/** How far loading has got, after each file. Byte counts come from the manifest. */
export interface LoadProgress {
  file: string;
  filesLoaded: number;
  filesTotal: number;
  bytesLoaded: number;
  bytesTotal: number;
}

export interface LoadOptions {
  /** Defaults to the browser's `fetch`. */
  fetch?: (url: string) => Promise<Response>;
  /** Called after each file is loaded. */
  onProgress?: (progress: LoadProgress) => void;
}

/** What was loaded: the manifest's pack version and the files, in load order. */
export interface LoadResult {
  packVersion?: string;
  files: string[];
}

/** The core pack's files in the order the loader hands them over: the county list last. */
export function coreLoadOrder(manifest: Manifest): string[] {
  const paths = (manifest.packs[CORE_PACK]?.files ?? []).map((f) => f.path);
  return [...paths.filter((p) => p !== COUNTY_LIST), ...paths.filter((p) => p === COUNTY_LIST)];
}

function isManifest(value: unknown): value is Manifest {
  if (typeof value !== 'object' || value === null) return false;
  const packs = (value as { packs?: unknown }).packs;
  return typeof packs === 'object' && packs !== null;
}

async function engineLoad(engine: Engine, name: string, bytes: Uint8Array): Promise<void> {
  const result = await engine.load_pack(name, bytes);
  if (!result.ok) throw new Error(result.error.message);
}

/**
 * Fetch data files (paths as the manifest lists them) from the site's own origin, all at once,
 * and hand them to the engine in the order given. Throws on the first file that fails.
 */
export async function loadDataFiles(
  engine: Engine,
  base: string,
  manifest: Manifest,
  paths: readonly string[],
  options: LoadOptions = {},
): Promise<void> {
  const get = options.fetch ?? ((url: string) => fetch(url));
  const sizes = new Map(Object.values(manifest.packs).flatMap((p) => p.files.map((f) => [f.path, f.bytes ?? 0] as const)));
  const bytesTotal = paths.reduce((sum, p) => sum + (sizes.get(p) ?? 0), 0);
  const version = manifest.pack_version ? `?v=${encodeURIComponent(manifest.pack_version)}` : '';
  const downloads = paths.map(async (path) => {
    const response = await get(`${base}${DATA_DIR}${path}${version}`);
    if (!response.ok) throw new Error(`The data file ${path} did not download (HTTP ${response.status}).`);
    return new Uint8Array(await response.arrayBuffer());
  });
  // Every download is awaited below in order; this only keeps a later failure from being reported
  // as unhandled while an earlier file is still loading.
  for (const d of downloads) d.catch(() => undefined);
  let bytesLoaded = 0;
  for (const [i, path] of paths.entries()) {
    await engineLoad(engine, path, await downloads[i]!);
    bytesLoaded += sizes.get(path) ?? 0;
    options.onProgress?.({ file: path, filesLoaded: i + 1, filesTotal: paths.length, bytesLoaded, bytesTotal });
  }
}

/**
 * Hand the engine the data it needs to plan: the manifest, then the core pack. Returns null when
 * the site has no data (the engine keeps planning with its built-in sample counties).
 */
export async function loadCorePacks(engine: Engine, base: string, options: LoadOptions = {}): Promise<LoadResult | null> {
  const get = options.fetch ?? ((url: string) => fetch(url));
  const response = await get(`${base}${DATA_DIR}${MANIFEST_PATH}`);
  const type = response.headers.get('content-type') ?? '';
  if (response.status === 404 || (response.ok && type.includes('text/html'))) return null;
  if (!response.ok) throw new Error(`The data manifest did not download (HTTP ${response.status}).`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  let manifest: unknown;
  try {
    manifest = JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw new Error('The data manifest is damaged: it is not JSON.');
  }
  if (!isManifest(manifest)) throw new Error('The data manifest is damaged: it lists no packs.');
  await engineLoad(engine, MANIFEST_PATH, bytes);
  const files = coreLoadOrder(manifest);
  await loadDataFiles(engine, base, manifest, files, options);
  const result: LoadResult = { files: [MANIFEST_PATH, ...files] };
  if (manifest.pack_version) result.packVersion = manifest.pack_version;
  return result;
}

/** Load the engine from `${base}pkg/`, check it speaks this app's contract, and give it the data. */
export async function loadWasmEngine(base: string, options: LoadOptions = {}): Promise<Engine> {
  const mod = (await import(/* @vite-ignore */ `${base}${PKG_MODULE}`)) as WasmModule;
  await mod.default({ module_or_path: `${base}${PKG_WASM}` });
  const engine = adaptRawEngine(mod);
  const info = await engine.engine_info();
  if (!info.ok) throw new Error(info.error.message);
  if (info.value.api_version !== ENGINE_API_VERSION) {
    throw new Error(
      `The planning engine speaks contract version ${info.value.api_version}, but this app needs version ${ENGINE_API_VERSION}. Rebuild it with crates/rr-wasm/build-web.sh.`,
    );
  }
  await loadCorePacks(engine, base, options);
  return engine;
}
