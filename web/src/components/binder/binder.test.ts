/**
 * The binder on screen (DESIGN-DELTA-v3 §6), drawn from the hand-written fixture binder that holds
 * every page kind, block kind and inline kind: each block renders as itself; the household's text
 * prints exactly as typed, markup and Markdown marks included, never parsed; page anchors match
 * page ids; the contents list every page; and on the Binder screen `#/binder/<page-id>` opens a page.
 */
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import type { Engine } from '../../engine/index';
import { FIXTURES } from '../../engine/fixtures';
import { createMockEngine } from '../../engine/mock';
import type { PlanOutput } from '../../engine/types';
import { AWKWARD, FIXTURE_BINDER } from '../../lib/binder/fixture';
import { BLOCK_KINDS, binderPages, blockKind, flatBlocks, pageDomId, sourceDomId } from '../../lib/binder/model';
import { PAGE_KINDS } from '../../engine/types';
import Binder from '../../screens/Binder.svelte';
import { render, savedFor, until, type Rendered } from '../../test/helpers';
import { golden } from '../../test/real';
import BinderPage from './BinderPage.svelte';
import BinderToc from './BinderToc.svelte';
import { plainView } from './view';

const cleanups: (() => void)[] = [];
afterEach(() => {
  while (cleanups.length) cleanups.pop()!();
});

/** Every page of the fixture binder, mounted one after another. */
function drawAll(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  const view = plainView(FIXTURE_BINDER);
  const comps = binderPages(FIXTURE_BINDER).map((entry) => mount(BinderPage, { target, props: { entry, view } }));
  flushSync();
  cleanups.push(() => {
    comps.forEach((c) => unmount(c));
    target.remove();
  });
  return target;
}

describe('the fixture binder covers the model', () => {
  it('holds every page kind and every block kind', () => {
    const pages = binderPages(FIXTURE_BINDER);
    expect(new Set(pages.map((e) => e.page.kind))).toEqual(new Set(PAGE_KINDS));
    const kinds = new Set(pages.flatMap((e) => flatBlocks(e.page.blocks).map(blockKind)));
    expect(kinds).toEqual(new Set(BLOCK_KINDS));
  });
});

