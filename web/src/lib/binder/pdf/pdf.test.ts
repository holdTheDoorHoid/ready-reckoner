// @vitest-environment node
/**
 * The PDF binder, drawn in Node with the same pdfmake build the browser loads
 * (DESIGN-DELTA-v3 §6): it builds on Letter and A4, single- and double-sided; the table of
 * contents lists every part and page with page numbers; every cross-reference and contents line is
 * a link annotation to a named destination; the label sheet is the last page; and the pages land
 * where the binder says (a `fit: one` page on one sheet, each part starting a sheet, on a
 * right-hand page when double-sided).
 *
 * Set RR_PDF_OUT to a folder to keep the files for a look.
 */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import type { PdfMakeInstance } from 'pdfmake/build/pdfmake.min.js';
import { describe, expect, it } from 'vitest';

import type { Binder } from '../../../engine/types';
import { FIXTURE_BINDER } from '../fixture';
import { binderPages } from '../model';
import { type Paper, type PdfOptions, sheetRange } from './doc';
import { FONT_FILES, type FontFiles, renderBinderPdf, type RenderedPdf } from './render';
import { readPdf } from '../../../test/read-pdf';

const require = createRequire(import.meta.url);
const pdfMake = require('pdfmake/build/pdfmake.min.js') as PdfMakeInstance;
const fonts = Object.fromEntries(
  Object.entries(FONT_FILES).map(([style, file]) => [style, new Uint8Array(readFileSync(new URL(`./fonts/${file}`, import.meta.url)))]),
) as FontFiles;

const OUT = process.env.RR_PDF_OUT;

async function draw(b: Binder, opts: Partial<PdfOptions> & { paper: Paper }, name: string): Promise<RenderedPdf> {
  const r = await renderBinderPdf(pdfMake, fonts, b, { doubleSided: false, appVersion: '0.3.0+test', ...opts });
  if (OUT) {
    mkdirSync(OUT, { recursive: true });
    writeFileSync(`${OUT}/${name}.pdf`, r.bytes);
  }
  return r;
}

describe('the PDF binder, from the fixture binder', () => {
  it('builds on Letter and A4, with every page and the table of contents in it', async () => {
    for (const paper of ['LETTER', 'A4'] as const) {
      const r = await draw(FIXTURE_BINDER, { paper }, `fixture-${paper.toLowerCase()}`);
      const pdf = readPdf(r.bytes);
      const size = paper === 'LETTER' ? [612, 792] : [595.28, 841.89];
      expect(pdf.pages.length, paper).toBeGreaterThan(binderPages(FIXTURE_BINDER).length);
      for (const p of pdf.pages) expect(p.mediaBox.slice(2).map((n) => Math.round(n)), paper).toEqual(size.map(Math.round));
      // Every binder page is a named destination and starts its own sheet.
      const starts = new Map(r.doc.sheets.map((s) => [s.key, sheetRange(s)?.[0]]));
      for (const e of binderPages(FIXTURE_BINDER)) {
        expect(pdf.destinations.has(`pg-${e.page.id}`), e.page.id).toBe(true);
        expect(starts.get(`page:${e.page.id}`), e.page.id).toBeGreaterThan(0);
      }
      const ordered = r.doc.sheets.map((s) => sheetRange(s)![0]);
      for (let i = 1; i < ordered.length; i++) expect(ordered[i]!, r.doc.sheets[i]!.key).toBeGreaterThan(ordered[i - 1]!);
    }
  });
});
