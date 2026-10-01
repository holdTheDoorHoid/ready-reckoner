/**
 * The mock engine's binder (`binder.ts`): for every fixture household it passes the structural
 * rules `Binder::check` holds the engine's binder to, has the engine's ten parts and pages (a page
 * per person and per place, the wallet cards, the logs, the Sources page listing every source),
 * echoes the household's answers exactly as written, and holds each kind of inline where the
 * engine's binder does (the parity test compares the two engines by shape; this checks the mock
 * on its own, so a slip shows here even without the WebAssembly build).
 */
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { binderPages, binderProblems, blockInlines, flatBlocks } from '../../lib/binder/model';
import { FIXTURE_NAMES, FIXTURES } from '../fixtures';
import { createMockEngine } from '../mock';
import type { Binder, Block, Inline, PlanInput, PlanOutput } from '../types';
import { MOCK_CHECKLISTS } from './checklists';

async function assess(input: PlanInput): Promise<PlanOutput> {
  const r = await createMockEngine().assess(input);
  if (!r.ok) throw new Error(r.error.message);
  return r.value;
}

const PARTS = ['start', 'people', 'home_places', 'pets_vehicles_documents', 'have', 'check_now', 'check_coming', 'check_ongoing', 'after', 'sources'];

/** The inline kinds found in each place a run of text sits, across a binder. */
function inlineKinds(b: Binder): Record<string, Set<string>> {
  const out: Record<string, Set<string>> = {};
  const note = (where: string, inl: readonly Inline[]) => {
    out[where] ??= new Set();
    for (const i of inl) out[where].add(Object.keys(i)[0]!);
  };
  const walk = (bl: Block, inCallout: boolean) => {
    if ('para' in bl) note(inCallout ? 'callout para' : 'para', bl.para);
    else if ('bullets' in bl) bl.bullets.forEach((x) => note('bullets', x));
    else if ('numbered' in bl) bl.numbered.forEach((x) => note('numbered', x));
    else if ('steps' in bl) bl.steps.forEach((s) => note('steps', s.text));
    else if ('table' in bl) bl.table.rows.forEach((r) => r.forEach((c) => note('table', c)));
    else if ('decision' in bl) bl.decision.branches.forEach((br) => (note('when', br.when), note('then', br.then)));
    else if ('cards' in bl) bl.cards.forEach((c) => c.lines.forEach((l) => note('cards', l)));
    else if ('callout' in bl) bl.callout.blocks.forEach((x) => walk(x, true));
  };
  for (const e of binderPages(b)) e.page.blocks.forEach((bl) => walk(bl, false));
  return out;
}

const sorted = (s: Set<string> | undefined) => [...(s ?? [])].sort();

