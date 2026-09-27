/**
 * Verification round 3 (docs/VERIFICATION.md, v0.2.0): the web fixes made on agent/verify2, each
 * checked on the screens a household sees. Real engine output comes from the committed goldens
 * (fixtures/golden, the engine's own output) through a mock engine whose `assess` answers with it.
 *
 * - Hidden words keep their space: Svelte drops the space at the start or end of a text node inside
 *   an element, so a visually hidden " for …" or " (opens in a new tab)" was read as "sourcesfor …".
 * - The legal-emergency switch in Your settings (`Dials.legal_opt_in`), off by default, and the
 *   source of its bail figures on the savings card.
 * - Decisions grouped on the Plan screen the way the packet groups them.
 * - Web copy keeps to the content policy's pressure-phrase list (rr-content `BANNED_PHRASES`).
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import type { Engine } from '../engine/index';
import { FIXTURES } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import type { PlanOutput } from '../engine/types';
import { render, savedFor, until, type Rendered } from '../test/helpers';
import { golden, repoRoot } from '../test/real';
import About from './About.svelte';
import PlanScreen from './PlanScreen.svelte';
import Risks from './Risks.svelte';
import Where from './Where.svelte';

let current: Rendered | undefined;
afterEach(() => {
  current?.cleanup();
  current = undefined;
  window.location.hash = '';
});

const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

/** The mock engine, except that `assess` answers with this output (the real engine's, from a golden). */
function answering(output: PlanOutput): Engine {
  const mock = createMockEngine();
  return { ...mock, assess: async () => ({ ok: true, value: output }) };
}

describe('hidden words keep their space', () => {
  it('in the sources disclosures: "sources for …", "Sources (8) for …"', async () => {
    current = await render(PlanScreen, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'plan' });
    const inline = [...current.target.querySelectorAll('details.sources--inline > summary')];
    expect(inline.length).toBeGreaterThan(0);
    for (const s of inline) expect(s.textContent!.trim()).toMatch(/^sources for \S.* \(\d+\)$/);
    current.cleanup();
    current = await render(Risks, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'risks' });
    const footers = [...current.target.querySelectorAll('article.gauge details.sources--footer > summary')];
    expect(footers.length).toBeGreaterThan(0);
    for (const s of footers) expect(s.textContent!.trim()).toMatch(/^Sources \(\d+\) for \S/);
    // A source's link, once the list is open: "… (opens in a new tab)".
    const first = current.target.querySelector('article.gauge details.sources') as HTMLDetailsElement;
    first.open = true;
    first.dispatchEvent(new Event('toggle'));
    await until(() => !!first.querySelector('a[target="_blank"]'), 'the open source list');
    for (const a of first.querySelectorAll('a[target="_blank"]')) expect(a.textContent).toMatch(/\S \(opens in a new tab\)$/);
  });

  it('on About: every link that opens a new tab says so after a space', async () => {
    current = await render(About, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'about' });
    const links = [...current.target.querySelectorAll('a[target="_blank"]')];
    expect(links.length).toBeGreaterThan(5);
    for (const a of links) expect(a.textContent, a.getAttribute('href') ?? '').toMatch(/\S \(opens in a new tab\)$/);
    expect(text(current.target)).toContain('Report a wrong number on GitHub (opens in a new tab)');
  });

  it('in the interview steps: "Where you live (answered)"', async () => {
    current = await render(Where, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'where' });
    // What a screen reader reads: the link's text without the aria-hidden step number or tick.
    const spoken = (a: Element) => {
      const copy = a.cloneNode(true) as Element;
      copy.querySelectorAll('[aria-hidden="true"]').forEach((n) => n.remove());
      return copy.textContent!.trim();
    };
    const steps = [...current.target.querySelectorAll('nav.steps a')].map(spoken);
    expect(steps[0]).toBe('Where you live (answered)');
    expect(steps.every((s) => s.endsWith(' (answered)'))).toBe(true);
  });
});

