/**
 * The v0.1.1 displays (round-2 review: the owner's risk matrix, H-02, M-04, W1, W3–W7, W9, W11,
 * C1), checked on the real engine's Philadelphia output (fixtures/golden) with the real item
 * catalogue, so what is tested is what a person sees on the live site.
 */
import axe from 'axe-core';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import type { PlanOutput } from '../engine/types';
import { ARTICLES, withSourceLinks } from '../learn/articles';
import { addMonths, formatMonth } from '../lib/format';
import { dialSentence, NUCLEAR_NOTE, STATUS_LINE } from '../lib/labels';
import { nextMilestone } from '../lib/savings';
import { render, savedFor, type Rendered } from '../test/helpers';
import { engineWith, golden } from '../test/real';
import About from './About.svelte';
import Learn from './Learn.svelte';
import PlanScreen from './PlanScreen.svelte';
import Risks from './Risks.svelte';
import Start from './Start.svelte';

const NAME = 'philadelphia-renters-4';
let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
});

async function screenWith(Screen: typeof Risks, route: string, name = NAME): Promise<{ r: Rendered; out: PlanOutput }> {
  const out = golden(name);
  current = await render(Screen, { plan: savedFor(FIXTURES[name as keyof typeof FIXTURES]), route, engine: await engineWith(out) });
  return { r: current, out };
}

const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

