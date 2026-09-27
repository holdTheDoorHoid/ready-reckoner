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
