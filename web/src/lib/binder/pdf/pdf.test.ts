/**
 * The PDF binder, drawn in Node with the same pdfmake build the browser loads
 * (DESIGN-DELTA-v3 §6), read back with a small PDF reader (`src/test/read-pdf.ts`):
 *
 * - it builds on Letter and A4; every binder page starts its own sheet and is a named
 *   destination; the table of contents lists every part and page with its number, each line a
 *   link; cross-references and "Turn to" cells are links; the footers run "page N of M" with the
 *   version and dates; the tab label sheet is the last page;
 * - printed on both sides, every part and the label sheet start on a right-hand page, and the
 *   wallet cards keep a blank back;
 * - the household's text comes out exactly as typed, and a cross-reference in a sentence gives
 *   the page it points to;
 * - a page that promises one sheet (or two) takes no more, on the fixture binder and on the
 *   engine's Philadelphia binder (the golden), whose page count is checked against a range and
 *   logged tab by tab beside the engine's fit proxy.
 *
 * Set RR_PDF_OUT to a folder to keep the files for a look, and RR_GOLDEN_DIR to a folder of
 * golden plans to draw instead of `fixtures/golden` (another branch's goldens, say).
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

import type { PdfMakeInstance } from 'pdfmake/build/pdfmake.min.js';
import { describe, expect, it } from 'vitest';

import type { Binder } from '../../../engine/types';
import { readPdf, type ReadPdf } from '../../../test/read-pdf';
import { AWKWARD, FIXTURE_BINDER } from '../fixture';
import { binderPages, FIT_CAPACITY, fitUnits } from '../model';
import { type Paper, type PdfOptions, sheetFill, sheetRange, sheetsAllowed } from './doc';
import { LABEL_SPEC } from './labels';
import { FONT_FILES, type FontFiles, renderBinderPdf, type RenderedPdf } from './render';

const require = createRequire(import.meta.url);
const pdfMake = require('pdfmake/build/pdfmake.min.js') as PdfMakeInstance;
const FONT_DIR = join(process.cwd(), 'src', 'lib', 'binder', 'pdf', 'fonts');
const fonts = Object.fromEntries(Object.entries(FONT_FILES).map(([style, file]) => [style, new Uint8Array(readFileSync(join(FONT_DIR, file)))])) as FontFiles;

const OUT = process.env.RR_PDF_OUT;

async function draw(b: Binder, opts: Partial<PdfOptions> & { paper: Paper }, name: string): Promise<{ r: RenderedPdf; pdf: ReadPdf }> {
  const r = await renderBinderPdf(pdfMake, fonts, b, { doubleSided: false, appVersion: '0.3.0+test', ...opts });
  if (OUT) {
    mkdirSync(OUT, { recursive: true });
    writeFileSync(`${OUT}/${name}.pdf`, r.bytes);
  }
  return { r, pdf: readPdf(r.bytes) };
}

/** Text with the PDF's line breaks flattened, for searching. */
const flat = (s: string) => s.replace(/\s+/g, ' ');