describe('the risk matrix (owner request)', () => {
  it('comes directly after the settings: the first table, before the warnings and the cards', async () => {
    const { r } = await screenWith(Risks, 'risks');
    const matrix = r.target.querySelector('section.matrix')!;
    expect(matrix).not.toBeNull();
    expect(r.target.querySelector('section.settings')!.nextElementSibling).toBe(matrix);
    expect(r.target.querySelector('table')).toBe(matrix.querySelector('table'));
    const likely = r.target.querySelector('#likely-title')!;
    expect(matrix.compareDocumentPosition(likely) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const warnings = r.target.querySelector('section[aria-label="Things to look at"]');
    if (warnings) expect(matrix.compareDocumentPosition(warnings) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(text(matrix.querySelector('caption'))).toMatch(/^Your risks, most likely first/);
    expect(text(matrix.querySelector('h2'))).toBe('Your risks at a glance');
  });

  it('has a row for every ranked hazard in the engine order, four columns, and the rare rows under a divider with a range', async () => {
    const { r, out } = await screenWith(Risks, 'risks');
    const ranked = out.register.filter((h) => h.display === 'ranked');
    const rare = out.register.filter((h) => h.display === 'rare_catastrophic');
    const matrix = r.target.querySelector('section.matrix')!;
    expect(matrix.querySelectorAll('thead th')).toHaveLength(4);
    const [main, rareBody] = [...matrix.querySelectorAll('tbody')];
    const rows = [...main!.querySelectorAll('tr')];
    expect(rows).toHaveLength(ranked.length);
    rows.forEach((row, i) => {
      const h = ranked[i]!;
      expect(text(row.querySelector('th')), h.id).toBe(`${i + 1}. ${h.name}`);
      expect(row.querySelector('a')!.getAttribute('href')).toBe(`#hazard-${h.id}`);
      expect(row.querySelectorAll('td'), h.id).toHaveLength(3);
    });
    // Worded as the cards word them.
    expect(text(rows[0]!.querySelectorAll('td')[0])).toBe('nearly every household; about 4.5 times a year');
    const supply = rows[ranked.findIndex((h) => h.id === 'supply_chain_disruption')]!;
    expect(text(supply.querySelectorAll('td')[0])).toBe('about 86 (63–98) of 100; about 1 in 6 a year');
    const fire = rows[ranked.findIndex((h) => h.id === 'house_fire')]!;
    expect(text(fire.querySelectorAll('td')[0])).toBe('about 5 of 100; about 1 in 190 a year');
    expect(text(fire.querySelectorAll('td')[1])).toBe('Severe');
    expect(text(fire.querySelectorAll('td')[2])).toBe('Mostly data');
    // The rare rows: a divider, then a range and a link to the box, never a rank or a count of 100.
    const rareRows = [...rareBody!.querySelectorAll('tr')];
    expect(text(rareRows[0])).toMatch(/^Rare but severe/);
    expect(rareRows.slice(1)).toHaveLength(rare.length);
    const cells = rareRows.slice(1).map((row) => text(row.querySelectorAll('td')[0]));
    expect(cells).toEqual(['between 1 in 200 and 1 in 41', 'between 1 in 1,000 and 1 in 100']);
    for (const row of rareRows.slice(1)) {
      expect(row.querySelector('a')!.getAttribute('href')).toBe('#rare-title');
      expect(text(row.querySelector('th'))).not.toMatch(/^\d/);
    }
    expect(text(matrix)).toContain(NUCLEAR_NOTE);
  });

  it('jumps to a card folded away in "All N risks": opens it, moves focus to the heading, and leaves the address alone', async () => {
    const { r, out } = await screenWith(Risks, 'risks');
    const ranked = out.register.filter((h) => h.display === 'ranked');
    for (const h of ranked) {
      const card = r.target.querySelector(`#hazard-${h.id}`);
      expect(card, h.id).not.toBeNull();
      expect(text(card), h.id).toContain('Back to the table');
    }
    const folded = ranked[10]!;
    const details = r.target.querySelector('details.all-risks') as HTMLDetailsElement;
    expect(details.open).toBe(false);
    const hash = window.location.hash;
    (r.target.querySelector(`#matrix-${folded.id}`) as HTMLAnchorElement).click();
    flushSync();
    expect(details.open).toBe(true);
    const heading = r.target.querySelector(`#hazard-${folded.id} h3`)!;
    expect(document.activeElement).toBe(heading);
    expect(heading.getAttribute('tabindex')).toBe('-1');
    expect(window.location.hash).toBe(hash);
    // Back to the table: focus returns to the same row's link, which keeps its place in the tab order.
    const back = [...r.target.querySelectorAll(`#hazard-${folded.id} a`)].find((a) => a.textContent?.includes('Back to the table')) as HTMLAnchorElement;
    back.click();
    flushSync();
    const link = r.target.querySelector(`#matrix-${folded.id}`)!;
    expect(document.activeElement).toBe(link);
    expect(link.hasAttribute('tabindex')).toBe(false);
    // A rare row goes to the rare box.
    (r.target.querySelector('#matrix-nuclear_attack') as HTMLAnchorElement).click();
    flushSync();
    expect(document.activeElement).toBe(r.target.querySelector('#rare-title'));
  });

  it('follows the "Show chances over" setting', async () => {
    const { r } = await screenWith(Risks, 'risks');
    r.app.plan!.input.dials.horizon_years = 1;
    flushSync();
    const matrix = r.target.querySelector('section.matrix')!;
    expect(text(matrix.querySelector('thead'))).toContain('How likely (1 year)');
    // The golden output is fixed, so only the horizon changes: nuclear over one year.
    expect(text(matrix)).toContain('between 1 in 2,000 and 1 in 400');
  });
});

describe('the rare box (H-02)', () => {
  it('shows a range only, a header without "households like yours", and the nuclear note', async () => {
    const { r } = await screenWith(Risks, 'risks');
    const box = r.target.querySelector('section.rare')!;
    const header = text(box.querySelectorAll('thead th')[1]);
    expect(header).toBe('How likely (in the next 10 years)');
    const cells = [...box.querySelectorAll('tbody tr')].map((row) => text(row.querySelectorAll('td')[0]));
    expect(cells).toEqual(['between 1 in 200 and 1 in 41', 'between 1 in 1,000 and 1 in 100']);
    expect(text(box)).not.toContain('households like yours');
    expect(text(box)).not.toMatch(/about \d+ of 100/);
    expect(text(box)).toContain(NUCLEAR_NOTE);
    expect(text(box)).toContain('Back to the table');
  });
});

describe('the Risks cards and targets', () => {
  it('say what the dial means per need and for all needs together (M-04), word for word at 1-in-100', async () => {
    const { r } = await screenWith(Risks, 'risks');
    expect(text(r.target)).toContain(dialSentence('one_in_100'));
    (r.target.querySelector('button[aria-controls="settings-panel"]') as HTMLButtonElement).click();
    flushSync();
    const dial = r.target.querySelector('fieldset.dial')!;
    expect(text(dial)).toContain('For any one need, something worse than its target comes in about 1 of every 10 ten-year stretches.');
    expect(text(dial)).toContain('Across all your needs together, the chance that at least one runs out is higher, roughly 1 in 3.');
    expect(text(r.target)).not.toContain('Something longer reaches');
  });

  it('the savings card starts its sentences with a capital', async () => {
    const { r } = await screenWith(Risks, 'risks');
    expect(text(r.target.querySelector('[data-bucket="income"]'))).toContain('Half a month saved so far.');
  });

  it('the evacuate card says "1 minute" (W1)', async () => {
    const { r } = await screenWith(Risks, 'risks');
    expect(text(r.target)).toContain('Notice could be 1 minute to 3 days');
    expect(text(r.target)).not.toContain('1 minutes');
  });

  it('what helps fits the hazard (W4, W5) and keeps acronyms (W6)', async () => {
    const { r } = await screenWith(Risks, 'risks');
    const helps = (id: string) => text(r.target.querySelector(`#hazard-${id} .helps`));
    expect(helps('heat_wave')).not.toMatch(/warm room/i);
    expect(helps('heat_wave')).toMatch(/cool room/i);
    // Towels are an assumed everyday basic: the card says the household has them, not a price.
    expect(helps('heat_wave')).toContain('towels to wet and cool down (have it)');
    expect(helps('cold_wave')).toMatch(/warm room/i);
    expect(helps('cold_wave')).not.toMatch(/fan|cool room/i);
    expect(helps('medical_emergency')).toMatch(/^What helps: family first-aid kit \(\$\d+\), bleeding-control kit/);
    expect(helps('medical_emergency')).not.toMatch(/N95/i);
    expect(text(r.target)).toContain('N95 respirators');
    expect(text(r.target)).not.toMatch(/\bn95\b/);
  });

  it('passes axe (jsdom; contrast is checked in a real browser)', async () => {
    const { r } = await screenWith(Risks, 'risks');
    const results = await axe.run(r.target, { rules: { 'color-contrast': { enabled: false }, region: { enabled: false } } });
    expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).slice(0, 3).join(', ')}`)).toEqual([]);
  });
});

describe('the plan (W3, W9, W11)', () => {
  it('counts only steps taken, and shows what the household already had apart', async () => {
    const { r, out } = await screenWith(PlanScreen, 'plan');
    const all = out.plan.months.flatMap((m) => m.items);
    const had = all.filter((i) => i.done).length;
    expect(had).toBe(8);
    const stat = [...r.target.querySelectorAll('.stats li')].find((li) => li.textContent?.includes('Done so far'))!;
    expect(text(stat)).toBe(`Done so far 0 of ${all.length - had} steps Already have: 8`);
    const section = r.target.querySelector('#had-title')!.closest('section')!;
    expect(text(section.querySelector('h2'))).toBe('Already have 8');
    expect(text(section.querySelector('summary'))).toBe('Show what you already have');
    expect(r.target.querySelector('#done-title')).toBeNull();
  });

  it('sets cash aside instead of buying it', async () => {
    const { r } = await screenWith(PlanScreen, 'plan');
    const envelopes = [...r.target.querySelectorAll('.envelopes li')].map(text);
    const cash = envelopes.find((t) => t.startsWith('Cash in small bills'))!;
    expect(cash).toMatch(/all set aside around \w+ \d{4}/);
    expect(cash).not.toContain('ready to buy');
    expect(envelopes.find((t) => t.startsWith('Cold-weather sleeping bag'))).toMatch(/ready to buy around/);
  });

  it('puts the first savings goal before the full goal, on the plan and the risks screens', async () => {
    const first = `First goal: one month of expenses, about $4,200, by ${formatMonth(addMonths('2026-10-01', 25 + 35))}.`;
    expect(first).toBe('First goal: one month of expenses, about $4,200, by October 2031.');
    for (const [Screen, route] of [
      [PlanScreen, 'plan'],
      [Risks, 'risks'],
    ] as const) {
      const { r } = await screenWith(Screen, route);
      const card = r.target.querySelector('[data-bucket="income"]')!;
      expect(text(card), route).toContain(first);
      expect(text(card).indexOf('First goal')).toBeLessThan(text(card).indexOf('Full goal: about 4 months'));
      current!.cleanup();
      current = undefined;
    }
  });

  it('passes axe (jsdom)', async () => {
    const { r } = await screenWith(PlanScreen, 'plan');
    const results = await axe.run(r.target, { rules: { 'color-contrast': { enabled: false }, region: { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });
});

describe('the nearer savings goal, worked out from the engine’s savings track', () => {
  it('is one month first, then three, dated from the end of the supplies plan', () => {
    const philly = golden(NAME).plan.savings_track!;
    expect(nextMilestone(philly, '2026-10-01', 25)).toEqual({ months: 1, first: true, usd: 4200, by: '2031-10-01' });
    // Phoenix: 2 of 7 months saved, $40 a month once the plan is done in month 38: three months next.
    const phoenix = golden('phoenix-apartment-cpap-1');
    const m = nextMilestone(phoenix.plan.savings_track!, '2026-10-01', phoenix.plan.done_month);
    expect(m).toEqual({ months: 3, first: false, usd: 7800, by: addMonths('2026-10-01', 38 + 65) });
    // Nothing nearer when a goal is already met, or the full goal is no bigger.
    expect(nextMilestone(golden('sugar-land-ev-household-3').plan.savings_track!, '2026-10-01', 2)).toBeNull();
    expect(nextMilestone(golden('coos-bay-well-owner-2').plan.savings_track!, '2026-10-01', 15)).toBeNull();
    // No expenses given: the goal in months only; no saving pace: no date.
    expect(nextMilestone({ ...philly, target_usd: 0 }, '2026-10-01', 25)).toEqual({ months: 1, first: true });
    expect(nextMilestone({ ...philly, monthly_suggestion_usd: 0 }, '2026-10-01', 25)).toEqual({ months: 1, first: true, usd: 4200 });
    expect(nextMilestone(philly, '2026-10-01', undefined)).toEqual({ months: 1, first: true, usd: 4200 });
  });
});

describe('the status line (C1)', () => {
  it('sits under the privacy box on Start, and on About', async () => {
    current = await render(Start, { plan: null, route: '' });
    const promise = current.target.querySelector('section.promise')!;
    expect(promise.nextElementSibling?.classList.contains('status-line')).toBe(true);
    expect(text(promise.nextElementSibling)).toBe(STATUS_LINE);
    current.cleanup();
    current = await render(About, { plan: null, route: 'about' });
    expect(text(current.target.querySelector('.status-line'))).toBe(STATUS_LINE);
  });
});

describe('Learn (W7)', () => {
  it('shows the five reviewed topic articles, with no draft labels', async () => {
    current = await render(Learn, { plan: null, route: 'learn' });
    expect(text(current.target)).not.toMatch(/draft/i);
    expect(ARTICLES.map((a) => [a.slug, a.title])).toEqual([
      ['consequences', 'Why the plan starts from what happens, not what causes it'],
      ['myths', 'Disaster myths and what really happens'],
      ['numbers', 'How the numbers are made'],
      ['children', 'Talking with children about emergencies'],
      ['community', 'Neighbours and mutual aid'],
    ]);
    for (const a of ARTICLES) {
      expect(a.draft, a.slug).toBe(false);
      expect(text(current.target)).toContain(a.title);
    }
  });

  it('renders every article cleanly: sources as numbered notes, no template marks, no old dial claim', async () => {
    for (const a of ARTICLES) {
      current = await render(Learn, { plan: null, route: `learn/${a.slug}` });
      const page = current.target;
      expect(text(page.querySelector('h1')), a.slug).toBe(a.title);
      expect(text(page), a.slug).not.toMatch(/draft|\{if:|\{\{|\[\^/i);
      expect(text(page), a.slug).not.toContain('would see a longer one');
      expect(page.querySelectorAll('.footnotes li').length, a.slug).toBeGreaterThan(0);
      expect(page.querySelector('.prose h3'), a.slug).toBeNull();
      current.cleanup();
      current = undefined;
    }
  });

  it('links each note to its source when the catalogue has it', () => {
    const body = 'Text.[^a]\n\n[^a]: Agency, A report (2025).\n[^b]: Unlisted.\n[^c]: Bad link.';
    const out = withSourceLinks(body, [
      { id: 'a', title: 'A report', publisher: 'Agency', url: 'https://www.example.gov/report', retrieved: '2026-09-26', license: 'Public domain' },
      { id: 'c', title: 'C', publisher: 'C', url: 'javascript:alert(1)', retrieved: '2026-09-26', license: '' },
    ]);
    expect(out).toContain('[^a]: Agency, A report (2025). [example.gov](https://www.example.gov/report)');
    expect(out).toContain('[^b]: Unlisted.');
    expect(out).toContain('[^c]: Bad link.');
    expect(withSourceLinks(body, undefined)).toBe(body);
  });
});
