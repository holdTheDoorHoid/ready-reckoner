/**
 * The real engine's output and catalogue items for tests, without a WebAssembly build: golden
 * packets from fixtures/golden (the engine's own output, committed), and the item catalogue read
 * from content/items/*.toml. The TOML reader handles only the forms those files use (strings,
 * numbers, booleans, arrays of strings, one-line inline tables), which keeps the tests in step with
 * the content without a TOML dependency. Everything else in the catalogue (bucket and hazard
 * names, tiers, citations) comes from the mock, whose ids are the contract's.
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import type { Engine } from '../engine/index';
import { createMockEngine } from '../engine/mock';
import type { Catalogue, Item, PlanOutput } from '../engine/types';

/** The repository root: the nearest directory above the working directory with fixtures/golden. */
export function repoRoot(): string {
  let dir = process.cwd();
  while (!existsSync(join(dir, 'fixtures', 'golden'))) {
    const up = dirname(dir);
    if (up === dir) throw new Error('fixtures/golden not found above the working directory');
    dir = up;
  }
  return dir;
}

export function golden(name: string): PlanOutput {
  return JSON.parse(readFileSync(join(repoRoot(), 'fixtures', 'golden', `${name}.json`), 'utf8')) as PlanOutput;
}

/** Brackets still open in `s`, counting only those outside double-quoted strings. */
function openBrackets(s: string): number {
  let depth = 0;
  let inString = false;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (inString) {
      if (c === '\\') i++;
      else if (c === '"') inString = false;
    } else if (c === '"') inString = true;
    else if (c === '[' || c === '{') depth++;
    else if (c === ']' || c === '}') depth--;
  }
  return depth;
}

function value(raw: string): unknown {
  const v = raw.trim();
  if (v === 'true' || v === 'false') return v === 'true';
  if (v.startsWith('"')) return JSON.parse(v);
  if (v.startsWith('[')) return JSON.parse(v.replace(/,(\s*)\]$/, '$1]'));
  if (v.startsWith('{')) return JSON.parse(v.replace(/([{,]\s*)([A-Za-z_]+)\s*=/g, '$1"$2":'));
  return Number(v);
}

function parseItems(toml: string): Record<string, unknown>[] {
  const items: Record<string, unknown>[] = [];
  let current: Record<string, unknown> | null = null;
  let pending = '';
  for (const line of toml.split('\n')) {
    if (pending) {
      pending += `\n${line}`;
      if (openBrackets(pending) > 0) continue;
      const eq = pending.indexOf('=');
      current![pending.slice(0, eq).trim()] = value(pending.slice(eq + 1));
      pending = '';
      continue;
    }
    const t = line.trim();
    if (t === '[[item]]') {
      current = {};
      items.push(current);
      continue;
    }
    if (!current || t === '' || t.startsWith('#')) continue;
    const eq = t.indexOf('=');
    if (eq < 0) continue;
    if (openBrackets(t) > 0) {
      pending = t;
      continue;
    }
    current[t.slice(0, eq).trim()] = value(t.slice(eq + 1));
  }
  return items;
}

/** Every item in content/items, shaped as `catalogue()` sends it. */
export function realItems(): Item[] {
  const dir = join(repoRoot(), 'content', 'items');
  return readdirSync(dir)
    .filter((f) => f.endsWith('.toml'))
    .sort()
    .flatMap((f) => parseItems(readFileSync(join(dir, f), 'utf8')))
    .map((r) => ({
      look_for: [],
      avoid: [],
      citations: [],
      hazard_extras: [],
      life_safety: false,
      rare_catastrophic: false,
      assumed_basic: false,
      ...r,
    })) as unknown as Item[];
}

/** The mock catalogue with the real items. */
export async function realCatalogue(): Promise<Catalogue> {
  const mock = await createMockEngine().catalogue();
  if (!mock.ok) throw new Error('mock catalogue');
  return { ...mock.value, items: realItems() };
}

/** The mock engine, except that `assess` answers with `output` and the catalogue has the real items. */
export async function engineWith(output: PlanOutput): Promise<Engine> {
  const mock = createMockEngine();
  const catalogue = await realCatalogue();
  return { ...mock, assess: async () => ({ ok: true, value: output }), catalogue: async () => ({ ok: true, value: catalogue }) };
}
