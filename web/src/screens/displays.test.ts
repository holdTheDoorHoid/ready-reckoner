/**
 * The v0.1.1 displays (round-2 review: the owner's risk matrix, H-02, M-04, W1, W3–W7, W9, W11,
 * C1), checked on the real engine's Philadelphia output (fixtures/golden) with the real item
 * catalogue, so what is tested is what a person sees on the live site. Expected values come from
 * the golden packet itself, not copied numbers, so these tests keep passing when the goldens are
 * regenerated; the wording rules behind them are pinned with fixed inputs in lib/*.test.ts.
 */
import axe from 'axe-core';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import type { PlanOutput } from '../engine/types';
import { ARTICLES, withSourceLinks } from '../learn/articles';
import { helpsFor } from '../lib/helps';
import { addMonths, chanceShort, chanceWithin, CONFIDENCE_LABELS, formatMonth, noticeRange, perYearWords, rangeOnly, severityBand, usd } from '../lib/format';
import { dialSentence, NUCLEAR_NOTE, STATUS_LINE } from '../lib/labels';
import { lowerFirst } from '../lib/lookup';
import { nextMilestone } from '../lib/savings';
import { render, savedFor, type Rendered } from '../test/helpers';
import { engineWith, golden, realCatalogue } from '../test/real';
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

async function screenWith(Screen: typeof Risks, route: string, name = NAME, output?: PlanOutput): Promise<{ r: Rendered; out: PlanOutput }> {
  const out = output ?? golden(name);
  current = await render(Screen, { plan: savedFor(FIXTURES[name as keyof typeof FIXTURES]), route, engine: await engineWith(out) });
  return { r: current, out };
}

/** What a matrix row's "How likely" cell should say for a hazard over `years`. */
function likelyCell(h: PlanOutput['register'][number], years = 10): string {
  const p = chanceWithin(h.rate_per_year, years);
  const chance = h.confidence === 'prior' ? chanceShort(p, chanceWithin(h.rate_range[0], years), chanceWithin(h.rate_range[1], years)) : chanceShort(p);
  return `${chance}; ${perYearWords(h.rate_per_year, h.annual_probability)}`;
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
      const cells = row.querySelectorAll('td');
      expect(cells, h.id).toHaveLength(3);
      // Worded as the card words it (lib/format.test.ts checks that wording against the engine's
      // sentences), expert estimates with their range, then how often a year.
      expect(text(cells[0]), h.id).toBe(likelyCell(h));
      expect(text(cells[1]), h.id).toBe(severityBand(h.severity).label);
      expect(text(cells[2]), h.id).toBe(CONFIDENCE_LABELS[h.confidence]);
    });
    expect(rows.some((row) => /\(\d+–\d+\) of 100/.test(text(row))), 'an expert estimate shows its range').toBe(true);
    // The rare rows: a divider, then a range and a link to the box, never a rank or a count of 100.
    const rareRows = [...rareBody!.querySelectorAll('tr')];
    expect(text(rareRows[0])).toMatch(/^Rare but severe/);
    expect(rareRows.slice(1)).toHaveLength(rare.length);
    const cells = rareRows.slice(1).map((row) => text(row.querySelectorAll('td')[0]));
    expect(cells).toEqual(rare.map((h) => rangeOnly(h.rate_range[0], h.rate_range[1], 10)));
    for (const c of cells) expect(c).toMatch(/^(between 1 in [\d,]+ and 1 in [\d,]+|very unlikely: less than 1 in [\d,]+|about 1 in [\d,]+)$/);
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
    const { r, out } = await screenWith(Risks, 'risks');
    r.app.plan!.input.dials.horizon_years = 1;
    flushSync();
    const matrix = r.target.querySelector('section.matrix')!;
    expect(text(matrix.querySelector('thead'))).toContain('How likely (1 year)');
    // The golden output is fixed, so only the horizon changes: every row over one year.
    const nuclear = out.register.find((h) => h.id === 'nuclear_attack')!;
    expect(text(matrix)).toContain(rangeOnly(nuclear.rate_range[0], nuclear.rate_range[1], 1));
    const first = out.register[0]!;
    expect(text(matrix.querySelector('tbody tr td'))).toBe(likelyCell(first, 1));
  });
});

