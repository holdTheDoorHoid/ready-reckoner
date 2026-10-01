/**
 * The PDF binder in the browser: the lazy chunk the Binder screen imports on the first press of
 * "Download PDF". It holds pdfmake (MIT) and this app's PDF code; the four font files are separate
 * assets fetched from this site on first use. The service worker precaches the chunk and the fonts
 * with the rest of the build, so the download works offline.
 */
import pdfMake from 'pdfmake/build/pdfmake.min.js';

import type { Binder } from '../../../engine/types';
import { binderTexts } from '../model';
import { type PdfOptions, sheetRange } from './doc';
import { missingCharacters } from './coverage';
import boldItalicUrl from './fonts/NotoSans-BoldItalic.woff?url';
import boldUrl from './fonts/NotoSans-Bold.woff?url';
import italicUrl from './fonts/NotoSans-Italic.woff?url';
import regularUrl from './fonts/NotoSans-Regular.woff?url';
import { type FontFiles, renderBinderPdf } from './render';

const URLS: Record<keyof FontFiles, string> = { normal: regularUrl, bold: boldUrl, italics: italicUrl, bolditalics: boldItalicUrl };

let fonts: Promise<FontFiles> | null = null;

/** The font files, fetched from this site once per visit (the service worker answers offline). */
function loadFonts(): Promise<FontFiles> {
  fonts ??= Promise.all(
    (Object.keys(URLS) as (keyof FontFiles)[]).map(async (style) => {
      const res = await fetch(URLS[style]);
      if (!res.ok) throw new Error(`the PDF's typeface could not be loaded (${res.status})`);
      return [style, new Uint8Array(await res.arrayBuffer())] as const;
    }),
  ).then((pairs) => Object.fromEntries(pairs) as FontFiles);
  fonts.catch(() => (fonts = null));
  return fonts;
}

export interface MadePdf {
  blob: Blob;
  /** Printed pages. */
  pages: number;
  /** Characters in the binder the PDF's typeface lacks (they print as empty boxes). */
  missing: string[];
}

/** The binder as a PDF file. */
export async function makeBinderPdf(b: Binder, opts: PdfOptions): Promise<MadePdf> {
  const { bytes, doc } = await renderBinderPdf(pdfMake, await loadFonts(), b, opts);
  const last = doc.sheets.at(-1);
  return {
    blob: new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'application/pdf' }),
    pages: (last && sheetRange(last)?.[1]) ?? 0,
    missing: missingCharacters(binderTexts(b)),
  };
}
