/**
 * The validation page's rows (lib/validation.ts) against the frozen backtest they copy
 * (docs/VALIDATION.md, rr-consequence's `--test backtest`). Once that file is in the repository
 * every verdict, earlier verdict and in-sample flag here must match its "By event" table and its
 * tally; until then the rows are checked against the counts recorded with them.
 * awaiting: consequence (docs/VALIDATION.md arrives with agent/consequence2).
 */
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { createMockEngine } from '../engine/mock';
import { repoRoot } from '../test/real';
import { agrees, tally, VALIDATION_DOC_URL, VALIDATION_EVENTS, VALIDATION_RUN, VERDICTS, type Verdict } from './validation';

/** docs/VALIDATION.md, or another copy to check against before it merges (RR_VALIDATION_MD=path). */
const DOC = process.env.RR_VALIDATION_MD ?? join(repoRoot(), 'docs', 'VALIDATION.md');

/** "short: boil water 5 d vs 6" -> "short"; "covered (water over)" -> "covered"; "not modelled" -> "not_modelled". */
function verdictOf(cell: string): Verdict {
  const word = cell.trim().toLowerCase();
  if (word.startsWith('not modelled')) return 'not_modelled';
  const first = word.split(/[\s:(]/)[0] as Verdict;
  if (!VERDICTS.includes(first)) throw new Error(`no verdict in "${cell}"`);
  return first;
}

/** The "By event" table: number, event cell (with the in-sample star), before, after. */
function byEvent(doc: string): { n: number; inSample: boolean; before: Verdict; after: Verdict }[] {
  const start = doc.indexOf('**By event**');
  const lines = doc.slice(start).split('\n').filter((l) => /^\|\s*\d+\s*\|/.test(l));
  return lines.map((l) => {
    const cells = l.split('|').map((c) => c.trim());
    return { n: Number(cells[1]), inSample: cells[2]!.includes('\\*'), before: verdictOf(cells[3]!), after: verdictOf(cells[4]!) };
  });
}

describe('the validation rows', () => {
  it('are the 22 events of the frozen test, in order, each with a household, what happened and what changed', () => {
    expect(VALIDATION_EVENTS.map((r) => r.n)).toEqual(Array.from({ length: 22 }, (_, i) => i + 1));
    for (const r of VALIDATION_EVENTS) {
      for (const field of ['event', 'place', 'when', 'household', 'happened', 'target', 'changed'] as const) {
        expect(r[field].trim().length, `${r.n} ${field}`).toBeGreaterThan(0);
      }
      // Plain words only: no engine ids on the page.
      expect(`${r.household} ${r.happened} ${r.target} ${r.changed}`, String(r.n)).not.toMatch(/[a-z]+_[a-z]+/);
    }
  });

  it('add up to the tallies recorded with them: 6 / 9 / 6 / 1 now, 6 / 5 / 10 / 1 before this round', () => {
    expect(tally(VALIDATION_EVENTS)).toEqual({ covered: 6, partial: 9, short: 6, not_modelled: 1 });
    expect(tally(VALIDATION_EVENTS, 'before')).toEqual(VALIDATION_RUN.first);
  });

  it('agree with the summary the engine carries (EngineInfo.validation, here the mock)', async () => {
    const info = await createMockEngine().engine_info();
    if (!info.ok || !info.value.validation) throw new Error('no validation summary');
    expect(agrees(info.value.validation, VALIDATION_EVENTS)).toBe(true);
    expect(agrees({ ...info.value.validation, short: 7 }, VALIDATION_EVENTS)).toBe(false);
  });

  it('link to the address the source registry gives the test (rr_validation_2026), which every packet cites', () => {
    const toml = readFileSync(join(repoRoot(), 'content', 'citations.toml'), 'utf8');
    const entry = toml.split('[[citation]]').find((block) => /^id = "rr_validation_2026"$/m.test(block));
    expect(entry, 'rr_validation_2026 in content/citations.toml').toBeDefined();
    expect(/^url = "([^"]+)"$/m.exec(entry!)?.[1]).toBe(VALIDATION_DOC_URL);
    expect(VALIDATION_DOC_URL).toMatch(/^https:\/\/github\.com\/holdTheDoorHoid\/ready-reckoner\/blob\/main\/docs\/VALIDATION\.md$/);
  });

  it.runIf(existsSync(DOC))('match docs/VALIDATION.md verdict for verdict, and its tally', () => {
    const doc = readFileSync(DOC, 'utf8');
    const rows = byEvent(doc);
    expect(rows.map((r) => r.n)).toEqual(VALIDATION_EVENTS.map((r) => r.n));
    for (const r of rows) {
      const mine = VALIDATION_EVENTS.find((e) => e.n === r.n)!;
      expect(mine.verdict, `event ${r.n} now`).toBe(r.after);
      expect(mine.before, `event ${r.n} before`).toBe(r.before);
      expect(mine.in_sample, `event ${r.n} in sample`).toBe(r.inSample);
    }
    // The tally row this page reports.
    const t = tally(VALIDATION_EVENTS);
    expect(doc).toContain(`| This version, with the tables and the v2 answers | ${t.covered} | ${t.partial} | ${t.short} | ${t.not_modelled} |`);
  });
});
