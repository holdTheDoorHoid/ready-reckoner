/**
 * The WebAssembly engine behind `Engine`. `crates/rr-wasm/build-web.sh` (web-engine workstream)
 * writes the wasm-bindgen output to `web/public/pkg/`; this adapter loads it at runtime from the
 * site's own origin, parses each JSON envelope, and hands the core data packs to `load_pack`.
 *
 * awaiting: rr-wasm (module name `rr_wasm.js`, exports matching `RawEngine`) and the pack layout
 * in `data/manifest.json` (which packs load at start). Until then the site ships with the mock.
 */
import type { Engine, RawEngine } from './index';
import type { Envelope } from './types';

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

interface Manifest {
  packs: Record<string, { files: { path: string }[] }>;
}

/** Fetch the packs the engine needs at start (same origin only) and hand them over. */
async function loadCorePacks(engine: Engine, base: string): Promise<void> {
  const response = await fetch(`${base}data/manifest.json`);
  if (!response.ok) throw new Error(`data/manifest.json: HTTP ${response.status}`);
  const manifest = (await response.json()) as Manifest;
  // awaiting: rr-data — whether load_pack takes one file or a whole pack; one file at a time for now.
  for (const file of manifest.packs.core?.files ?? []) {
    const bytes = new Uint8Array(await (await fetch(`${base}data/${file.path}`)).arrayBuffer());
    const result = await engine.load_pack(file.path, bytes);
    if (!result.ok) throw new Error(`${file.path}: ${result.error.message}`);
  }
}

export async function loadWasmEngine(base: string): Promise<Engine> {
  const url = `${base}pkg/rr_wasm.js`;
  const mod = (await import(/* @vite-ignore */ url)) as RawEngine & { default: () => Promise<unknown> };
  await mod.default();
  const engine = adaptRawEngine(mod);
  await loadCorePacks(engine, base);
  return engine;
}
