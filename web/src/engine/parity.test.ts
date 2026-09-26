/**
 * Parity between the engines that answer the same contract (docs/ENGINE-API.md):
 *
 * 1. **Mock vs WebAssembly, shapes.** For every fixture household, and for every other function,
 *    the mock engine and the WebAssembly engine answer with the same JSON shape: the same keys
 *    holding the same kinds of value, at every depth. A key the contract marks optional (`?` in
 *    types.ts) may be missing on either side; array elements are compared as the union of their
 *    shapes.
 * 2. **WebAssembly vs the CLI, numbers.** Every fixture's answer equals
 *    `fixtures/golden/<name>.json` (written by the native engine: `rr golden`, rr-plan's golden
 *    helper) value for value, every number to the last bit.
 *
 * The WebAssembly engine runs twice: once on its built-in sample counties (the goldens are made
 * that way) and once loaded with the real packs in `data/` through `loadCorePacks`, the loader
 * the site uses.
 *
 * Needs the WebAssembly build (`bash crates/rr-wasm/build-web.sh`). Without it the suite is
 * skipped, unless RR_WASM_PARITY=required (CI), which turns a missing build into a failure.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { beforeAll, describe, expect, it } from 'vitest';

import type { Engine } from './index';
import { FIXTURE_NAMES, FIXTURES } from './fixtures';
import { createMockEngine } from './mock';
import type { Envelope, ExplainKind, PlanInput, PlanOutput } from './types';
import type { WasmModule } from './wasm';
import { adaptRawEngine, loadCorePacks, loadDataFiles } from './wasm';

/** The repository root: the nearest directory above the working directory with fixtures/golden and web/. */
function repoRoot(): string {
  for (let dir = process.cwd(), i = 0; i < 6; i++, dir = dirname(dir)) {
    if (existsSync(join(dir, 'fixtures', 'golden')) && existsSync(join(dir, 'web', 'package.json'))) return `${dir}/`;
  }
  throw new Error(`cannot find the repository root above ${process.cwd()}`);
}

const REPO = repoRoot();
const PKG = `${REPO}web/public/pkg/`;
const BUILT = existsSync(`${PKG}rr_wasm.js`) && existsSync(`${PKG}rr_wasm_bg.wasm`);
const REQUIRED = process.env.RR_WASM_PARITY === 'required';

// ---------------------------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------------------------

/** The kinds of value found at one place, merged over every example seen there. */
type Shape =
  | { t: 'leaf'; types: Set<string> }
  | { t: 'array'; items: Shape | undefined }
  | { t: 'object'; fields: Map<string, { shape: Shape; present: number }>; count: number };

function shapeOf(value: unknown): Shape {
  if (Array.isArray(value)) return { t: 'array', items: value.reduce<Shape | undefined>((acc, v) => merge(acc, shapeOf(v)), undefined) };
  if (value !== null && typeof value === 'object') {
    const fields = new Map<string, { shape: Shape; present: number }>();
    for (const [k, v] of Object.entries(value)) fields.set(k, { shape: shapeOf(v), present: 1 });
    return { t: 'object', fields, count: 1 };
  }
  return { t: 'leaf', types: new Set([value === null ? 'null' : typeof value]) };
}

function merge(a: Shape | undefined, b: Shape | undefined): Shape | undefined {
  if (!a || !b) return a ?? b;
  if (a.t === 'leaf' && b.t === 'leaf') return { t: 'leaf', types: new Set([...a.types, ...b.types]) };
  if (a.t === 'array' && b.t === 'array') return { t: 'array', items: merge(a.items, b.items) };
  if (a.t === 'object' && b.t === 'object') {
    const fields = new Map<string, { shape: Shape; present: number }>();
    for (const k of new Set([...a.fields.keys(), ...b.fields.keys()])) {
      const fa = a.fields.get(k);
      const fb = b.fields.get(k);
      fields.set(k, { shape: merge(fa?.shape, fb?.shape)!, present: (fa?.present ?? 0) + (fb?.present ?? 0) });
    }
    return { t: 'object', fields, count: a.count + b.count };
  }
  // A string in one place and an object in another (`powered_device`): record both kinds.
  return { t: 'leaf', types: new Set([kindOf(a), kindOf(b)]) };
}

