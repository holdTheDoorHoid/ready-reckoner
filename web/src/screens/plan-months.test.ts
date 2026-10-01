/**
 * The Plan screen numbers months exactly as the printed packet does (the packet is the oracle;
 * verification R3-16): month 0 begins on the plan date and reads "This month (October 2026)",
 * month 1 is "Month 1 (November 2026)", and so on, so a household reading the screen next to the
 * printed packet sees the same month for the same purchase. Checked against the packet's own
 * checklists ("… (month 4)", "… (months 1–13)") with the real engine output from the goldens.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import type { Engine } from '../engine/index';
import { FIXTURES, type FixtureName } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import type { PlanOutput } from '../engine/types';
import { planMonthLabel, planMonthPhrase } from '../lib/format';
import { render, savedFor, type Rendered } from '../test/helpers';
import { golden, repoRoot } from '../test/real';
import PlanScreen from './PlanScreen.svelte';

let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
});

const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

/** The mock engine, except that `assess` answers with the real engine's golden output. */
function answering(output: PlanOutput): Engine {
  const mock = createMockEngine();
  return { ...mock, assess: async () => ({ ok: true, value: output }) };
}

/** The golden's printed Prepare sheet: since v0.3 its `.md` holds the binder, a rule, then the sheet (`rr plan`). */
function packet(name: FixtureName): string {
  const md = readFileSync(join(repoRoot(), 'fixtures', 'golden', `${name}.md`), 'utf8');
  const at = md.indexOf('\n# Prepare: what to do before\n');
  return at >= 0 ? md.slice(at + 1) : md;
}

/** Every purchase line of the packet's checklists with its month or months: "- [ ] Name: 7 gallons (month 1)". */
function checklistMonths(md: string): { name: string; first: number; last: number }[] {
  const section = md.split('\n## Checklists\n')[1]!.split('\n## ')[0]!;
  return section
    .split('\n')
    .map((line) => /^- \[ \] (.+?): .* \(months? (\d+)(?:–(\d+))?\)$/.exec(line))
    .filter((m): m is RegExpExecArray => m !== null)
    .map((m) => ({ name: m[1]!, first: Number(m[2]), last: Number(m[3] ?? m[2]) }));
}

/** The Plan screen's whole plan: each month's label and the names listed under it. */
function screenMonths(r: Rendered): Map<string, string[]> {
  const months = new Map<string, string[]>();
  for (const d of r.target.querySelectorAll('details.month')) {
    const label = text(d.querySelector('summary > span'));
    months.set(label, [...d.querySelectorAll('.month__list > li > span:first-child')].map(text));
  }
  return months;
}

/** The packet names an item by its short name ("Get-home bag for each commuter"); the screen may add a description after a colon. */
const sameItem = (screenName: string, packetName: string) => screenName === packetName || screenName.startsWith(`${packetName}:`);

async function plan(name: FixtureName, today?: string): Promise<{ r: Rendered; out: PlanOutput; date: string }> {
  const out = golden(name);
  current = await render(PlanScreen, { plan: savedFor(FIXTURES[name]), route: 'plan', engine: answering(out), today });
  return { r: current, out, date: FIXTURES[name].planning_date };
}

describe('plan month labels', () => {
  it('number months from 0 at the plan date, as the packet does', () => {
    expect(planMonthLabel('2026-10-01', 0)).toBe('This month (October 2026)');
    expect(planMonthLabel('2026-10-01', 1)).toBe('Month 1 (November 2026)');
    expect(planMonthLabel('2026-10-01', 4)).toBe('Month 4 (February 2027)');
    expect(planMonthLabel('2026-10-01', 40)).toBe('Month 40 (February 2030)');
    // Back two months later: the month the household is in keeps its number, and the others do not move.
    expect(planMonthLabel('2026-10-01', 2, 2)).toBe('This month: month 2 (December 2026)');
    expect(planMonthLabel('2026-10-01', 0, 2)).toBe('Month 0 (October 2026)');
    expect(planMonthPhrase('2026-10-01', 12)).toBe('month 12 (October 2027)');
  });
});