describe('the PDF binder, from the fixture binder', () => {
  const sizes = { LETTER: [612, 792], A4: [595, 842] } as const;

  for (const paper of ['LETTER', 'A4'] as const) {
    it(`builds on ${paper === 'LETTER' ? 'Letter' : 'A4'}: a sheet per page, the contents with numbers and links, footers, the label sheet last`, async () => {
      const { r, pdf } = await draw(FIXTURE_BINDER, { paper }, `fixture-${paper.toLowerCase()}`);
      const pages = binderPages(FIXTURE_BINDER);
      for (const p of pdf.pages) expect(p.mediaBox.slice(2).map(Math.round), paper).toEqual([...sizes[paper]]);
      // Every binder page is a named destination on the sheet it starts, in order.
      const starts = pages.map((e) => pdf.destinations.get(`pg-${e.page.id}`));
      expect(starts.every((n) => typeof n === 'number' && n > 0)).toBe(true);
      for (let i = 1; i < starts.length; i++) expect(starts[i]!, pages[i]!.page.id).toBeGreaterThan(starts[i - 1]!);
      for (const s of r.doc.sheets) if (s.entry) expect(sheetRange(s)![0]).toBe(pdf.destinations.get(`pg-${s.entry.page.id}`));
      // The contents: right after "How to use", every part and page title, each line a link.
      const toc = r.doc.sheets.find((s) => s.key === 'toc')!;
      const [from, to] = sheetRange(toc)!;
      expect(from).toBe(pdf.destinations.get('pg-how_to_use')! + 1);
      const tocText = flat(pdf.pages.slice(from - 1, to).map((p) => p.text).join('\n'));
      for (const part of FIXTURE_BINDER.parts) expect(tocText, part.title).toContain(`Tab ${part.tab}`);
      for (const e of pages) expect(tocText, e.page.id).toContain(flat(e.page.kind === 'cover' ? 'Cover' : e.page.title));
      const tocLinks = pdf.pages.slice(from - 1, to).flatMap((p) => p.links);
      for (const e of pages) expect(tocLinks.some((l) => l.dest === `pg-${e.page.id}`), e.page.id).toBe(true);
      // The page numbers in the contents are the real ones: "Wallet cards ... 12".
      const wallet = pdf.destinations.get('pg-wallet_cards')!;
      expect(tocText).toMatch(new RegExp(`Wallet cards ?${wallet}\\b`));
      // Cross-references are links: "Which checklist?" to the checklists, a decision to its page.
      const index = pdf.pages[pdf.destinations.get('pg-index')! - 1]!;
      expect(index.links.some((l) => l.dest === 'pg-check_house_fire')).toBe(true);
      const fire = pdf.pages[pdf.destinations.get('pg-check_house_fire')! - 1]!;
      expect(fire.links.some((l) => l.dest === 'pg-neighbourhood')).toBe(true);
      expect(fire.links.some((l) => l.dest?.startsWith('src-'))).toBe(true);
      // On paper a cross-reference gives its page: the number the page really has.
      expect(flat(fire.text)).toContain(`(Tab 9, After a disaster: the first 30 days, page ${pdf.destinations.get('pg-after')})`);
      // Headers carry the tab number and label; footers the running page number and the dates.
      expect(flat(fire.text)).toContain('Happening now');
      expect(flat(fire.text)).toContain(`page ${fire.number} of ${pdf.pages.length}`);
      expect(flat(fire.text)).toContain('Ready Reckoner 0.3.0+test · made Oct 1, 2026 · review by Oct 1, 2027');
      // The tab label sheet is the last page: each tab's number and label, twice, and the note.
      const last = flat(pdf.pages.at(-1)!.text);
      expect(last).toContain('Tab labels');
      expect(last).toContain('Or write the titles on the tabs by hand');
      for (const part of FIXTURE_BINDER.parts) expect(last.split(part.short_title).length - 1, part.short_title).toBeGreaterThanOrEqual(LABEL_SPEC.copies);
      // The household's words, exactly as typed (accents in Latin Extended included).
      const all = flat(pdf.pages.map((p) => p.text).join('\n'));
      for (const s of [AWKWARD.markup, AWKWARD.markdown, AWKWARD.bar, AWKWARD.accents]) expect(all).toContain(s);
      expect(all).not.toMatch(/page 000\b|\b00000\b/);
      expect(pdf.fonts.every((f) => /\+NotoSans-/.test(f))).toBe(true);
      expect(pdf.info.Title).toBe(FIXTURE_BINDER.title);
    });
  }

  it('printed on both sides: every part and the labels start on a right-hand page; the wallet cards keep a blank back', async () => {
    const { r, pdf } = await draw(FIXTURE_BINDER, { paper: 'LETTER', doubleSided: true }, 'fixture-letter-double');
    for (const part of FIXTURE_BINDER.parts) {
      const n = pdf.destinations.get(`pg-${part.pages[0]!.id}`)!;
      expect(n % 2, `${part.id} starts on page ${n}`).toBe(1);
    }
    expect(pdf.destinations.get('rr-labels')! % 2).toBe(1);
    const cards = r.doc.sheets.find((s) => s.entry?.page.kind === 'wallet_cards')!;
    const [, end] = sheetRange(cards)!;
    expect(end % 2).toBe(1);
    expect(flat(pdf.pages[end]!.text)).toContain('This page is blank on purpose.');
  });

  it('keeps every one-sheet page to one sheet', async () => {
    const { r } = await draw(FIXTURE_BINDER, { paper: 'LETTER' }, 'fixture-fit');
    for (const s of r.doc.sheets) {
      if (!s.entry) continue;
      const f = sheetFill(s)!;
      expect(f.pages, `${s.entry.page.id} (${s.entry.page.fit}, scale ${s.scale.toFixed(2)})`).toBeLessThanOrEqual(sheetsAllowed(s.entry.page));
    }
  });
});