function kindOf(s: Shape): string {
  return s.t === 'leaf' ? [...s.types].sort().join('|') : s.t;
}

/** Field names the contract marks optional (`name?:` in types.ts). */
const OPTIONAL = new Set([...readFileSync(`${REPO}web/src/engine/types.ts`, 'utf8').matchAll(/^\s+(\w+)\?:/gm)].map((m) => m[1]!));

interface ShapeReport {
  problems: string[];
  /** Leaf places compared. */
  compared: number;
  /** Places not compared because one side had only empty arrays there. */
  unchecked: string[];
}

function compareShapes(mock: Shape | undefined, wasm: Shape | undefined, path: string, r: ShapeReport): void {
  if (!mock || !wasm) {
    r.unchecked.push(path);
    return;
  }
  if (mock.t !== wasm.t) {
    r.problems.push(`${path}: ${kindOf(mock)} in the mock, ${kindOf(wasm)} in the WebAssembly engine`);
    return;
  }
  if (mock.t === 'leaf' && wasm.t === 'leaf') {
    r.compared += 1;
    if (kindOf(mock) !== kindOf(wasm)) r.problems.push(`${path}: ${kindOf(mock)} in the mock, ${kindOf(wasm)} in the WebAssembly engine`);
    return;
  }
  if (mock.t === 'array' && wasm.t === 'array') {
    compareShapes(mock.items, wasm.items, `${path}[]`, r);
    return;
  }
  if (mock.t === 'object' && wasm.t === 'object') {
    for (const k of new Set([...mock.fields.keys(), ...wasm.fields.keys()])) {
      const m = mock.fields.get(k);
      const w = wasm.fields.get(k);
      const at = path ? `${path}.${k}` : k;
      if (!m || !w) {
        if (!OPTIONAL.has(k)) r.problems.push(`${at}: only the ${m ? 'mock' : 'WebAssembly engine'} sends it, and the contract does not make it optional`);
        continue;
      }
      const alwaysM = m.present === mock.count;
      const alwaysW = w.present === wasm.count;
      if (alwaysM !== alwaysW && !OPTIONAL.has(k)) {
        r.problems.push(`${at}: always sent by the ${alwaysM ? 'mock' : 'WebAssembly engine'}, only sometimes by the other`);
      }
      compareShapes(m.shape, w.shape, at, r);
    }
  }
}

function shapeReport(mock: unknown, wasm: unknown): ShapeReport {
  const r: ShapeReport = { problems: [], compared: 0, unchecked: [] };
  compareShapes(shapeOf(mock), shapeOf(wasm), '', r);
  return r;
}

// ---------------------------------------------------------------------------------------------
// Exact values
// ---------------------------------------------------------------------------------------------

interface ValueReport {
  differences: string[];
  numbers: number;
  strings: number;
}

function compareValues(expected: unknown, actual: unknown, path: string, r: ValueReport): void {
  if (r.differences.length > 20) return;
  if (typeof expected === 'number' && typeof actual === 'number') {
    r.numbers += 1;
    if (!Object.is(expected, actual)) r.differences.push(`${path}: golden ${expected}, WebAssembly ${actual}`);
    return;
  }
  if (Array.isArray(expected) && Array.isArray(actual)) {
    if (expected.length !== actual.length) r.differences.push(`${path}: ${expected.length} entries in the golden, ${actual.length} from WebAssembly`);
    expected.forEach((e, i) => compareValues(e, actual[i], `${path}[${i}]`, r));
    return;
  }
  if (expected !== null && actual !== null && typeof expected === 'object' && typeof actual === 'object') {
    const ek = Object.keys(expected);
    const ak = Object.keys(actual);
    if (ek.join() !== ak.join()) r.differences.push(`${path}: keys ${ek.join(',')} in the golden, ${ak.join(',')} from WebAssembly`);
    for (const k of ek) compareValues((expected as Record<string, unknown>)[k], (actual as Record<string, unknown>)[k], path ? `${path}.${k}` : k, r);
    return;
  }
  if (typeof expected === 'string') r.strings += 1;
  if (expected !== actual) r.differences.push(`${path}: golden ${JSON.stringify(expected)?.slice(0, 80)}, WebAssembly ${JSON.stringify(actual)?.slice(0, 80)}`);
}

