/**
 * Verification (docs/VERIFICATION.md, V-10): the screens with the real engine's output.
 *
 * The screen tests use the mock engine, which never lists a savings deposit and the purchase it
 * pays for in the same month. The real engine does: when a sinking fund fills, that month holds a
 * `reserve` line and a `purchase` line for the same item and tier (Philadelphia, month 14: "save
 * toward cash in small bills" and "Cash in small bills"). PlanScreen keys its month lists by
 * `item.item_id + item.tier`, so Svelte throws `each_key_duplicate` and the plan screen never
 * renders in the browser (seen in the built site with the WebAssembly engine, 2026-09-26).
 *
 * The fix belongs to the web workstream: key those lists by `item.item_id + item.tier +
 * item.kind` (PlanScreen.svelte lines 113, 129, 134, 143, 189, 213; ReadinessCard.svelte 34, 42;
 * HazardCard.svelte 61). Fixed by the keyedItems helper (web/src/lib/lookup.ts): the test below now passes.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import { describe, expect, it } from 'vitest';

import type { Engine } from '../engine/index';
import { FIXTURES } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import type { PlanOutput } from '../engine/types';
import { render, savedFor } from '../test/helpers';
import PlanScreen from './PlanScreen.svelte';
import Risks from './Risks.svelte';

/** The repository root: the nearest directory above the working directory with fixtures/golden. */
function root(): string {
  let dir = process.cwd();
  while (!existsSync(join(dir, 'fixtures', 'golden'))) {
    const up = dirname(dir);
    if (up === dir) throw new Error('fixtures/golden not found above the working directory');
    dir = up;
  }
  return dir;
}

function golden(name: string): PlanOutput {
  return JSON.parse(readFileSync(join(root(), 'fixtures', 'golden', `${name}.json`), 'utf8')) as PlanOutput;
}

/** The mock engine, except that `assess` answers with the real engine's golden output. */
function answering(output: PlanOutput): Engine {
  const mock = createMockEngine();
  return { ...mock, assess: async () => ({ ok: true, value: output }) };
}

describe('screens with the real engine output (verification)', () => {
  const name = 'philadelphia-renters-4';

  it('the real plan can list a deposit and its purchase for one item in the same month', () => {
    const out = golden(name);
    const month = out.plan.months.find((m) => {
      const keys = m.items.map((i) => `${i.item_id}${i.tier}`);
      return new Set(keys).size !== keys.length;
    });
    expect(month, 'a month with a reserve and a purchase of the same item').toBeDefined();
    const kinds = month!.items.filter((i) => i.item_id === 'docs_cash_reserve').map((i) => i.kind);
    expect(kinds.sort()).toEqual(['purchase', 'reserve']);
  });

  it('the risks screen renders the real Philadelphia output', async () => {
    const r = await render(Risks, { plan: savedFor(FIXTURES[name]), route: 'risks', engine: answering(golden(name)) });
    expect(r.target.querySelector('h1')?.textContent).toContain('Your risks');
    r.cleanup();
  });

  it('V-10 (fixed by keyedItems): the plan screen renders the real Philadelphia plan', async () => {
    const r = await render(PlanScreen, { plan: savedFor(FIXTURES[name]), route: 'plan', engine: answering(golden(name)) });
    try {
      expect(r.target.querySelector('h1')?.textContent).toContain('Your plan');
      expect(r.text()).toContain('Cash in small bills');
    } finally {
      r.cleanup();
    }
  });
});