describe('the legal-emergency switch (Dials.legal_opt_in)', () => {
  const box = (r: Rendered) =>
    [...r.target.querySelectorAll<HTMLInputElement>('#settings-panel input[type="checkbox"]')].find((i) =>
      (i.closest('label')?.textContent ?? '').includes("Also save toward a legal emergency (bail, a lawyer's retainer)"),
    );

  it('sits in Your settings, off by default, and writes the dial only when turned on', async () => {
    current = await render(Risks, { plan: savedFor(FIXTURES['minot-missile-field-3']), route: 'risks' });
    const r = current;
    (r.target.querySelector('button[aria-controls="settings-panel"]') as HTMLButtonElement).click();
    flushSync();
    const input = box(r)!;
    expect(input, 'the switch').toBeDefined();
    expect(input.checked).toBe(false);
    expect(r.app.plan!.input.dials.legal_opt_in).toBeUndefined();
    // Right after the rare-family list.
    expect(input.closest('label')!.previousElementSibling?.matches('fieldset.rare-opt-in')).toBe(true);
    expect(text(input.closest('label'))).toContain('It never takes money from your supplies budget.');
    input.click();
    flushSync();
    expect(r.app.plan!.input.dials.legal_opt_in).toBe(true);
    expect(text(r.target.querySelector('.settings__summary'))).toContain('also saving toward a legal emergency');
    input.click();
    flushSync();
    // Off leaves the key out of the saved plan, as the engine writes it.
    expect('legal_opt_in' in r.app.plan!.input.dials).toBe(false);
    expect(text(r.target.querySelector('.settings__summary'))).not.toContain('legal emergency');
  });

  it('adds the source of the bail figures to the savings card when the engine prints the line', async () => {
    // The real Minot plan, as the engine answers with the dial on: the legal sentence in the
    // savings track and its source in the provenance (rr-budget savings.rs, legal_sentence).
    const off = golden('minot-missile-field-3');
    const on = golden('minot-missile-field-3');
    on.plan.savings_track!.why += ' Apart from these months: an arrest in the household can mean paying bail and a lawyer.';
    on.provenance.push({
      id: 'bjs_felony_defendants_2009',
      title: 'Felony Defendants in Large Urban Counties, 2009: Statistical Tables',
      publisher: 'Reaves B.A., Bureau of Justice Statistics',
      year: 2013,
      url: 'https://bjs.ojp.gov/content/pub/pdf/fdluc09.pdf',
      retrieved: '2026-09-26',
      license: 'US Government Work (public domain)',
    });
    const incomeSources = off.buckets.find((b) => b.id === 'income')!.sources.length;
    const card = (r: Rendered) => r.target.querySelector('[data-bucket="income"]')!;

    const input = { ...FIXTURES['minot-missile-field-3'], dials: { ...FIXTURES['minot-missile-field-3'].dials, legal_opt_in: true } };
    current = await render(Risks, { plan: savedFor(input), route: 'risks', engine: answering(on) });
    expect(text(card(current))).toContain('an arrest in the household can mean paying bail and a lawyer');
    expect(text(card(current).querySelector('details.sources > summary'))).toBe(`Sources (${incomeSources + 1})`);
    current.cleanup();

    current = await render(Risks, { plan: savedFor(FIXTURES['minot-missile-field-3']), route: 'risks', engine: answering(off) });
    expect(text(card(current).querySelector('details.sources > summary'))).toBe(`Sources (${incomeSources})`);
  });
});