// ---------------------------------------------------------------------------------------------
// Engines
// ---------------------------------------------------------------------------------------------

/** A fresh instance of the WebAssembly engine (its own memory, so its own loaded packs). */
async function wasmInstance(tag: string): Promise<Engine> {
  const mod = (await import(/* @vite-ignore */ `${pathToFileURL(`${PKG}rr_wasm.js`).href}?${tag}`)) as WasmModule;
  mod.initSync({ module: readFileSync(`${PKG}rr_wasm_bg.wasm`) });
  return adaptRawEngine(mod);
}

/** `fetch` over the repository's own `data/` directory, standing in for the site's origin. */
async function fromDataDir(url: string): Promise<Response> {
  const path = url.replace(/^\/site\//, '').replace(/\?.*$/, '');
  const file = `${REPO}${path}`;
  if (!existsSync(file)) return new Response('not found', { status: 404 });
  return new Response(readFileSync(file), { headers: { 'content-type': path.endsWith('.json') ? 'application/json' : 'text/plain' } });
}

function value<T>(e: Envelope<T>, what: string): T {
  if (!e.ok) throw new Error(`${what}: ${e.error.code}: ${e.error.message}`);
  return e.value;
}

function errorOf<T>(e: Envelope<T>, what: string) {
  if (e.ok) throw new Error(`${what}: expected an error`);
  return e.error;
}

function clone<T>(x: T): T {
  return JSON.parse(JSON.stringify(x)) as T;
}

function median(xs: number[]): number {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)]!;
}

function expectSameShape(mock: unknown, wasm: unknown, what: string): ShapeReport {
  const r = shapeReport(mock, wasm);
  expect(r.problems, `${what}: the mock and WebAssembly answers differ in shape`).toEqual([]);
  expect(r.compared, what).toBeGreaterThan(0);
  return r;
}

// ---------------------------------------------------------------------------------------------

describe.runIf(REQUIRED && !BUILT)('parity needs the WebAssembly build', () => {
  it('is present in web/public/pkg', () => {
    expect.fail('web/public/pkg/rr_wasm.js is missing: run `bash crates/rr-wasm/build-web.sh` first (RR_WASM_PARITY=required).');
  });
});