// ---------------------------------------------------------------------------------------------
// The engine's binder: the Philadelphia golden
// ---------------------------------------------------------------------------------------------

function repoRoot(): string {
  for (let dir = process.cwd(), i = 0; i < 6; i++, dir = dirname(dir)) if (existsSync(join(dir, 'fixtures', 'golden'))) return dir;
  throw new Error('no repository root');
}

const GOLDEN_DIR = process.env.RR_GOLDEN_DIR ?? join(repoRoot(), 'fixtures', 'golden');
const golden = JSON.parse(readFileSync(join(GOLDEN_DIR, 'philadelphia-renters-4.json'), 'utf8')) as { binder: Binder };

describe("the PDF of the engine's Philadelphia binder", () => {
  for (const paper of ['LETTER', 'A4'] as const) {
    it(`keeps every page's fit, and comes to between 75 and 110 pages (${paper === 'LETTER' ? 'Letter' : 'A4'})`, async () => {
      const { r, pdf } = await draw(golden.binder, { paper }, `philadelphia-${paper.toLowerCase()}`);
      expect(pdf.pages.length).toBeGreaterThanOrEqual(75);
      expect(pdf.pages.length).toBeLessThanOrEqual(110);
      const lines: string[] = [`${paper}: ${pdf.pages.length} pages in ${r.passes} layouts`];
      const tabs = new Map<number, { real: number; proxy: number }>();
      for (const s of r.doc.sheets) {
        if (!s.entry) continue;
        const page = s.entry.page;
        const f = sheetFill(s)!;
        expect(f.pages, `${page.id} (${page.fit}, ${fitUnits(page)} units, scale ${s.scale.toFixed(2)})`).toBeLessThanOrEqual(sheetsAllowed(page));
        const t = tabs.get(s.entry.part.tab) ?? { real: 0, proxy: 0 };
        t.real += f.pages;
        t.proxy += Math.max(1, Math.ceil(fitUnits(page) / FIT_CAPACITY));
        tabs.set(s.entry.part.tab, t);
        if (page.fit !== 'one' || page.kind === 'checklist') lines.push(`${page.id} (${page.fit}): proxy ${(fitUnits(page) / FIT_CAPACITY).toFixed(2)} pages, PDF ${f.fill.toFixed(2)} sheets at type scale ${s.scale.toFixed(2)}`);
      }
      // Every link in the text gives a page that is really there.
      const all = flat(pdf.pages.map((p) => p.text).join('\n'));
      expect(all).not.toMatch(/page 000\b|\b00000\b/);
      const sorted = [...tabs.entries()].sort((a, b) => a[0] - b[0]);
      lines.push(`pages per tab, PDF ${sorted.map(([, t]) => t.real).join('/')}; proxy ${sorted.map(([, t]) => t.proxy).join('/')}`);
      console.info(`Philadelphia, the PDF against the fit proxy (${FIT_CAPACITY} units a page):\n  ${lines.join('\n  ')}`);
    }, 180_000);
  }
});