describe('decisions on the Plan screen, grouped as the packet groups them', () => {
  const hays = 'hays-kansas-farm-5';
  /** The packet's own one-line list of month 1's decisions. */
  function packetLine(name: string): string {
    const md = readFileSync(join(repoRoot(), 'fixtures', 'golden', `${name}.md`), 'utf8');
    return /\*\*Decide this month\*\* \(see Documents and money\): (.*)\.$/m.exec(md)![1]!;
  }

  it('lists next month’s decisions on one line (Hays: eight), not one line each', async () => {
    const out = golden(hays);
    const decisions = out.plan.months[1]!.items.filter((i) => i.decision);
    expect(decisions).toHaveLength(8);
    current = await render(PlanScreen, { plan: savedFor(FIXTURES[hays]), route: 'plan', engine: answering(out) });
    const lines = [...current.target.querySelector('#next-title')!.closest('section')!.querySelectorAll('ul.preview > li')].map(text);
    expect(lines.filter((l) => l.startsWith('Decide'))).toEqual([`Decide (8): ${packetLine(hays)}. No cost to your supplies budget.`]);
    expect(lines).toHaveLength(out.plan.months[1]!.items.filter((i) => !i.done).length - 8 + 1);
    // The whole plan's month 2 also gives them one line.
    const month = [...current.target.querySelectorAll('details.month')].find((d) => text(d.querySelector('summary')).startsWith('Next month'))!;
    const decideLines = [...month.querySelectorAll('.month__list > li')].map(text).filter((l) => l.startsWith('Decide'));
    expect(decideLines).toEqual([`Decide (8): ${packetLine(hays)} free Decisions`]);
  });

  it('gives this month’s decisions one block, each still on its own card to tick off', async () => {
    const out = golden(hays);
    // A month into the plan, month 1 is this month.
    current = await render(PlanScreen, { plan: savedFor(FIXTURES[hays]), route: 'plan', engine: answering(out), today: '2026-11-01' });
    const section = current.target.querySelector('#this-title')!.closest('section')!;
    expect(text(section.querySelector('#decide-title'))).toBe('Decide this month (8)');
    expect(text(section.querySelector('#decide-title + p'))).toBe(
      `Choices about insurance, papers and your home. They cost nothing from your supplies budget: ${packetLine(hays)}.`,
    );
    const cards = [...section.querySelectorAll('details.decisions article.item')];
    expect(cards).toHaveLength(8);
    for (const c of cards) expect(text(c.querySelector('.item__check label'))).toMatch(/^Decided: Decide: /);
    // The free steps above no longer count them.
    const free = out.plan.months[1]!.items.filter((i) => i.kind === 'free_action' && !i.decision && !i.done).length;
    expect(text([...section.querySelectorAll('h3')].find((h) => text(h).startsWith('Free steps')))).toBe(`Free steps (${free})`);
  });
});

describe('web copy keeps to the content policy', () => {
  /** rr-content's list of pressure phrases (docs/CONTENT_STANDARDS.md §4), read from the Rust source. */
  function bannedPhrases(): string[] {
    const src = readFileSync(join(repoRoot(), 'crates', 'rr-content', 'src', 'policy.rs'), 'utf8');
    const list = /pub const BANNED_PHRASES: &\[&str\] = &\[([\s\S]*?)\];/.exec(src)![1]!;
    return [...list.matchAll(/"([^"]+)"/g)].map((m) => m[1]!);
  }
  /** Lower-case words, apostrophes and hyphens splitting them, as the validator tokenises. */
  const words = (s: string) => ` ${(s.toLowerCase().match(/[a-z0-9µ]+/g) ?? []).join(' ')} `;

  function sourceFiles(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return sourceFiles(path);
      return /\.(svelte|ts)$/.test(name) && !name.endsWith('.test.ts') ? [path] : [];
    });
  }

  it('uses none of the pressure phrases in any screen, component, label or the stand-in engine', () => {
    const banned = bannedPhrases();
    expect(banned).toContain('hurry');
    const hits: string[] = [];
    for (const file of sourceFiles(join(repoRoot(), 'web', 'src'))) {
      readFileSync(file, 'utf8')
        .split('\n')
        .forEach((line, i) => {
          for (const b of banned) if (words(line).includes(` ${b} `)) hits.push(`${file.slice(repoRoot().length + 1)}:${i + 1} "${b}"`);
        });
    }
    expect(hits).toEqual([]);
  });
});