describe.runIf(BUILT)('parity: mock and WebAssembly engines, and the goldens', () => {
  const mock = createMockEngine();
  let fixturesWasm: Engine;
  let packsWasm: Engine;
  let packVersion = '';
  let loadMs = 0;

  beforeAll(async () => {
    fixturesWasm = await wasmInstance('sample-counties');
    packsWasm = await wasmInstance('packs');
    const start = performance.now();
    const loaded = await loadCorePacks(packsWasm, '/site/', { fetch: fromDataDir });
    loadMs = performance.now() - start;
    if (!loaded) throw new Error('data/manifest.json is missing');
    packVersion = loaded.packVersion ?? '';
  });

  it('WebAssembly equals the golden JSON for every fixture, number for number', async () => {
    const lines: string[] = [];
    for (const name of FIXTURE_NAMES) {
      const golden = JSON.parse(readFileSync(`${REPO}fixtures/golden/${name}.json`, 'utf8')) as PlanOutput;
      // The goldens record which data made them; run the engine the same way.
      const onPacks = !golden.data_pack_version.startsWith('fixtures+');
      if (onPacks) expect(golden.data_pack_version, `${name}: the golden was made from other packs than data/`).toBe(packVersion);
      const engine = onPacks ? packsWasm : fixturesWasm;
      const times: number[] = [];
      let out: PlanOutput | undefined;
      for (let i = 0; i < 3; i++) {
        const t = performance.now();
        out = value(await engine.assess(FIXTURES[name]), name);
        times.push(performance.now() - t);
      }
      const r: ValueReport = { differences: [], numbers: 0, strings: 0 };
      compareValues(golden, out, '', r);
      expect(r.differences, `${name}: the WebAssembly answer differs from fixtures/golden/${name}.json (rebuild with build-web.sh if the engine changed)`).toEqual([]);
      expect(JSON.stringify(out)).toBe(JSON.stringify(golden));
      lines.push(
        `${name}: ${r.numbers} numbers and ${r.strings} strings identical to the golden (${onPacks ? 'data packs' : 'sample counties'}); assess ${median(times).toFixed(0)} ms`,
      );
    }
    console.info(`parity with fixtures/golden:\n  ${lines.join('\n  ')}`);
  });

  it('the plan has the same shape from the mock and from WebAssembly, for every fixture', async () => {
    const lines: string[] = [];
    for (const name of FIXTURE_NAMES) {
      const m = value(await mock.assess(FIXTURES[name]), `mock ${name}`);
      for (const [mode, engine] of [
        ['sample counties', fixturesWasm],
        ['data packs', packsWasm],
      ] as const) {
        const w = value(await engine.assess(FIXTURES[name]), `wasm ${name}`);
        const r = expectSameShape(m, w, `${name} (${mode})`);
        lines.push(`${name} (${mode}): ${r.compared} places compared, ${r.unchecked.length} empty on one side`);
      }
    }
    console.info(`plan shapes, mock vs WebAssembly:\n  ${lines.join('\n  ')}`);
  });

  it('every other function answers with the same shape', async () => {
    const pack = new TextEncoder().encode('a,b\n1,2\n');
    const mockLoaded = createMockEngine();
    value(await mockLoaded.load_pack('core', pack), 'mock load_pack');

    // engine_info before and after data, and what load_pack returns.
    expectSameShape(value(await mock.engine_info(), 'mock'), value(await fixturesWasm.engine_info(), 'wasm'), 'engine_info (no packs)');
    expectSameShape(value(await mockLoaded.engine_info(), 'mock'), value(await packsWasm.engine_info(), 'wasm'), 'engine_info (packs)');
    const manifest = readFileSync(`${REPO}data/manifest.json`);
    const fresh = await wasmInstance('load-pack');
    expectSameShape(value(await mockLoaded.load_pack('core', pack), 'mock'), value(await fresh.load_pack('manifest.json', manifest), 'wasm'), 'load_pack');

    expectSameShape(value(await mock.catalogue(), 'mock'), value(await fixturesWasm.catalogue(), 'wasm'), 'catalogue');
    expectSameShape(value(await mock.defaults(), 'mock'), value(await fixturesWasm.defaults(), 'wasm'), 'defaults');
    for (const engine of [fixturesWasm, packsWasm]) {
      expectSameShape(value(await mock.county_search('phila'), 'mock'), value(await engine.county_search('phila'), 'wasm'), 'county_search');
      const place = { country: 'US', zip: '19147', setting: 'urban' } as const;
      expectSameShape(value(await mock.resolve_location(place), 'mock'), value(await engine.resolve_location(place), 'wasm'), 'resolve_location');
    }

    // explain, for one id of each kind taken from each engine's own plan.
    const input = FIXTURES['philadelphia-renters-4'];
    const idsIn = (o: PlanOutput): [ExplainKind, string][] => {
      const ids: [ExplainKind, string][] = [
        ['hazard', o.register[0]!.id],
        ['bucket', 'water_out'],
        ['item', o.plan.months.flatMap((m) => m.items)[0]!.item_id],
        ['requirement', o.requirements[0]!.id],
      ];
      if (o.warnings[0]) ids.push(['warning', o.warnings[0].id]);
      return ids;
    };
    const mockIds = idsIn(value(await mock.assess(input), 'mock'));
    const wasmIds = idsIn(value(await fixturesWasm.assess(input), 'wasm'));
    for (const [kind, mockId] of mockIds) {
      const wasmId = wasmIds.find(([k]) => k === kind)?.[1];
      if (!wasmId) continue;
      expectSameShape(
        value(await mock.explain({ kind, id: mockId, input }), `mock explain ${kind}`),
        value(await fixturesWasm.explain({ kind, id: wasmId, input }), `wasm explain ${kind}`),
        `explain ${kind}`,
      );
    }
  });

  it('errors have the same code and the same shape of details', async () => {
    const at = (location: PlanInput['location']): PlanInput => ({ ...clone(FIXTURES['philadelphia-renters-4']), location });
    const cases: [string, PlanInput, Engine][] = [
      ['unknown_zip', at({ country: 'US', zip: '00000', setting: 'urban' }), fixturesWasm],
      ['bad_input', { ...clone(FIXTURES['philadelphia-renters-4']), people: [] }, fixturesWasm],
    ];
    for (const [code, input, engine] of cases) {
      const m = errorOf(await mock.assess(input), `mock ${code}`);
      const w = errorOf(await engine.assess(input), `wasm ${code}`);
      expect([m.code, w.code]).toEqual([code, code]);
      expectSameShape(m, w, code);
    }
    // An ambiguous ZIP code: the mock's sample one and a real one from the packs.
    const mockZip = errorOf(await mock.resolve_location({ country: 'US', zip: '19087', setting: 'urban' }), 'mock ambiguous');
    const realZip = ambiguousZip();
    const wasmZip = errorOf(await packsWasm.resolve_location({ country: 'US', zip: realZip, setting: 'urban' }), `wasm ambiguous ${realZip}`);
    expect([mockZip.code, wasmZip.code]).toEqual(['ambiguous_zip', 'ambiguous_zip']);
    expectSameShape(mockZip, wasmZip, 'ambiguous_zip');
    // Unknown county: both answer unknown_county with a suggestions list.
    const m = errorOf(await mock.resolve_location({ country: 'US', county_fips: '99998', setting: 'urban' }), 'mock unknown county');
    const w = errorOf(await packsWasm.resolve_location({ country: 'US', county_fips: '99998', setting: 'urban' }), 'wasm unknown county');
    expect([m.code, w.code]).toEqual(['unknown_county', 'unknown_county']);
  });

  it('says in engine_info whether it runs on the data packs or on its sample counties', async () => {
    const sample = value(await fixturesWasm.engine_info(), 'sample');
    expect(sample.packs_loaded).toEqual([]);
    expect(sample.data_pack_version).toBeUndefined();
    const loaded = value(await packsWasm.engine_info(), 'packs');
    expect(loaded.packs_loaded).toEqual(['core']);
    expect(loaded.data_pack_version).toBe(packVersion);
    expect(loaded.attributions[0]!.source).toBe('FEMA National Risk Index');
    // The map pack loads on demand through the same loader.
    const manifest = JSON.parse(readFileSync(`${REPO}data/manifest.json`, 'utf8'));
    await loadDataFiles(packsWasm, '/site/', manifest, ['geo/counties.json'], { fetch: fromDataDir });
    expect(value(await packsWasm.engine_info(), 'packs').packs_loaded).toEqual(['core', 'geo']);
    console.info(`data packs ${packVersion}: loaded through loadCorePacks in ${loadMs.toFixed(0)} ms`);
  });
});

/** A ZIP code in data/core/zip_county.csv whose largest county holds less than 80 % of it. */
function ambiguousZip(): string {
  const [header, ...rows] = readFileSync(`${REPO}data/core/zip_county.csv`, 'utf8').trim().split('\n');
  const cols = header!.split(',');
  const zipCol = cols.indexOf('zip');
  const shareCol = cols.findIndex((c) => c.includes('share'));
  const top = new Map<string, number>();
  for (const row of rows) {
    const cells = row.split(',');
    const zip = cells[zipCol]!;
    top.set(zip, Math.max(top.get(zip) ?? 0, Number(cells[shareCol])));
  }
  for (const [zip, share] of top) if (share > 0.3 && share < 0.6) return zip;
  throw new Error('no ambiguous ZIP code in data/core/zip_county.csv');
}
