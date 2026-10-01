/**
 * Drawing the binder's PDF with pdfmake: registers the four Noto Sans faces, builds the document
 * (`doc.ts`), lays it out, and keeps each page's `fit` promise: a page that promises one sheet (or
 * two) and takes more is drawn again with smaller type, down to `MIN_SCALE` (8 points). pdfmake is
 * handed in, so the browser (`browser.ts`, the lazy chunk) and the tests in Node (which read the
 * font files from disk) draw with the same code.
 *
 * pdfmake is told never to fetch a URL or read a file: the fonts come from memory and the maps are
 * data URLs, so making a PDF sends nothing anywhere and works offline.
 */
import type { PdfMakeInstance } from 'pdfmake/build/pdfmake.min.js';

import type { Binder } from '../../../engine/types';
import { fitUnits } from '../model';
import { binderDocument, type BinderDoc, MIN_SCALE, pageStarts, type PdfOptions, REGISTER_KINDS, sheetFill, sheetsAllowed } from './doc';
import coverage from './fonts/coverage.json';

export type FontStyle = 'normal' | 'bold' | 'italics' | 'bolditalics';
/** The four font files' bytes. */
export type FontFiles = Record<FontStyle, Uint8Array>;

export const FONT_STYLES: readonly FontStyle[] = ['normal', 'bold', 'italics', 'bolditalics'];
/** The font files' names (the faces of `fonts/coverage.json`). */
export const FONT_FILES: Record<FontStyle, string> = coverage.files as Record<FontStyle, string>;

const registered = new WeakSet<object>();

function base64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

/** Give pdfmake the fonts (once per pdfmake instance) and shut its doors to the network and disk. */
export function preparePdfMake(pdfMake: PdfMakeInstance, fonts: FontFiles): void {
  if (registered.has(pdfMake)) return;
  const vfs: Record<string, string> = {};
  for (const style of FONT_STYLES) vfs[FONT_FILES[style]] = base64(fonts[style]);
  pdfMake.addVirtualFileSystem(vfs);
  pdfMake.setFonts({ NotoSans: { normal: FONT_FILES.normal, bold: FONT_FILES.bold, italics: FONT_FILES.italics, bolditalics: FONT_FILES.bolditalics } });
  pdfMake.setUrlAccessPolicy(() => false);
  pdfMake.localAccessPolicy = () => false;
  registered.add(pdfMake);
}

export interface RenderedPdf {
  bytes: Uint8Array;
  /** The laid-out document: each sheet's start and end marks hold the pages it landed on. */
  doc: BinderDoc;
  /** Layouts it took (1, or more when some page had to be redrawn smaller). */
  passes: number;
}

/** At most this many layouts that shrink type: the first, and up to two redraws of pages over their fit. */
const MAX_PASSES = 3;
/** And at most this many in all, counting layouts that only bring the cross-references' page numbers up to date. */
const MAX_LAYOUTS = 5;

/**
 * Word units (DESIGN-DELTA-v3 §5.6) that fit on one sheet at full-size type, measured on the
 * content's 57 checklists (see the calibration in `pdf.test.ts`). A page with more starts its
 * first layout at a smaller scale, so most binders take one or two layouts, not four.
 */
export const UNITS_PER_SHEET = 330;

/** The scale a page starts at: full size, or what its proxy load says it will need. */
export function firstScale(units: number, sheets: number): number {
  const room = UNITS_PER_SHEET * sheets;
  if (units <= room) return 1;
  // Text set smaller takes less room in both directions; lists of short lines, mostly in one.
  return Math.max(MIN_SCALE, Math.min(1, (room / units) ** (1 / 1.5)));
}

/** The binder as PDF bytes, every `fit` promise kept where type of 8 points or more allows. */
export async function renderBinderPdf(pdfMake: PdfMakeInstance, fonts: FontFiles, b: Binder, opts: PdfOptions): Promise<RenderedPdf> {
  preparePdfMake(pdfMake, fonts);
  let scales = new Map<string, number>();
  for (const part of b.parts) {
    for (const page of part.pages) {
      const allowed = sheetsAllowed(page, opts);
      // The proxy was calibrated on prose pages; a register sets its tables far denser.
      if (Number.isFinite(allowed) && !REGISTER_KINDS.has(page.kind)) {
        const k = firstScale(fitUnits(page), allowed);
        if (k < 1) scales.set(page.id, k);
      }
    }
  }
  let numbers = new Map<string, number>();
  for (let pass = 1; ; pass++) {
    const doc = binderDocument(b, opts, scales, numbers);
    const bytes = await pdfMake.createPdf(doc.definition).getBuffer();
    const starts = pageStarts(doc);
    // The cross-references printed the page numbers of the layout before: are they still right?
    const stale = [...doc.refs].some((id) => numbers.get(id) !== starts.get(id));
    const next = pass < MAX_PASSES ? refit(doc) : null;
    if ((!next && !stale) || pass >= MAX_LAYOUTS) return { bytes: new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength), doc, passes: pass };
    if (next) scales = next;
    // The next layout's numbers: where each page will start if every page drawn smaller now fits.
    numbers = next ? predictStarts(doc, next) : starts;
  }
}

/**
 * Where each binder page will start in the next layout, by page id: this layout's pages, except
 * that a page about to be drawn smaller (in `next`) is counted at the sheets it is allowed. Pages
 * that start on a right-hand page (`beforeEven`) skip to the next odd page. A guess, checked after
 * the layout: it saves a layout whenever the pages drawn smaller fit as hoped.
 */
export function predictStarts(doc: BinderDoc, next: ReadonlyMap<string, number>): Map<string, number> {
  const out = new Map<string, number>();
  let last = 0;
  for (const s of doc.sheets) {
    const fill = sheetFill(s);
    let pages = fill?.pages ?? 1;
    if (s.entry && (next.get(s.entry.page.id) ?? 1) < s.scale && pages > s.allowed) pages = s.allowed;
    let start = last + 1;
    if (last > 0 && s.node.pageBreak === 'beforeEven' && start % 2 === 0) start += 1;
    if (s.entry) out.set(s.entry.page.id, start);
    last = start + pages - 1;
  }
  return out;
}

/**
 * The scales for the next layout: every page that took more sheets than its `fit` allows, a step
 * smaller (by what its overflow suggests, at most 0.16 a step, never below `MIN_SCALE`); null when
 * nothing is over, or nothing over can shrink further.
 */
export function refit(doc: BinderDoc): Map<string, number> | null {
  const next = new Map<string, number>();
  let changed = false;
  for (const s of doc.sheets) {
    if (!s.entry) continue;
    if (s.scale < 1) next.set(s.entry.page.id, s.scale);
    const allowed = s.allowed;
    const f = sheetFill(s);
    if (!f || f.pages <= allowed || s.scale <= MIN_SCALE) continue;
    const want = s.scale * ((allowed - 0.03) / f.fill) ** (1 / 1.5);
    next.set(s.entry.page.id, Math.max(MIN_SCALE, Math.min(s.scale - 0.02, Math.max(s.scale - 0.16, want))));
    changed = true;
  }
  return changed ? next : null;
}
