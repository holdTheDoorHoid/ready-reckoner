/**
 * Enough of a PDF reader to test the binder's PDF (pdfkit's output: plain objects, no object
 * streams, Flate-compressed content): the pages and their sizes, each page's text (decoded through
 * the fonts' ToUnicode maps), its link annotations, and the named destinations.
 */
import { inflateSync } from 'node:zlib';

export interface PdfLink {
  /** A named destination (`pg-home`), or the address of a web link. */
  dest?: string;
  uri?: string;
  rect: number[];
}

export interface PdfPage {
  /** 1-based. */
  number: number;
  mediaBox: number[];
  /** The page's text, a line per text line. */
  text: string;
  links: PdfLink[];
}

export interface ReadPdf {
  pages: PdfPage[];
  /** Named destination → 1-based page number. */
  destinations: Map<string, number>;
  /** The document information dictionary's text entries. */
  info: Record<string, string>;
  /** Base font names (`ABCDEF+NotoSans-Bold`). */
  fonts: string[];
}

interface Obj {
  dict: string;
  stream?: Buffer;
}

function objects(raw: Buffer): Map<number, Obj> {
  const text = raw.toString('latin1');
  const out = new Map<number, Obj>();
  const re = /(\d+) 0 obj\s*/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    const id = Number(m[1]);
    const start = m.index + m[0].length;
    const end = text.indexOf('endobj', start);
    const body = text.slice(start, end);
    const s = body.indexOf('stream');
    if (s >= 0 && /^stream\r?\n/.test(body.slice(s))) {
      const dict = body.slice(0, s);
      const len = Number(/\/Length (\d+)/.exec(dict)?.[1] ?? 0);
      const dataStart = start + s + body.slice(s).indexOf('\n') + 1;
      let data = raw.subarray(dataStart, dataStart + len);
      if (/\/Filter \/FlateDecode/.test(dict)) data = inflateSync(data);
      out.set(id, { dict, stream: data });
    } else {
      out.set(id, { dict: body });
    }
    re.lastIndex = end;
  }
  return out;
}

const ref = (dict: string, key: string): number | undefined => {
  const m = new RegExp(`/${key} (\\d+) 0 R`).exec(dict);
  return m ? Number(m[1]) : undefined;
};

/** A PDF literal string's bytes as text (escapes resolved). */
function literal(s: string): string {
  return s.replace(/\\([nrtbf()\\]|[0-7]{1,3})/g, (_m, e: string) => {
    if (/^[0-7]+$/.test(e)) return String.fromCharCode(parseInt(e, 8));
    return ({ n: '\n', r: '\r', t: '\t', b: '\b', f: '\f' } as Record<string, string>)[e] ?? e;
  });
}

/** A PDF text string (UTF-16BE with a byte-order mark, or PDFDocEncoding) as text. */
function textString(bytes: string): string {
  if (bytes.startsWith('þÿ')) {
    let s = '';
    for (let i = 2; i + 1 < bytes.length; i += 2) s += String.fromCharCode((bytes.charCodeAt(i) << 8) | bytes.charCodeAt(i + 1));
    return s;
  }
  return bytes;
}

function hexToUtf16(hex: string): string {
  let s = '';
  for (let i = 0; i + 3 < hex.length; i += 4) s += String.fromCharCode(parseInt(hex.slice(i, i + 4), 16));
  return s;
}

/** Glyph id (4 hex digits) → text, from a ToUnicode CMap. */
function toUnicode(cmap: string): Map<number, string> {
  const map = new Map<number, string>();
  for (const block of cmap.matchAll(/beginbfrange([\s\S]*?)endbfrange/g)) {
    for (const line of block[1]!.matchAll(/<([0-9a-fA-F]+)>\s*<([0-9a-fA-F]+)>\s*(\[[^\]]*\]|<[0-9a-fA-F]+>)/g)) {
      const lo = parseInt(line[1]!, 16);
      const hi = parseInt(line[2]!, 16);
      if (line[3]!.startsWith('[')) {
        [...line[3]!.matchAll(/<([0-9a-fA-F]+)>/g)].forEach((u, i) => map.set(lo + i, hexToUtf16(u[1]!)));
      } else {
        const base = parseInt(line[3]!.slice(1, -1), 16);
        for (let g = lo; g <= hi; g++) map.set(g, String.fromCharCode(base + g - lo));
      }
    }
  }
  for (const block of cmap.matchAll(/beginbfchar([\s\S]*?)endbfchar/g)) {
    for (const line of block[1]!.matchAll(/<([0-9a-fA-F]+)>\s*<([0-9a-fA-F]+)>/g)) map.set(parseInt(line[1]!, 16), hexToUtf16(line[2]!));
  }
  return map;
}

