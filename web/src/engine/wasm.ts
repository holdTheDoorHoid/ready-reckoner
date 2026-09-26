/**
 * The WebAssembly engine behind `Engine`.
 *
 * `crates/rr-wasm/build-web.sh` writes the wasm-bindgen output (`--target web`) to
 * `web/public/pkg/` and copies the data packs to `web/public/data/`. `loadWasmEngine` loads the
 * module from the site's own origin, checks that it speaks this app's contract, and returns at
 * once, so the app can show its first screens while the data arrives: `PackLoader` (loader.ts)
 * fetches the core pack in the background, the ZIP tables when a ZIP code is needed and the map
 * when one is shown.
 *
 * `gateEngine` makes the wait invisible to the screens: a call that needs data (county search, a
 * location, a plan, an explanation) waits for the part it needs, then goes to the engine. Answers
 * that do not depend on data (the catalogue, the defaults, a problem with the answers themselves)
 * come straight back. If the data cannot be had, those calls answer `pack_missing` with the reason,
 * and the progress line offers to try again. Planning from the engine's built-in sample counties
 * is never shown while real data is on its way.
 */
import type { Engine, RawEngine } from './index';
import type { EngineError, Envelope, LocationInput, PlanOutput } from './types';
import { ENGINE_API_VERSION } from './types';
import { needsZipTables, PKG_MODULE, PKG_WASM } from './data-files';
import { PackLoader, type LoaderOptions } from './loader';

export { COUNTY_LIST, CORE_PACK, coreLoadOrder, DATA_DIR, MANIFEST_PATH, PKG_MODULE, PKG_WASM, type Manifest } from './data-files';

/** Performance-timeline names: the engine's own `assess` call, and the whole call with its JSON. */
export const ASSESS_ENGINE_MEASURE = 'rr:assess:engine';
export const ASSESS_MEASURE = 'rr:assess';

function measure(name: string, start: number, end: number): void {
  try {
    if (typeof performance.measure === 'function') performance.measure(name, { start, end });
  } catch {
    // Timing is for measuring the app, never a reason for a call to fail.
  }
}

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

/** Wrap the string-level WebAssembly exports as an `Engine`. `assess` is timed in the Performance timeline. */
export function adaptRawEngine(raw: RawEngine): Engine {
  return {
    engine_info: () => parse(() => raw.engine_info()),
    load_pack: (name, bytes) => parse(() => raw.load_pack(name, bytes)),
    county_search: (query) => parse(() => raw.county_search(query)),
    resolve_location: (input) => parse(() => raw.resolve_location(JSON.stringify(input))),
    assess: (input) => {
      const start = performance.now();
      const result = parse<PlanOutput>(() => {
        const json = JSON.stringify(input);
        const t = performance.now();
        const out = raw.assess(json);
        measure(ASSESS_ENGINE_MEASURE, t, performance.now());
        return out;
      });
      measure(ASSESS_MEASURE, start, performance.now());
      return result;
    },
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

/**
 * An `Engine` whose data-dependent calls wait for the data they need. `county_search` needs the core
 * pack; a location, a plan or an explanation needs the core pack and, when it has a real ZIP code,
 * the ZIP tables. While data is still loading, a call is first tried as it is, so a problem with the
 * answers themselves (`bad_input`, which never depends on data) comes back at once.
 */
export function gateEngine(engine: Engine, loader: PackLoader): Engine {
  const ready = (location?: LocationInput) => loader.isReady('core') && (!needsZipTables(location) || loader.isReady('zip'));

  async function wait(location?: LocationInput): Promise<EngineError | null> {
    try {
      await loader.core();
      if (needsZipTables(location)) await loader.zip();
      return null;
    } catch (e) {
      return { code: 'pack_missing', message: e instanceof Error ? e.message : String(e) };
    }
  }

  async function gated<T>(location: LocationInput | undefined, call: () => Promise<Envelope<T>>, probe: boolean): Promise<Envelope<T>> {
    if (ready(location)) return call();
    if (probe) {
      const early = await call();
      if (!early.ok && early.error.code === 'bad_input') return early;
    }
    const failed = await wait(location);
    return failed ? { ok: false, error: failed } : call();
  }

  return {
    engine_info: () => engine.engine_info(),
    load_pack: (name, bytes) => engine.load_pack(name, bytes),
    county_search: (query) => gated(undefined, () => engine.county_search(query), false),
    resolve_location: (input) => gated(input, () => engine.resolve_location(input), true),
    assess: (input) => gated(input?.location, () => engine.assess(input), true),
    explain: (request) => gated(request?.input?.location, () => engine.explain(request), true),
    catalogue: () => engine.catalogue(),
    defaults: () => engine.defaults(),
  };
}

/** The engine with its data loader. */
export interface LoadedWasm {
  engine: Engine;
  loader: PackLoader;
}

/**
 * Load the engine from `${base}pkg/` and check it speaks this app's contract. Returns as soon as
 * the engine is running; the core data starts loading in the background (`loader.core()`).
 */
export async function loadWasmEngine(base: string, options: LoaderOptions = {}): Promise<LoadedWasm> {
  const mod = (await import(/* @vite-ignore */ `${base}${PKG_MODULE}`)) as WasmModule;
  await mod.default({ module_or_path: `${base}${PKG_WASM}` });
  const raw = adaptRawEngine(mod);
  const info = await raw.engine_info();
  if (!info.ok) throw new Error(info.error.message);
  if (info.value.api_version !== ENGINE_API_VERSION) {
    throw new Error(
      `The planning engine speaks contract version ${info.value.api_version}, but this app needs version ${ENGINE_API_VERSION}. Rebuild it with crates/rr-wasm/build-web.sh.`,
    );
  }
  const loader = new PackLoader(raw, base, options);
  void loader.core().catch(() => undefined);
  return { engine: gateEngine(raw, loader), loader };
}
