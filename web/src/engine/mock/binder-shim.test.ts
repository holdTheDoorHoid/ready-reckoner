/**
 * Contract v3: the mock's `assess` returns the Prepare sheet (`prepare_markdown`, its packet) and a
 * transitional binder built from it the way the engine builds its own (binder-shim.ts,
 * crates/rr-plan/src/packet/shim.rs), passing the structural rules of `Binder::check`.
 */
import { describe, expect, it } from 'vitest';

import { FIXTURE_NAMES, FIXTURES } from '../fixtures';
import { createMockEngine } from '../mock';
import type { Binder, Block, PlanOutput } from '../types';
import { MAX_TABS, SHORT_TITLE_MAX } from '../types';
import { SHIM_PARTS, shimBinder } from './binder-shim';

async function assess(name: (typeof FIXTURE_NAMES)[number]): Promise<PlanOutput> {
  const r = await createMockEngine().assess(FIXTURES[name]);
  if (!r.ok) throw new Error(`${name}: ${r.error.message}`);
  return r.value;
}

/** The structural problems `Binder::check` (rr-types) looks for. */
function problems(b: Binder): string[] {
  const out: string[] = [];
  let last = 0;
  const partIds = new Set<string>();
  for (const p of b.parts) {
    if (p.tab < 1 || p.tab > MAX_TABS) out.push(`${p.id}: tab ${p.tab}`);
    if (p.tab <= last) out.push(`${p.id}: tab ${p.tab} after ${last}`);
    last = p.tab;
    if (partIds.has(p.id)) out.push(`part ${p.id} twice`);
    partIds.add(p.id);
    if (p.pages.length === 0) out.push(`${p.id}: no pages`);
    if ([...p.short_title].length > SHORT_TITLE_MAX) out.push(`${p.id}: long label`);
  }
  const pageIds = new Set<string>();
  for (const page of b.parts.flatMap((p) => p.pages)) {
    if (pageIds.has(page.id)) out.push(`page ${page.id} twice`);
    pageIds.add(page.id);
  }
  b.sources.forEach((s, i) => {
    if (s.n !== i + 1) out.push(`source ${i + 1} numbered ${s.n}`);
  });
  return out;
}

function paragraphs(b: Binder): string[] {
  return b.parts
    .flatMap((p) => p.pages)
    .flatMap((page) => page.blocks)
    .map((block: Block) => {
      if (!('para' in block)) throw new Error(`the transitional binder holds only paragraphs: ${JSON.stringify(block)}`);
      return block.para.map((i) => ('t' in i ? i.t : '')).join('');
    });
}

describe('the transitional binder', () => {
  it('carries the whole packet for every fixture and passes the structural rules', async () => {
    for (const name of FIXTURE_NAMES) {
      const out = await assess(name);
      const b = out.binder;
      expect(problems(b), name).toEqual([]);
      const fromPacket = out.prepare_markdown
        .split('\n\n')
        .map((p) => p.trim())
        .filter((p) => p !== '' && !p.startsWith('# ') && !p.startsWith('## '))
        .sort();
      expect(paragraphs(b).sort(), name).toEqual(fromPacket);
      const sections = out.prepare_markdown.split('\n').filter((l) => l.startsWith('## ')).length;
      expect(b.parts.flatMap((p) => p.pages)).toHaveLength(sections + 1);
      expect(b.parts[0]!.pages[0]!.kind).toBe('cover');
      expect(b.sources.map((s) => s.title)).toEqual(out.provenance.map((c) => c.title));
      expect(b.generated_on).toBe(FIXTURES[name].planning_date);
      for (const p of b.parts) expect(SHIM_PARTS.some(([tab, id]) => tab === p.tab && id === p.id), p.id).toBe(true);
    }
  });

  it('places the engine sections where DESIGN-DELTA-v3 §4.2 puts them', () => {
    const md = '# T\n\nFor: x\n\n## Summary\n\nA.\n\n## Wallet cards\n\nB.\n\n## Sources\n\nC.\n\n## Something new\n\nD.\n';
    const b = shimBinder(md, {
      generatedOn: '2026-10-01',
      household: '1 adult',
      location: 'Somewhere County, State',
      reviewBy: '2027-10-01',
      provenance: [],
      attributions: [],
    });
    expect(b.title).toBe('T');
    expect(b.parts.map((p) => [p.tab, p.id, p.pages.map((pg) => pg.id)])).toEqual([
      [1, 'start', ['cover', 'summary']],
      [2, 'people', ['wallet_cards']],
      [5, 'have', ['something_new']],
      [10, 'sources', ['sources']],
    ]);
    expect(b.parts[0]!.pages[1]!.blocks).toEqual([{ para: [{ t: 'A.' }] }]);
    expect(problems(b)).toEqual([]);
  });
});