describe('the rare box (H-02)', () => {
  it('shows a range only, a header without "households like yours", and the nuclear note', async () => {
    const { r, out } = await screenWith(Risks, 'risks');
    const box = r.target.querySelector('section.rare')!;
    const header = text(box.querySelectorAll('thead th')[1]);
    expect(header).toBe('How likely (in the next 10 years)');
    const cells = [...box.querySelectorAll('tbody tr')].map((row) => text(row.querySelectorAll('td')[0]));
    const rare = out.register.filter((h) => h.display === 'rare_catastrophic');
    expect(cells).toEqual(rare.map((h) => rangeOnly(h.rate_range[0], h.rate_range[1], 10)));
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
    const { r, out } = await screenWith(Risks, 'risks');
    const t = out.buckets.find((b) => b.id === 'evacuate')!.target;
    if (t.kind !== 'evacuate') throw new Error('evacuate target');
    const notice = `Notice could be ${noticeRange(t.notice_hours_low, t.notice_hours_high)}`;
    expect(text(r.target)).toContain(notice);
    // v0.1.0 printed "1 minutes" here (0.02 hours).
    if (t.notice_hours_low * 60 < 1.5) expect(notice).toMatch(/Notice could be 1 minute\b/);
    expect(text(r.target)).not.toMatch(/(?<![\d.,])1 minutes/);
  });

  it('what helps fits the hazard (W4, W5), keeps acronyms (W6), and never prices what the household has', async () => {
    const { r, out } = await screenWith(Risks, 'risks');
    const cat = await realCatalogue();
    const helps = (id: string) => text(r.target.querySelector(`#hazard-${id} .helps`));
    expect(helps('heat_wave')).not.toMatch(/warm room/i);
    expect(helps('heat_wave')).toMatch(/cool room/i);
    expect(helps('cold_wave')).toMatch(/warm room/i);
    expect(helps('cold_wave')).not.toMatch(/fan|cool room/i);
    expect(helps('medical_emergency')).toMatch(/^What helps: family first-aid kit \((\$\d+|have it)\), bleeding-control kit/);
    expect(helps('medical_emergency')).not.toMatch(/N95/i);
    if (/N95/i.test(text(r.target))) expect(text(r.target)).toContain('N95 respirators');
    expect(text(r.target)).not.toMatch(/\bn95\b/);
    // Each card lists its first three, each with "free", a price, or "have it" / "done" when the
    // household already has it (the heat card's towels are an assumed everyday basic).
    let owned = 0;
    for (const h of out.register.filter((x) => x.display === 'ranked')) {
      for (const i of helpsFor(out, cat, h.id).slice(0, 3)) {
        const note = i.done ? (i.kind === 'free_action' ? 'done' : 'have it') : i.kind === 'free_action' ? 'free' : usd(i.est_cost_usd);
        expect(helps(h.id), `${h.id}: ${i.item_id}`).toContain(`${lowerFirst(i.name)} (${note})`);
        if (i.done) owned += 1;
      }
    }
    expect(owned, 'at least one card offers something the household already has').toBeGreaterThan(0);
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
    // Nothing is checked off yet: everything done is an assumed everyday basic (8 in v0.1.0).
    const had = all.filter((i) => i.done).length;
    expect(had).toBeGreaterThan(0);
    const stat = [...r.target.querySelectorAll('.stats li')].find((li) => li.textContent?.includes('Done so far'))!;
    expect(text(stat)).toBe(`Done so far 0 of ${all.length - had} steps Already have: ${had}`);
    const section = r.target.querySelector('#had-title')!.closest('section')!;
    expect(text(section.querySelector('h2'))).toBe(`Already have ${had}`);
    expect(text(section.querySelector('summary'))).toBe('Show what you already have');
    expect(r.target.querySelector('#done-title')).toBeNull();
  });

  it('sets cash aside instead of buying it', async () => {
    // Cash in small bills saved up for over a few months (Philadelphia's v0.1.0 plan has exactly
    // this; the envelope is added here if a regenerated plan buys the cash sooner).
    const out = golden(NAME);
    const cashLine = out.plan.months.flatMap((m) => m.items).find((i) => i.item_id === 'docs_cash_reserve' && i.kind === 'purchase');
    expect(cashLine, 'the plan sets cash aside').toBeDefined();
    if (!out.plan.envelopes.some((e) => e.item_id === 'docs_cash_reserve')) {
      out.plan.envelopes.push({ item_id: 'docs_cash_reserve', saved_usd: 0, needed_usd: cashLine!.est_cost_usd });
    }
    const { r } = await screenWith(PlanScreen, 'plan', NAME, out);
    const envelopes = [...r.target.querySelectorAll('.envelopes li')].map(text);
    const cash = envelopes.find((t) => t.startsWith('Cash in small bills'))!;
    expect(cash).toMatch(/all set aside around \w+ \d{4}/);
    expect(cash).not.toContain('ready to buy');
    // Things (not money) keep "ready to buy".
    for (const t of envelopes.filter((x) => !x.startsWith('Cash') && x.includes(' around '))) expect(t).toMatch(/ready to buy around/);
  });

  it('puts the nearer savings goal before the full goal, on the plan and the risks screens', async () => {
    const out = golden(NAME);
    const m = nextMilestone(out.plan.savings_track!, FIXTURES[NAME].planning_date, out.plan.done_month);
    // Philadelphia has half a month saved of a four-month goal: the first goal is one month.
    expect(m?.months).toBe(1);
    const want = `First goal: one month of expenses${m!.usd !== undefined ? `, about ${usd(m!.usd)}` : ''}${m!.by ? `, by ${formatMonth(m!.by)}` : ''}.`;
    for (const [Screen, route] of [
      [PlanScreen, 'plan'],
      [Risks, 'risks'],
    ] as const) {
      const { r } = await screenWith(Screen, route);
      const card = text(r.target.querySelector('[data-bucket="income"]'));
      expect(card, route).toContain(want);
      expect(card).toMatch(/Full goal: about [\d½]+ months?/);
      expect(card.indexOf('First goal')).toBeLessThan(card.indexOf('Full goal'));
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
    const track = (target_months: number, target_usd: number, current_months: number, monthly_suggestion_usd: number) => ({
      target_months,
      target_usd,
      current_months,
      monthly_suggestion_usd,
      why: '',
    });
    // Philadelphia in v0.1.0: half a month of $4,200 saved toward four, $60 a month from month 25:
    // $2,100 to go at $60 is 35 months, so month 60, October 2031.
    const philly = track(4, 16_800, 0.5, 60);
    expect(nextMilestone(philly, '2026-10-01', 25)).toEqual({ months: 1, first: true, usd: 4200, by: '2031-10-01' });
    // Phoenix in v0.1.0: 2 of 7 months ($2,600 each) saved, $40 a month from month 38: three months
    // next, $2,600 to go is 65 months.
    expect(nextMilestone(track(7, 18_200, 2, 40), '2026-10-01', 38)).toEqual({ months: 3, first: false, usd: 7800, by: addMonths('2026-10-01', 38 + 65) });
    // Nothing nearer when a goal is already met (Sugar Land, 4 of 2.5), or three months are saved (Coos Bay).
    expect(nextMilestone(track(2.5, 17_500, 4, 0), '2026-10-01', 2)).toBeNull();
    expect(nextMilestone(track(12, 45_600, 3, 150), '2026-10-01', 15)).toBeNull();
    // A full goal of one month or less has no nearer goal.
    expect(nextMilestone(track(1, 3000, 0, 50), '2026-10-01', 5)).toBeNull();
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