describe('the mock binder', () => {
  it('passes the binder rules for every fixture, with the ten parts in order and the pages the engine has', async () => {
    for (const name of FIXTURE_NAMES) {
      const input = FIXTURES[name];
      const b = (await assess(input)).binder;
      expect(binderProblems(b), name).toEqual([]);
      expect(b.parts.map((p) => p.id), name).toEqual(PARTS);
      const ids = binderPages(b).map((e) => e.page.id);
      expect(ids.slice(0, 5), name).toEqual(['cover', 'how_to_use', 'quick_start', 'index', 'contacts']);
      for (let i = 1; i <= input.people.length; i++) expect(ids, name).toContain(`person_${i}`);
      for (const id of ['wallet_cards', 'special_needs', 'home', 'neighbourhood', 'getting_out', 'documents', 'inventory', 'risks_glance', 'forecast', 'after', 'log_damage', 'log_medications', 'sources']) expect(ids, `${name}: ${id}`).toContain(id);
      // Every checklist page is one of the content's checklists.
      const checklists = binderPages(b).filter((e) => e.page.kind === 'checklist' && e.page.id !== 'forecast');
      for (const e of checklists) expect(MOCK_CHECKLISTS.some((c) => c.id === e.page.id), e.page.id).toBe(true);
      // One wallet card per person; the Sources page lists every source, numbered as cited.
      const cards = flatBlocks(binderPages(b).find((e) => e.page.id === 'wallet_cards')!.page.blocks).find((x) => 'cards' in x);
      expect(cards && 'cards' in cards ? cards.cards.length : 0, name).toBe(input.people.length);
      const list = binderPages(b).find((e) => e.page.id === 'sources')!.page.blocks.find((x) => 'numbered' in x);
      expect(list && 'numbered' in list ? list.numbered.length : -1, name).toBe(b.sources.length);
      expect(b.sources.length, name).toBeGreaterThan(5);
    }
  });

  it('holds each kind of inline where the engine binder does (rr-plan, agent/binder bd072a9)', async () => {
    for (const name of FIXTURE_NAMES) {
      const k = inlineKinds((await assess(FIXTURES[name])).binder);
      expect(sorted(k.para), name).toEqual(['b', 'cite', 'link', 't']);
      expect(sorted(k.bullets), name).toEqual(['b', 'blank', 'cite', 'link', 't']);
      expect(sorted(k.steps), name).toEqual(['b', 'blank', 'cite', 'link', 't']);
      expect(sorted(k.table), name).toEqual(['b', 'blank', 'cite', 'link', 't']);
      expect(sorted(k.numbered), name).toEqual(['cite', 't']);
      expect(sorted(k.when), name).toEqual(['b', 'blank', 'cite', 't']);
      expect(sorted(k.cards), name).toEqual(['b', 'blank', 't']);
      // A blank in a decision's "then" only where the household has not said where it would go.
      expect(sorted(k.then), name).toEqual(FIXTURES[name].family_plan?.where_we_would_go ? ['cite', 't'] : ['blank', 'cite', 't']);
      // The leave-first box (with its citation) where the engine prints it.
      const leave = ['cameron-insulin-well-farm-2', 'coos-bay-well-owner-2', 'galveston-highrise-1', 'miami-condo-retiree-1', 'missoula-smoke-2', 'sacramento-leveed-2', 'san-juan-2', 'sugar-land-ev-household-3'];
      expect(sorted(k['callout para']), name).toEqual(leave.includes(name) ? ['cite', 't'] : ['t']);
    }
  });

  it("echoes the household's answers exactly as written, markup and Markdown marks included", async () => {
    const philly = (await assess(FIXTURES['philadelphia-renters-4'])).binder;
    const text = JSON.stringify(philly);
    expect(text).toContain('Dr. Morgan, Sample Family Practice, 555-0103, 12 Sample Avenue, Philadelphia');
    expect(text).toContain('Hook inside the kitchen cabinet; a spare with Aunt Rosa');
    const tricky = structuredClone(FIXTURES['philadelphia-renters-4']);
    tricky.people[0]!.profile = { ...tricky.people[0]!.profile, name: 'Dana <b>**Lee**</b> | #1 [x](y)', allergies: 'Peanuts | latex' };
    const b = (await assess(tricky)).binder;
    const person = binderPages(b).find((e) => e.page.id === 'person_1')!.page;
    expect(person.title).toBe('Dana <b>**Lee**</b> | #1 [x](y)');
    const values = person.blocks.flatMap((bl) => ('fields' in bl ? bl.fields.map((f) => f.value) : []));
    expect(values).toContain('Dana <b>**Lee**</b> | #1 [x](y)');
    expect(values).toContain('Peanuts | latex');
    expect(binderProblems(b)).toEqual([]);
  });

  it('links point at pages in the binder; "Which checklist?" sends every risk somewhere', async () => {
    const b = (await assess(FIXTURES['miami-condo-retiree-1'])).binder;
    const index = binderPages(b).find((e) => e.page.id === 'index')!.page;
    const table = index.blocks.find((x) => 'table' in x);
    expect(table && 'table' in table).toBe(true);
    if (table && 'table' in table) for (const row of table.table.rows) expect(row[1]!.some((i) => 'link' in i), JSON.stringify(row[0])).toBe(true);
    const ids = new Set(binderPages(b).map((e) => e.page.id));
    for (const e of binderPages(b)) for (const bl of flatBlocks(e.page.blocks)) for (const i of blockInlines(bl)) if ('link' in i) expect(ids.has(i.link.to), i.link.to).toBe(true);
  });
});

describe("the mock's checklist table", () => {
  it('matches the front matter of every file in content/checklists', () => {
    let dir = process.cwd();
    while (!readdirSync(dir).includes('content')) dir = dirname(dir);
    const files = readdirSync(join(dir, 'content', 'checklists')).filter((f) => f.endsWith('.md')).sort();
    const fromFiles = files.map((f) => {
      const fm = /^---\n([\s\S]*?)\n---\n/.exec(readFileSync(join(dir, 'content', 'checklists', f), 'utf8'))![1]!;
      const get = (k: string) => new RegExp(`^${k}: (.*)$`, 'm').exec(fm)?.[1]?.trim() ?? '';
      return {
        id: get('id'),
        title: get('title'),
        onset: get('onset'),
        applies_to: get('applies_to').replace(/^\[|\]$/g, '').split(',').map((x) => x.trim()).filter(Boolean),
        pages: Number(get('pages') || 1),
      };
    });
    expect(MOCK_CHECKLISTS).toEqual(fromFiles);
  });
});