export function readPdf(bytes: Uint8Array): ReadPdf {
  const raw = Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const objs = objects(raw);
  const pageIds = [...objs.entries()].filter(([, o]) => /\/Type \/Page\b(?!s)/.test(o.dict)).map(([id]) => id);
  // Page order: the page tree's Kids, in order.
  const kids = [...objs.values()].find((o) => /\/Type \/Pages/.test(o.dict))?.dict;
  const order = kids ? [...(/\/Kids \[([^\]]*)\]/.exec(kids)?.[1] ?? '').matchAll(/(\d+) 0 R/g)].map((k) => Number(k[1])) : pageIds;
  const pageNumber = new Map(order.map((id, i) => [id, i + 1]));

  const fontMaps = new Map<number, Map<number, string>>();
  const fontMap = (id: number) => {
    if (!fontMaps.has(id)) {
      const tu = ref(objs.get(id)?.dict ?? '', 'ToUnicode');
      fontMaps.set(id, tu ? toUnicode(objs.get(tu)?.stream?.toString('latin1') ?? '') : new Map());
    }
    return fontMaps.get(id)!;
  };

  const pages: PdfPage[] = order.map((id, i) => {
    const dict = objs.get(id)!.dict;
    const mediaBox = (/\/MediaBox \[([^\]]*)\]/.exec(dict)?.[1] ?? '').trim().split(/\s+/).map(Number);
    // Fonts named in the page's resources.
    const resId = ref(dict, 'Resources');
    const res = resId ? objs.get(resId)!.dict : dict;
    const fontDict = /\/Font\s*<<([\s\S]*?)>>/.exec(res)?.[1] ?? '';
    const fonts = new Map([...fontDict.matchAll(/\/(\w+) (\d+) 0 R/g)].map((f) => [f[1]!, Number(f[2])]));
    const contents = [...(/\/Contents \[([^\]]*)\]/.exec(dict)?.[1] ?? /\/Contents (\d+ 0 R)/.exec(dict)?.[1] ?? '').matchAll(/(\d+) 0 R/g)].map((c) => objs.get(Number(c[1]))?.stream?.toString('latin1') ?? '');
    let text = '';
    let current: Map<number, string> = new Map();
    let lastY: number | null = null;
    for (const stream of contents) {
      for (const op of stream.matchAll(/\/(\w+) [\d.]+ Tf|([\d.-]+) ([\d.-]+) Tm|\[((?:<[0-9a-fA-F]*>|[^\]])*)\] TJ|<([0-9a-fA-F]*)> Tj/g)) {
        if (op[1]) current = fontMap(fonts.get(op[1]) ?? -1);
        else if (op[3] !== undefined) {
          const y = Number(op[3]);
          if (lastY !== null && Math.abs(y - lastY) > 1) text += '\n';
          lastY = y;
        } else {
          const hex = op[4] !== undefined ? [...op[4].matchAll(/<([0-9a-fA-F]*)>/g)].map((h) => h[1]!).join('') : (op[5] ?? '');
          for (let k = 0; k + 3 < hex.length; k += 4) text += current.get(parseInt(hex.slice(k, k + 4), 16)) ?? '�';
        }
      }
    }
    const annots = [...(/\/Annots \[([^\]]*)\]/.exec(dict)?.[1] ?? '').matchAll(/(\d+) 0 R/g)].map((a) => objs.get(Number(a[1]))?.dict ?? '');
    const links: PdfLink[] = annots
      .filter((a) => /\/Subtype \/Link/.test(a))
      .map((a) => {
        const rect = (/\/Rect \[([^\]]*)\]/.exec(a)?.[1] ?? '').trim().split(/\s+/).map(Number);
        // The action is inline, or (as pdfkit writes it) an object of its own.
        const actionRef = ref(a, 'A');
        const action = actionRef !== undefined ? (objs.get(actionRef)?.dict ?? '') : a;
        const dest = /\/D \(((?:\\.|[^)])*)\)/.exec(action)?.[1] ?? /\/Dest \(((?:\\.|[^)])*)\)/.exec(a)?.[1];
        const uri = /\/URI \(((?:\\.|[^)])*)\)/.exec(action)?.[1];
        return { rect, ...(dest !== undefined ? { dest: literal(dest) } : {}), ...(uri !== undefined ? { uri: literal(uri) } : {}) };
      });
    return { number: i + 1, mediaBox, text, links };
  });

  // Named destinations: the catalog's /Dests name tree (pdfkit writes one flat /Names array).
  const destinations = new Map<string, number>();
  for (const o of objs.values()) {
    const names = /\/Names \[([\s\S]*)\]/.exec(o.dict)?.[1];
    if (!names || !/\/XYZ|\/Fit/.test(names)) continue;
    for (const d of names.matchAll(/\(((?:\\.|[^)])*)\)\s*\[\s*(\d+) 0 R/g)) destinations.set(literal(d[1]!), pageNumber.get(Number(d[2])) ?? 0);
  }
  // The trailer's /Info dictionary; pdfkit writes each entry as a string object of its own.
  const infoId = ref(raw.toString('latin1').slice(raw.lastIndexOf('trailer')), 'Info');
  const infoDict = (infoId !== undefined ? objs.get(infoId)?.dict : undefined) ?? '';
  const info: Record<string, string> = {};
  for (const e of infoDict.matchAll(/\/(\w+) (?:\(((?:\\.|[^)])*)\)|(\d+) 0 R)/g)) {
    const str = e[2] ?? /^\s*\(((?:\\.|[^)])*)\)/.exec(objs.get(Number(e[3]))?.dict ?? '')?.[1];
    if (str !== undefined) info[e[1]!] = textString(literal(str));
  }
  const fonts = [...objs.values()].flatMap((o) => (/\/Type \/Font/.test(o.dict) ? [/\/BaseFont \/([\w+-]+)/.exec(o.dict)?.[1] ?? ''] : [])).filter(Boolean);
  return { pages, destinations, info, fonts };
}