describe('the Plan screen and the printed packet name the same month for the same purchase', () => {
  it('Philadelphia: every purchase in the packet’s checklists sits under the month the packet gives it', async () => {
    const { r, out, date } = await plan('philadelphia-renters-4');
    const md = packet('philadelphia-renters-4');
    expect(md).toBe(out.prepare_markdown);
    const lines = checklistMonths(md);
    expect(lines.length).toBeGreaterThan(40);
    const months = screenMonths(r);
    for (const { name, first, last } of lines) {
      for (const n of new Set([first, last])) {
        const label = planMonthLabel(date, n, 0);
        const listed = months.get(label);
        expect(listed, `${label} (packet: "${name}", month ${n})`).toBeDefined();
        expect(
          listed!.some((s) => sameItem(s, name)),
          `"${name}" should be listed under "${label}", as the packet's "(month ${n})"; listed: ${listed!.join(' | ')}`,
        ).toBe(true);
      }
    }
    // The packet's "This month" is the plan date's month and its "Next month" is month 1.
    expect(md).toContain('### This month\n');
    expect(md).toMatch(/^### Next month: Month 1 \(from November 1, 2026\)/m);
    expect(text(r.target.querySelector('#this-title'))).toBe('This month (October 2026) from Oct 1, 2026');
    expect(text(r.target.querySelector('#next-title'))).toBe('Month 1 (November 2026) from Nov 1, 2026');
    // Month 1's purchases, in the packet's "Next month" list and on the screen.
    const nextList = md.split(/^### Next month: .*$/m)[1]!.split('\n### ')[0]!;
    const nextBuys = [...nextList.matchAll(/^- \[ \] \*\*(.+?)\*\*: /gm)].map((m) => m[1]!).filter((n) => n !== 'Decide this month');
    expect(nextBuys.length).toBeGreaterThan(3);
    const month1 = months.get('Month 1 (November 2026)')!;
    for (const n of nextBuys) expect(month1.some((s) => sameItem(s, n)), n).toBe(true);
    // When everything is done: the packet's summary month and the screen's.
    const done = planMonthPhrase(date, out.plan.done_month!);
    expect(done).toBe('month 40 (February 2030)');
    expect(md).toContain(`everything by ${done}`);
    expect(text(r.target.querySelector('.done-note'))).toContain(`After ${done} you are done for your risk.`);
    // No label from the old numbering, which counted from 1.
    expect(text(r.target)).not.toMatch(/Month \d+: [A-Z][a-z]+ \d{4}/);
    expect(text(r.target)).not.toMatch(/\bNext month\b/);
  });

  it('Minot: month 0’s purchases are "This month (October 2026)", and the rare-event allowance keeps the packet’s months', async () => {
    const { r, date } = await plan('minot-missile-field-3');
    const md = packet('minot-missile-field-3');
    const months = screenMonths(r);
    const month0 = checklistMonths(md).filter((l) => l.first === 0);
    expect(month0.length).toBeGreaterThan(5);
    const thisMonth = months.get('This month (October 2026)')!;
    for (const { name } of month0) expect(thisMonth.some((s) => sameItem(s, name)), name).toBe(true);
    // "**Personal radiation dosimeter card** (month 4, about $25)"
    const rare = [...md.matchAll(/^- \*\*(.+?)\*\* \(month (\d+), about \$[\d,]+\)/gm)].map((m) => ({ name: m[1]!, month: Number(m[2]) }));
    expect(rare.map((x) => x.month)).toEqual([4, 12]);
    for (const { name, month } of rare) expect(months.get(planMonthLabel(date, month, 0))!.includes(name), name).toBe(true);
  });

  it('keeps the packet’s numbers when the household comes back later', async () => {
    // Two and a half months after the plan date: December is month 2, in the packet and on the screen.
    const { r, date } = await plan('philadelphia-renters-4', '2026-12-15');
    expect(text(r.target.querySelector('#this-title'))).toBe('This month: month 2 (December 2026) from Dec 1, 2026');
    const months = screenMonths(r);
    expect(months.has('This month: month 2 (December 2026)')).toBe(true);
    const extinguisher = checklistMonths(packet('philadelphia-renters-4')).find((l) => l.name === 'Multipurpose fire extinguisher')!;
    expect(extinguisher.first).toBe(2);
    expect(months.get('This month: month 2 (December 2026)')!).toContain('Multipurpose fire extinguisher');
    // Later months keep their numbers.
    expect(months.get(planMonthLabel(date, 4, 2))!.length).toBeGreaterThan(0);
    expect(planMonthLabel(date, 4, 2)).toBe('Month 4 (February 2027)');
  });
});