describe('the binder drawn on screen', () => {
  it('draws each kind of block as itself', () => {
    const el = drawAll();
    expect(el.querySelectorAll('article.binder-page')).toHaveLength(binderPages(FIXTURE_BINDER).length);
    // Headings sit under the page title (h3), from h4 down.
    expect([...el.querySelectorAll('h4')].map((h) => h.textContent)).toContain('Do first');
    // Airline steps: the memory items marked and bold, the rest plain, numbered in order.
    const fire = el.querySelector(`#${pageDomId('check_house_fire')}`)!;
    const memory = fire.querySelectorAll('.steps .step--memory');
    expect(memory.length).toBe(5);
    expect(memory[0]!.textContent).toContain('From memory:');
    expect(fire.querySelectorAll('.steps .step:not(.step--memory)').length).toBe(5);
    // Leave or stay: an If / Then table, with the page to turn to as a link.
    const decision = fire.querySelector('.decision__table')!;
    expect([...decision.querySelectorAll('th')].map((th) => th.textContent)).toEqual(['If', 'Then']);
    const turn = decision.querySelector('a.turn-to')!;
    expect(turn.textContent).toBe('Turn to Tab 3, Neighborhood');
    expect(turn.getAttribute('href')).toBe('#/binder/neighbourhood');
    // Fields: the answer as written, or ruled lines to write on (as many as the row asks for).
    const person = el.querySelector(`#${pageDomId('person_2')}`)!;
    const conditions = [...person.querySelectorAll('.field')].find((f) => f.querySelector('dt')?.textContent === 'Conditions')!;
    expect(conditions.querySelector('dd')?.textContent).toBe('Asthma, mild');
    const blankField = [...person.querySelectorAll('.field')].find((f) => f.querySelector('dt')?.textContent === 'Insurance')!;
    expect(blankField.querySelector('dd.ruled')).not.toBeNull();
    expect((blankField.querySelector('dd.ruled') as HTMLElement).style.getPropertyValue('--lines')).toBe('2');
    // A table with a header row, and empty cells left to write in.
    expect(person.querySelectorAll('table thead th')).toHaveLength(4);
    expect(person.querySelectorAll('td.write-in').length).toBeGreaterThan(0);
    // Callouts say their kind in a word.
    expect(el.querySelector('.callout--stop .callout__word')?.textContent).toBe('STOP');
    expect(el.querySelector('.callout--warning .callout__word')?.textContent).toBe('WARNING');
    expect(el.querySelector('.callout--note .callout__word')?.textContent).toBe('NOTE');
    expect(el.querySelector('.callout--decision .callout__word')?.textContent).toBe('DECIDE');
    // Maps: none added here, so one line says so for each slot.
    expect(el.querySelectorAll('.binder-map--missing')).toHaveLength(3);
    expect(el.querySelector('.binder-map--missing')?.textContent).toContain('no map added');
    // Wallet cards, logs and page breaks.
    expect(el.querySelectorAll('.wallet-card')).toHaveLength(4);
    expect(el.querySelector(`#${pageDomId('log_damage')} tbody`)?.querySelectorAll('tr')).toHaveLength(14);
    expect(el.querySelectorAll('.binder-break')).toHaveLength(1);
    // Blanks are a line to write on, with words for a screen reader.
    expect(el.querySelector('.blank .visually-hidden')?.textContent).toContain('blank');
  });

  it("prints the household's words exactly, never as markup or Markdown", () => {
    const el = drawAll();
    const text = el.textContent ?? '';
    for (const s of [AWKWARD.markup, AWKWARD.markdown, AWKWARD.bar, AWKWARD.accents, AWKWARD.long]) expect(text).toContain(s);
    // "<b>Rosa</b>" stays text: no element was made from it, and no Markdown link either.
    expect([...el.querySelectorAll('b')].some((b) => b.textContent === 'Rosa')).toBe(false);
    expect([...el.querySelectorAll('a')].some((a) => a.getAttribute('href') === 'a link' || a.textContent === 'not')).toBe(false);
    expect(el.querySelector('script, iframe, img')).toBeNull();
  });

  it('gives each page its anchor and each source its number; citations and links point at them', () => {
    const el = drawAll();
    for (const e of binderPages(FIXTURE_BINDER)) expect(el.querySelector(`#${pageDomId(e.page.id)}`)?.getAttribute('data-page')).toBe(e.page.id);
    for (const s of FIXTURE_BINDER.sources) expect(el.querySelector(`#${sourceDomId(s.n)}`), `source ${s.n}`).not.toBeNull();
    for (const a of el.querySelectorAll('.cite a')) expect(el.querySelector(a.getAttribute('href')!)).not.toBeNull();
    for (const a of el.querySelectorAll('a.xref')) expect(a.getAttribute('href')).toMatch(/^#\/binder\/[a-z0-9_]+$/);
    // A link inside a sentence reads "(Tab 9, …)"; a link alone in a cell is just the link.
    const fire = el.querySelector(`#${pageDomId('check_house_fire')}`)!;
    expect(fire.textContent).toContain('(Tab 9, After a disaster: the first 30 days)');
    const index = el.querySelector(`#${pageDomId('index')}`)!;
    expect(index.querySelector('td:last-child')?.textContent).toBe('Tab 6, Gas leak or carbon monoxide alarm');
  });

  it('lists every part and page in the contents, each a link to its page', () => {
    const target = document.createElement('div');
    document.body.append(target);
    const c = mount(BinderToc, { target, props: { binder: FIXTURE_BINDER, current: 'home' } });
    flushSync();
    cleanups.push(() => {
      unmount(c);
      target.remove();
    });
    const links = [...target.querySelectorAll('.binder-toc__pages a')];
    expect(links.map((a) => a.getAttribute('href'))).toEqual(binderPages(FIXTURE_BINDER).map((e) => `#/binder/${e.page.id}`));
    expect(target.querySelectorAll('.binder-toc__partlink')).toHaveLength(FIXTURE_BINDER.parts.length);
    expect(target.querySelector('[aria-current="location"]')?.textContent?.trim()).toBe('Home');
  });
});

describe('the Binder screen', () => {
  let r: Rendered | null = null;
  afterEach(() => {
    r?.cleanup();
    r = null;
  });

  it('opens at the page an address names, and at the wallet cards for the old address', async () => {
    r = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder/getting_out' });
    await until(() => (document.activeElement?.textContent ?? '') === 'Getting out', 'focus on Getting out');
    expect(r.target.querySelector('[aria-current="location"]')?.textContent?.trim()).toBe('Getting out');
    r.cleanup();
    r = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder/wallet-cards' });
    await until(() => (document.activeElement?.textContent ?? '') === 'Wallet cards', 'focus on the wallet cards');
  });

  it('offers Download PDF with Letter or A4 and two-sided printing, and Print', async () => {
    r = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder' });
    const radios = [...r.target.querySelectorAll<HTMLInputElement>('input[name="paper"]')].map((i) => i.value);
    expect(radios).toEqual(['LETTER', 'A4']);
    expect(r.text()).toContain('Printing on both sides of the paper');
    expect([...r.target.querySelectorAll('button')].map((b) => b.textContent?.trim())).toEqual(expect.arrayContaining(['Download PDF', 'Print', 'Add maps']));
    // Each page can be printed alone.
    expect(r.target.querySelectorAll('.binder-page__head .button').length).toBe(r.target.querySelectorAll('.binder-page').length);
  });
});

/** The mock engine, except that `assess` answers with the real engine's output. */
function answering(output: PlanOutput): Engine {
  const mock = createMockEngine();
  return { ...mock, assess: async () => ({ ok: true, value: output }) };
}

describe("the engine's Philadelphia binder on the Binder screen", () => {
  let r: Rendered | null = null;
  afterEach(() => {
    r?.cleanup();
    r = null;
  });

  it('draws every page, every link and citation lands, and the answers print as typed', async () => {
    const output = golden('philadelphia-renters-4');
    const binder = output.binder;
    r = await render(Binder, { plan: savedFor(FIXTURES['philadelphia-renters-4']), route: 'binder', engine: answering(output) });
    const pages = binderPages(binder);
    const drawn = [...r.target.querySelectorAll<HTMLElement>('article.binder-page')];
    expect(drawn.map((a) => a.dataset.page)).toEqual(pages.map((e) => e.page.id));
    // Every cross-reference names a page of this binder; every citation a numbered source.
    const ids = new Set([...r.target.querySelectorAll('[id]')].map((e) => e.id));
    const xrefs = [...r.target.querySelectorAll('a.xref')].map((a) => a.getAttribute('href')!);
    expect(xrefs.length).toBeGreaterThan(50);
    expect(xrefs.filter((h) => !ids.has(pageDomId(h.replace('#/binder/', ''))))).toEqual([]);
    const cites = [...r.target.querySelectorAll('.cite a')].map((a) => a.getAttribute('href')!);
    expect(cites.length).toBeGreaterThan(100);
    expect(cites.filter((h) => !ids.has(h.slice(1)))).toEqual([]);
    // The Sources page's own numbered list carries the anchors, one per source.
    expect(binder.sources.filter((s) => !ids.has(sourceDomId(s.n))).map((s) => s.n)).toEqual([]);
    // The household's answers, exactly as the engine passed them on.
    const values = pages.flatMap((e) => flatBlocks(e.page.blocks).flatMap((bl) => ('fields' in bl ? bl.fields.flatMap((f) => (f.value ? [f.value] : [])) : [])));
    expect(values.length).toBeGreaterThan(20);
    const text = r.target.textContent ?? '';
    for (const v of values) expect(text, v).toContain(v);
    // Three map slots, none filled yet: one line each, never an empty frame.
    expect(r.target.querySelectorAll('.binder-map--missing')).toHaveLength(3);
    // A wallet card for each person; nothing printed as a stray value.
    const cards = pages.flatMap((e) => e.page.blocks).reduce((n, bl) => n + ('cards' in bl ? bl.cards.length : 0), 0);
    expect(cards).toBe(FIXTURES['philadelphia-renters-4'].people.length);
    expect(r.target.querySelectorAll('.wallet-card')).toHaveLength(cards);
    expect(text).not.toMatch(/undefined|\[object Object\]|NaN/);
  });
});
