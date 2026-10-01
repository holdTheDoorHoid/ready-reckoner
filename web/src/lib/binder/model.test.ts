/**
 * Reading the binder tree (`model.ts`): where an address leads, the structural checks, the
 * contents, the fit proxy of DESIGN-DELTA-v3 §5.6 and the PDF's file name.
 */
import { describe, expect, it } from 'vitest';

import type { Binder } from '../../engine/types';
import { FIXTURE_BINDER } from './fixture';
import { binderPages, binderProblems, countySlug, findPage, fitUnits, pdfFileName, slugify, sourceListIndex, tableOfContents } from './model';
import { missingCharacters } from './pdf/coverage';

describe('the binder model', () => {
  it('finds a page by id, by its id written with hyphens, or by kind; the old wallet-cards address works', () => {
    expect(findPage(FIXTURE_BINDER, 'check_house_fire')?.page.id).toBe('check_house_fire');
    expect(findPage(FIXTURE_BINDER, 'wallet-cards')?.page.kind).toBe('wallet_cards');
    expect(findPage(FIXTURE_BINDER, 'Wallet-Cards')?.page.kind).toBe('wallet_cards');
    expect(findPage(FIXTURE_BINDER, 'risks-glance')?.page.id).toBe('risks_glance');
    expect(findPage(FIXTURE_BINDER, 'no-such-page')).toBeUndefined();
    expect(findPage(FIXTURE_BINDER, undefined)).toBeUndefined();
  });

  it('passes the structural checks, and names what breaks them', () => {
    expect(binderProblems(FIXTURE_BINDER)).toEqual([]);
    const broken = structuredClone(FIXTURE_BINDER) as Binder;
    broken.parts[1]!.short_title = 'Far too long a tab label';
    broken.parts[2]!.pages[0]!.blocks.push({ para: [{ link: { to: 'nowhere', text: 'Tab 0' } }, { cite: [999] }] });
    broken.parts[3]!.pages.push({ ...broken.parts[3]!.pages[0]! });
    expect(binderProblems(broken)).toEqual([
      'part people: tab label "Far too long a tab label" is over 14 characters',
      'page pets used twice',
      'page home: link to nowhere names no page',
      'page home: cite 999 is not a source',
    ]);
  });

  it('lists every part and page in order for the contents', () => {
    const toc = tableOfContents(FIXTURE_BINDER);
    expect(toc.map((p) => p.tab)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(toc.flatMap((p) => p.pages.map((pg) => pg.id))).toEqual(binderPages(FIXTURE_BINDER).map((e) => e.page.id));
    expect(toc.flatMap((p) => p.pages.map((pg) => pg.index))).toEqual(binderPages(FIXTURE_BINDER).map((_, i) => i));
  });

  it('counts a page as rr-plan does (§5.6): headings 6, steps and bullets their words plus 2, fields 8, table rows 10', () => {
    const page = { id: 'p', title: 'P', kind: 'checklist' as const, fit: 'one' as const, blocks: [
      { heading: { level: 1, text: 'Do first' } },
      { steps: [{ text: [{ t: 'Get out now.' }], memory: true }] },
      { bullets: [[{ b: 'Do not' }, { t: ' go back in.' }]] },
      { fields: [{ label: 'Meet', lines: 1 }, { label: 'Call', value: '555', lines: 1 }] },
      { table: { header: ['A'], rows: [[[{ t: 'x' }]], [[{ t: 'y' }]]] } },
      { para: [{ t: 'Three plain words.' }] },
    ] };
    expect(fitUnits(page)).toBe(6 + (3 + 2) + (5 + 2) + 16 + 30 + 3);
  });

  it('names the PDF after the county and the date', () => {
    expect(slugify('Doña Ana County')).toBe('dona-ana-county');
    expect(countySlug(FIXTURE_BINDER, { state_abbr: 'PA' })).toBe('philadelphia-county-pa');
    expect(pdfFileName(FIXTURE_BINDER, { state_abbr: 'PA' })).toBe('ready-reckoner-binder-philadelphia-county-pa-2026-10-01.pdf');
    expect(pdfFileName({ ...FIXTURE_BINDER, location: '' }, null)).toBe('ready-reckoner-binder-binder-2026-10-01.pdf');
  });

  it('finds the list of numbered sources on the Sources page when the engine prints one', () => {
    const page = binderPages(FIXTURE_BINDER).find((e) => e.page.kind === 'sources')!.page;
    expect(sourceListIndex(page, FIXTURE_BINDER)).toBe(-1);
    const withList = { ...page, blocks: [{ para: [{ t: 'Sources.' }] }, { numbered: FIXTURE_BINDER.sources.map((s) => [{ t: s.title }]) }] };
    expect(sourceListIndex(withList, FIXTURE_BINDER)).toBe(1);
  });
});

describe("the PDF's typeface", () => {
  it('draws Latin, Latin-1, Latin Extended-A and the letters names here use; names the rest', () => {
    expect(missingCharacters(['Zoë Łukasiewicz-Nguyễn, Ștefan, José, Ğ, ā, ʻokina ʻ, 1–2 “quotes”, €5'])).toEqual([]);
    // Decomposed accents are composed first: "e" + combining acute is "é".
    expect(missingCharacters(['José'])).toEqual([]);
    expect(missingCharacters(['Иван', '王', 'Ολγα', '→'])).toEqual(['И', 'в', 'а', 'н', '王', 'Ο', 'λ', 'γ', 'α', '→']);
  });
});
