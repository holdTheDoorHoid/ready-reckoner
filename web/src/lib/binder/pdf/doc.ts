/**
 * The binder as a pdfmake document (DESIGN-DELTA-v3 §6): a pure function of the engine's tree, the
 * household's maps and the paper. pdfmake itself is loaded elsewhere (`render.ts`, the lazy chunk);
 * this module only builds the description, so it can be tested without drawing anything.
 *
 * Layout, in order: the cover; "How to use this binder"; the table of contents with page numbers
 * (every entry a link); the rest of the parts, each binder page starting a fresh sheet and each part
 * starting on a right-hand page when the binder is printed on both sides; the tab label sheet last.
 * Every sheet but the cover carries the tab number and short title in its header, and the running
 * page number ("page 37 of 112"), the app version, the day it was made and the review date in its
 * footer. Pages that are cut up (the wallet cards, the labels) keep a blank back when printed on
 * both sides, so cutting them out never cuts into another page.
 *
 * Page numbers come from pdfmake's own page references, so they are exact without a second pass:
 * the table of contents, and a "Page" column added to every table and decision that points to
 * pages ("Which checklist?", "Risks at a glance", "Leave or stay?"), print them. (pdfmake fills a
 * page reference only on the first line of its text, so each number has a cell of its own.) A
 * link inside running text prints as "(Tab 3, Home)", as the Markdown binder does, and is a link
 * in the PDF; the tab is enough to find it on paper.
 *
 * Type is 9.5 points. A page whose `fit` promises one sheet (or two) and does not fit is drawn
 * again with its type and spacing scaled down (`scales`, chosen by `render.ts`), never below
 * `MIN_SCALE`, so the promise holds wherever the type can stay readable.
 *
 * Text is the engine's, exactly: user answers are never parsed, only normalised to composed
 * characters (NFC) so an accent typed as two code points prints as one letter.
 */
import type { Binder, Block, CalloutKind, Inline, IsoDate, MapSlot, MapSlotKind, Page } from '../../../engine/types';
import { formatDate } from '../../format';
import { binderPages, creditsNotShown, type PageEntry, sourceListIndex } from '../model';
import { CALLOUT_STYLES, HAIRLINE, HEADER_FILL, INK, MEMORY_FILL, MUTED, STEP_STYLES } from '../palette';
import { labelSheet } from './labels';

// ---------------------------------------------------------------------------------------------
// Options and the pieces the caller supplies
// ---------------------------------------------------------------------------------------------

export type Paper = 'LETTER' | 'A4';

/** One composed map for a slot (web-maps' `MapRecord`, the parts the PDF prints). */
export interface PdfMap {
  /** `data:image/jpeg;base64,…` */
  dataUrl: string;
  /** CSS pixels: the aspect ratio. */
  width: number;
  height: number;
  legend: { mark: string; name: string; kind: string; address?: string; phone?: string; own?: boolean; offMap?: boolean }[];
  keys: { pattern: string; text: string }[];
  statuses: { text: string }[];
  notes: string[];
  credits: string[];
  scale: string;
  fetched_on: IsoDate;
}

/** Why a slot has no map: none fetched, maps that need refreshing, or a browser that keeps none. */
export type MapsStatus = 'none' | 'ready' | 'stale' | 'unavailable';

export interface PdfOptions {
  paper: Paper;
  /** Printed on both sides: parts start on a right-hand page; cut-out pages keep a blank back. */
  doubleSided: boolean;
  /** "0.3.0+abc1234", for the footer. */
  appVersion: string;
  /** The household's maps by slot kind; a slot without one prints a one-line note instead. */
  maps?: Partial<Record<MapSlotKind, PdfMap>>;
  mapsStatus?: MapsStatus;
}

// pdfmake's document description is plain objects; these names keep the code readable.
type Node = Record<string, unknown>;
type Content = Node | string;
type TextRun = Content[];

export interface PaperSize {
  width: number;
  height: number;
}

export const PAPER_SIZES: Record<Paper, PaperSize> = {
  LETTER: { width: 612, height: 792 },
  A4: { width: 595.28, height: 841.89 },
};

/** Left, top, right, bottom, in points. Equal sides, so a page lays out the same front or back. */
export const MARGINS = [54, 58, 54, 52] as const;
/** The wallet cards' page uses narrower sides so two 3.5-inch cards fit across with a gutter. */
const CARD_MARGINS = [36, 58, 36, 52] as const;

/** Body type, in points. */
export const BASE_SIZE = 9.5;
/** The smallest a page's type may be scaled to keep its `fit` promise (8 points). */
export const MIN_SCALE = 0.84;

/** Ids of the destinations in the PDF. */
export const pageDest = (id: string): string => `pg-${id}`;
export const sourceDest = (n: number): string => `src-${n}`;
const TOC_DEST = 'rr-contents';
const LABELS_DEST = 'rr-labels';

/** The type styles, at scale 1 (sizes and spacing scale with a page; see `Ctx.st`). */
const STYLES = {
  title: { fontSize: 15, bold: true, lineHeight: 1.1, margin: [0, 0, 0, 5] },
  coverTitle: { fontSize: 26, bold: true, lineHeight: 1.1, margin: [0, 90, 0, 18] },
  partLabel: { fontSize: 8, bold: true, characterSpacing: 0.6, color: MUTED, margin: [0, 0, 0, 3] },
  h1: { fontSize: 10.5, bold: true, margin: [0, 5, 0, 1.5] },
  h2: { fontSize: 10, bold: true, margin: [0, 4, 0, 1] },
  h3: { fontSize: 9.5, bold: true, italics: true, margin: [0, 4, 0, 1] },
  para: { margin: [0, 0, 0, 4] },
  cell: { fontSize: 9 },
  th: { fontSize: 8.5, bold: true },
  fieldLabel: { fontSize: 8.5, bold: true },
  small: { fontSize: 8.5, color: MUTED },
  tiny: { fontSize: 7.5, color: MUTED },
  cite: { fontSize: 7, color: MUTED },
  tocPart: { fontSize: 10, bold: true },
  tocPage: { fontSize: 9 },
} as const;
type StyleName = keyof typeof STYLES;

// ---------------------------------------------------------------------------------------------
// The document
// ---------------------------------------------------------------------------------------------

/** One top-level stack per printed binder page (and the table of contents and the labels). */
export interface Sheet {
  key: string;
  /** The binder page, when the sheet is one. */
  entry?: PageEntry;
  /** The pdfmake node. */
  node: Node;
  /**
   * Its first line (the title) and a mark after its last block: once laid out, their `positions`
   * say on which pages the sheet begins and ends (the positions of content inside pdfmake's
   * unbreakable blocks are not reliable; these two always are).
   */
  start: Node;
  end: Node;
  /** The type scale it was drawn at. */
  scale: number;
}

export interface BinderDoc {
  /** The pdfmake document definition. */
  definition: Node;
  /** In document order. */
  sheets: Sheet[];
}

/** The pdfmake document for a binder; `scales` shrinks the type of some pages (by page id). */
export function binderDocument(b: Binder, opts: PdfOptions, scales: ReadonlyMap<string, number> = new Map()): BinderDoc {
  const paper = PAPER_SIZES[opts.paper];
  const entries = binderPages(b);
  const ctx = new Ctx(b, opts, paper, entries);
  const sheets: Sheet[] = [];
  const add = (key: string, node: Node, scale: number, entry?: PageEntry) => {
    const stack = node.stack as Content[];
    const start = stack.find((n): n is Node => typeof n === 'object' && typeof n.id === 'string') ?? (stack[0] as Node);
    const end = endMark();
    stack.push(end);
    sheets.push({ key, node, entry, start, end, scale });
  };

  // The table of contents follows "How to use this binder", or the cover when there is none.
  const tocAfter = entries.find((e) => e.page.kind === 'how_to_use') ?? entries[0];
  for (const e of entries) {
    const scale = Math.min(1, Math.max(MIN_SCALE, scales.get(e.page.id) ?? 1));
    add(`page:${e.page.id}`, ctx.sheet(e, sheets.length === 0, scale), scale, e);
    if (e === tocAfter) add('toc', ctx.contents(), 1);
  }
  add('labels', ctx.labels(), 1);

  const owners = new PageOwners(sheets);
  const definition: Node = {
    pageSize: { width: paper.width, height: paper.height },
    pageMargins: [...MARGINS],
    info: {
      title: nfc(b.title),
      author: 'Ready Reckoner',
      subject: nfc(`Emergency binder: ${b.location}`),
      creator: `Ready Reckoner ${opts.appVersion}`,
      producer: 'Ready Reckoner (pdfmake)',
    },
    language: 'en-US',
    version: '1.7',
    displayTitle: true,
    defaultStyle: { font: 'NotoSans', fontSize: BASE_SIZE, lineHeight: 1.18, color: INK },
    content: sheets.map((s) => s.node),
    header: (current: number) => ctx.header(owners.owner(current), current),
    footer: (current: number, count: number) => ctx.footer(owners.owner(current), current, count),
  };
  return { definition, sheets };
}

/** A mark after a sheet's last block: no height, no ink, but a position once laid out. */
function endMark(): Node {
  return { canvas: [{ type: 'line', x1: 0, y1: 0, x2: 0, y2: 0, lineWidth: 0, lineColor: '#ffffff' }] };
}

interface Position {
  pageNumber: number;
  verticalRatio?: number;
}

function positionsOf(n: Node): Position[] {
  return (n.positions as Position[] | undefined) ?? [];
}

/** The first and last page a laid-out sheet covers, or null before layout. */
export function sheetRange(s: Pick<Sheet, 'start' | 'end'>): [number, number] | null {
  const first = positionsOf(s.start)[0]?.pageNumber;
  const last = positionsOf(s.end).at(-1)?.pageNumber;
  return first !== undefined && last !== undefined ? [first, Math.max(first, last)] : null;
}

/**
 * How much paper a laid-out sheet used: the pages before its last one, plus how far down its last
 * page its last block ends (0 to 1). For calibrating the engine's fit proxy against real pages.
 */
export function sheetFill(s: Pick<Sheet, 'start' | 'end'>): { pages: number; fill: number } | null {
  const range = sheetRange(s);
  const end = positionsOf(s.end).at(-1);
  if (!range || !end) return null;
  return { pages: range[1] - range[0] + 1, fill: range[1] - range[0] + Math.min(1, Math.max(0, end.verticalRatio ?? 0)) };
}

/** Sheets a laid-out binder page may take: one or two by its `fit`, any number when it flows. */
export function sheetsAllowed(page: Page): number {
  return page.fit === 'one' ? 1 : page.fit === 'two' ? 2 : Infinity;
}

/** Which sheet each printed page belongs to, read from the laid-out nodes (once per layout). */
class PageOwners {
  #owners: (Sheet | undefined)[] = [];
  constructor(private readonly sheets: Sheet[]) {}

  owner(page: number): Sheet | undefined {
    // pdfmake lays out the body, then asks for headers page by page from 1: read the positions once.
    if (page === 1) {
      this.#owners = [];
      for (const s of this.sheets) {
        const range = sheetRange(s);
        if (range) for (let p = range[0]; p <= range[1]; p++) this.#owners[p] ??= s;
      }
    }
    return this.#owners[page];
  }
}

// ---------------------------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------------------------

/** Everything the block builders share. */
class Ctx {
  readonly width: number;
  readonly pages: Map<string, PageEntry>;
  /** The type scale of the page being built. */
  private k = 1;
  /** The page being built. */
  private page: Page | undefined;

  constructor(
    readonly b: Binder,
    readonly opts: PdfOptions,
    readonly paper: PaperSize,
    private readonly entries: PageEntry[],
  ) {
    this.width = paper.width - MARGINS[0] - MARGINS[2];
    this.pages = new Map(entries.map((e) => [e.page.id, e]));
  }

  /** A style's properties at the current page's scale. */
  private st(name: StyleName): Node {
    const s = STYLES[name] as Node;
    const out: Node = { ...s };
    if (typeof s.fontSize === 'number') out.fontSize = s.fontSize * this.k;
    if (Array.isArray(s.margin)) out.margin = (s.margin as number[]).map((m) => m * this.k);
    return out;
  }

  /** A length at the current page's scale. */
  private z(points: number): number {
    return points * this.k;
  }

  /** How a printed page starts: on the next page, or on the next right-hand one. */
  private breakBefore(recto: boolean): string {
    return recto && this.opts.doubleSided ? 'beforeEven' : 'before';
  }

  /** One binder page as a top-level stack starting on a fresh sheet. */
  sheet(e: PageEntry, first: boolean, scale: number): Node {
    this.k = scale;
    this.page = e.page;
    const p = e.page;
    const firstOfPart = e.inPart === 0;
    const cut = p.kind === 'wallet_cards';
    const head: Content[] = [];
    if (p.kind === 'cover') {
      head.push({ text: nfc(this.b.title), ...this.st('coverTitle'), id: pageDest(p.id), outline: true, outlineText: 'Cover' });
    } else {
      if (firstOfPart) head.push({ text: `TAB ${e.part.tab} · ${nfc(e.part.title).toUpperCase()}`, ...this.st('partLabel') });
      head.push({ text: nfc(p.title), ...this.st('title'), id: pageDest(p.id), outline: true, outlineText: `${e.part.tab}. ${nfc(p.title)}` });
    }
    const width = cut ? this.paper.width - CARD_MARGINS[0] - CARD_MARGINS[2] : this.width;
    const body = p.kind === 'sources' ? this.sourcesPage(p, width) : this.blocks(p.blocks, width, p);
    const node: Node = { stack: [...head, ...body], fontSize: BASE_SIZE * this.k };
    if (cut) node.margin = [CARD_MARGINS[0] - MARGINS[0], 0, CARD_MARGINS[2] - MARGINS[2], 0];
    // A part starts on a right-hand page when printed on both sides, and so does the page after a
    // cut-out page, so the cut-out keeps a blank back.
    const afterCut = this.entries[e.index - 1]?.page.kind === 'wallet_cards';
    if (!first) node.pageBreak = this.breakBefore(firstOfPart || cut || afterCut);
    this.k = 1;
    this.page = undefined;
    return node;
  }

  // ---- table of contents --------------------------------------------------------------------

  contents(): Node {
    const rows: Content[][] = [];
    const partRow = (left: Content, title: string, dest: string | undefined): Content[] => [
      left,
      { text: title, ...this.st('tocPart'), linkToDestination: dest, margin: [0, 5, 0, 1] },
      dest ? { text: '', pageReference: dest, ...this.st('tocPart'), alignment: 'right', margin: [0, 5, 0, 1] } : '',
    ];
    for (const part of this.b.parts) {
      const first = part.pages[0];
      const dest = first ? pageDest(first.id) : undefined;
      rows.push(partRow({ text: `Tab ${part.tab}`, ...this.st('tocPart'), linkToDestination: dest, margin: [0, 5, 0, 1] }, nfc(part.title), dest));
      for (const page of part.pages) {
        rows.push([
          '',
          { text: page.kind === 'cover' ? 'Cover' : nfc(page.title), ...this.st('tocPage'), linkToDestination: pageDest(page.id), margin: [10, 0, 0, 0] },
          { text: '', pageReference: pageDest(page.id), ...this.st('tocPage'), alignment: 'right' },
        ]);
      }
    }
    rows.push(partRow({ text: '', margin: [0, 5, 0, 1] }, 'Tab labels to cut out', LABELS_DEST));
    return {
      pageBreak: 'before',
      stack: [
        { text: 'Contents', ...this.st('title'), id: TOC_DEST, outline: true, outlineText: 'Contents' },
        { text: 'Each tab of the binder, and the pages behind it. In the PDF, every line is a link.', ...this.st('small'), margin: [0, 0, 0, 4] },
        {
          table: { widths: [38, '*', 40], body: rows, dontBreakRows: true },
          layout: { hLineWidth: () => 0, vLineWidth: () => 0, paddingLeft: () => 0, paddingRight: () => 2, paddingTop: () => 1, paddingBottom: () => 1 },
        },
      ],
    };
  }

  // ---- the tab label sheet ------------------------------------------------------------------

  labels(): Node {
    return {
      pageBreak: this.breakBefore(true),
      stack: [{ text: 'Tab labels', ...this.st('title'), id: LABELS_DEST, outline: true, outlineText: 'Tab labels' }, ...labelSheet(this.b.parts, this.width)],
    };
  }

  // ---- header and footer ---------------------------------------------------------------------

  /** Odd pages are right-hand pages: their outer edge is the right one. */
  private outer(page: number): 'left' | 'right' {
    return this.opts.doubleSided && page % 2 === 0 ? 'left' : 'right';
  }

  header(owner: Sheet | undefined, page: number): Node | null {
    if (!owner || owner.entry?.page.kind === 'cover') return null;
    const part = owner.entry?.part ?? (owner.key === 'toc' ? this.b.parts[0] : undefined);
    const title = owner.entry ? nfc(owner.entry.page.title) : owner.key === 'toc' ? 'Contents' : 'Tab labels';
    const tab: Node = part
      ? { text: [{ text: ` ${part.tab} `, bold: true, color: '#ffffff', background: INK }, { text: `  ${nfc(part.short_title)}`, bold: true }], fontSize: 9.5 }
      : { text: '' };
    const name: Node = { text: title, fontSize: 8.5, color: MUTED, noWrap: true };
    const outer = this.outer(page);
    return {
      margin: [MARGINS[0], 26, MARGINS[2], 0],
      stack: [
        {
          columns: outer === 'right' ? [{ ...name, width: '*' }, { ...tab, width: 'auto' }] : [{ ...tab, width: 'auto' }, { ...name, width: '*', alignment: 'right' }],
          columnGap: 12,
        },
        { canvas: [{ type: 'line', x1: 0, y1: 4, x2: this.width, y2: 4, lineWidth: 0.5, lineColor: HAIRLINE }] },
      ],
    };
  }

  footer(owner: Sheet | undefined, page: number, count: number): Node {
    const made = `Ready Reckoner ${this.opts.appVersion} · made ${formatDate(this.b.generated_on)} · review by ${formatDate(this.b.review_by)}`;
    const blank = owner ? '' : 'This page is blank on purpose. · ';
    const num: Node = { text: `${blank}page ${page} of ${count}`, bold: true, width: 'auto' };
    const info: Node = { text: made, width: '*', color: MUTED };
    const outer = this.outer(page);
    return {
      margin: [MARGINS[0], 18, MARGINS[2], 0],
      fontSize: 7.5,
      columns: outer === 'right' ? [info, num] : [num, { ...info, alignment: 'right' }],
      columnGap: 12,
    };
  }

  // ---- blocks ---------------------------------------------------------------------------------

  /** A page's blocks, each heading kept with what follows it, page breaks honoured. */
  blocks(blocks: readonly Block[], width: number, page?: Page): Content[] {
    const out: Content[] = [];
    let breakNext = false;
    const push = (n: Content) => {
      if (breakNext && typeof n === 'object') {
        n.pageBreak = 'before';
        breakNext = false;
      }
      out.push(n);
    };
    for (let i = 0; i < blocks.length; i++) {
      const bl = blocks[i]!;
      if ('page_break' in bl) {
        breakNext = out.length > 0 && i < blocks.length - 1;
        continue;
      }
      const next = blocks[i + 1];
      // A checklist's two closing lists ("Do not", "When it is over") sit side by side: both are
      // short by the content standard, and a page of airline steps should fit on one sheet.
      const pair = page?.kind === 'checklist' ? closingPair(blocks, i) : null;
      if (pair) {
        const gap = 14;
        const half = (width - gap) / 2;
        // Not unbreakable: on a full page the two lists may run on together, rather than leave a gap.
        push({
          columnGap: gap,
          columns: pair.map(([head, list]) => ({ width: half, stack: [this.heading(head.level, head.text), ...this.list(list, false, 1)] })),
        });
        i += 3;
        continue;
      }
      if ('heading' in bl && next && 'table' in next) {
        // The heading becomes the table's first header row: it stays with the first rows and
        // repeats over a page break.
        push(this.table(next.table.header, next.table.rows, width, bl.heading));
        i += 1;
        continue;
      }
      if ('heading' in bl && next && 'log' in next) {
        push(this.log(next.log.columns, next.log.rows, width, bl.heading));
        i += 1;
        continue;
      }
      if ('heading' in bl && next && !('heading' in next) && !('page_break' in next)) {
        const [first, rest] = this.split(next, width, page);
        push({ stack: [this.heading(bl.heading.level, bl.heading.text), ...first], unbreakable: true });
        rest.forEach(push);
        i += 1;
        continue;
      }
      this.block(bl, width, page).forEach(push);
    }
    return out;
  }

  /**
   * A block cut so its first part can stay with the heading above it: the first two items of a
   * list, the first two steps or rows; a whole short block otherwise.
   */
  private split(bl: Block, width: number, page?: Page): [Content[], Content[]] {
    const KEEP = 2;
    if ('bullets' in bl && bl.bullets.length > KEEP + 1) return [this.list(bl.bullets.slice(0, KEEP), false, 1, true), this.list(bl.bullets.slice(KEEP), false, 1)];
    if ('numbered' in bl && bl.numbered.length > KEEP + 1) return [this.list(bl.numbered.slice(0, KEEP), true, 1, true), this.list(bl.numbered.slice(KEEP), true, KEEP + 1)];
    if ('steps' in bl && bl.steps.length > KEEP + 1) return [[this.steps(bl.steps.slice(0, KEEP), width, 1, true)], [this.steps(bl.steps.slice(KEEP), width, KEEP + 1)]];
    if ('fields' in bl && bl.fields.length > KEEP + 1 && this.page?.kind !== 'contacts' && this.page?.kind !== 'checklist') return [[this.fields(bl.fields.slice(0, KEEP), width, true)], [this.fields(bl.fields.slice(KEEP), width)]];
    return [this.block(bl, width, page), []];
  }

  private heading(level: number, text: string): Node {
    const l = level <= 1 ? 1 : level >= 3 ? 3 : 2;
    return { text: nfc(text), ...this.st(`h${l}` as StyleName), headlineLevel: l };
  }

  block(bl: Block, width: number, page?: Page): Content[] {
    if ('heading' in bl) return [this.heading(bl.heading.level, bl.heading.text)];
    if ('para' in bl) return [{ text: this.run(bl.para), ...this.st('para') }];
    if ('bullets' in bl) return this.list(bl.bullets, false, 1);
    if ('numbered' in bl) return this.list(bl.numbered, true, 1);
    if ('steps' in bl) return [this.steps(bl.steps, width, 1)];
    if ('fields' in bl) return [this.fields(bl.fields, width)];
    if ('table' in bl) return [this.table(bl.table.header, bl.table.rows, width)];
    if ('callout' in bl) return [this.callout(bl.callout.kind, bl.callout.title, bl.callout.blocks, width, page)];
    if ('decision' in bl) return this.decision(bl.decision.question, bl.decision.branches, width);
    if ('map_slot' in bl) return this.map(bl.map_slot, width);
    if ('cards' in bl) return this.cards(bl.cards, width);
    if ('log' in bl) return [this.log(bl.log.columns, bl.log.rows, width)];
    return [];
  }

  private list(items: readonly Inline[][], numbered: boolean, start: number, continues = false): Content[] {
    const body = items.map((it) => ({ text: this.run(it), margin: [0, 0, 0, this.z(1)] }));
    const margin = [0, 0, 0, continues ? 0 : this.z(4)];
    return [numbered ? { ol: body, start, margin } : { ul: body, margin }];
  }

  /** Airline-style steps: numbered rows, the memory items bold on a grey band. */
  private steps(steps: readonly { text: Inline[]; memory: boolean }[], width: number, from: number, continues = false): Node {
    const num = this.z(14);
    const body = steps.map((s, i) => {
      const style = s.memory ? STEP_STYLES.memory : STEP_STYLES.plain;
      return [
        { text: `${from + i}`, bold: true, alignment: 'right', color: style.ink },
        { text: this.run(s.text), bold: style.bold, color: style.ink },
      ];
    });
    const pad = this.z(1.5);
    return {
      table: { widths: [num, width - num - 13], body, dontBreakRows: true },
      layout: {
        fillColor: (row: number) => (steps[row]?.memory ? MEMORY_FILL : null),
        hLineWidth: (i: number, node: { table: { body: unknown[] } }) => (i > 0 && i < node.table.body.length ? 1.5 : 0),
        hLineColor: () => '#ffffff',
        vLineWidth: () => 0,
        paddingLeft: (i: number) => (i === 0 ? 1 : 6),
        paddingRight: () => 3,
        paddingTop: () => pad,
        paddingBottom: () => pad,
      },
      margin: [0, 0, 0, continues ? 1.5 : this.z(5)],
    };
  }

  /** Labelled answers: the household's words as written, or ruled lines to write on. */
  private fields(rows: readonly { label: string; value?: string; lines: number }[], width: number, continues = false): Node {
    // "Contacts at a glance" is a phone list, and a checklist's "Where and who" short lines to fill
    // in: their answers sit in two columns, so the page fits one sheet.
    const twoUp = this.page?.kind === 'contacts' || this.page?.kind === 'checklist';
    if (twoUp && rows.length >= 3 && !continues) {
      const gap = 14;
      const half = Math.ceil(rows.length / 2);
      const w = (width - gap) / 2;
      return {
        columns: [
          { ...this.fieldTable(rows.slice(0, half), w, true), width: w },
          { ...this.fieldTable(rows.slice(half), w, true), width: w },
        ],
        columnGap: gap,
        margin: [0, 0, 0, this.z(5)],
      };
    }
    return this.fieldTable(rows, width, continues);
  }

  private fieldTable(rows: readonly { label: string; value?: string; lines: number }[], width: number, continues: boolean): Node {
    const labelW = Math.round(width * 0.34);
    const valueW = width - labelW - 16;
    const body = rows.map((r) => [
      { text: nfc(r.label), ...this.st('fieldLabel') },
      r.value !== undefined && r.value !== '' ? { text: nfc(r.value) } : this.ruled(Math.max(1, r.lines), valueW),
    ]);
    const pad = this.z(1.6);
    return {
      table: { widths: [labelW, valueW], body, dontBreakRows: true },
      layout: {
        hLineWidth: (i: number) => (i === 0 ? 0 : 0.5),
        hLineColor: () => HAIRLINE,
        vLineWidth: () => 0,
        paddingLeft: () => 4,
        paddingRight: () => 4,
        paddingTop: () => pad,
        paddingBottom: () => pad,
      },
      margin: [0, 0, 0, continues ? 0 : this.z(5)],
    };
  }

  /** Ruled lines to write on, as wide as the cell. */
  private ruled(lines: number, width: number): Node {
    const gap = this.z(17);
    const first = this.z(12);
    return {
      canvas: Array.from({ length: lines }, (_, k) => ({ type: 'line', x1: 0, y1: first + k * gap, x2: width, y2: first + k * gap, lineWidth: 0.6, lineColor: INK })),
      margin: [0, 0, 0, 2],
    };
  }

  /** A table's lines, padding and header shading; the first `skip` rows are a heading over it. */
  private grid(skip = 0): Node {
    const pad = this.z(2);
    return {
      hLineWidth: (i: number) => (i < skip ? 0 : 0.5),
      vLineWidth: () => 0.5,
      hLineColor: () => INK,
      vLineColor: () => HAIRLINE,
      fillColor: (row: number, node: { table: { headerRows?: number } }) => (row >= skip && row < (node.table.headerRows ?? 0) ? HEADER_FILL : null),
      paddingLeft: () => 4,
      paddingRight: () => 4,
      paddingTop: () => pad,
      paddingBottom: () => pad,
    };
  }

  /** A heading as a table's first row, across every column, with no lines around it. */
  private headingRow(heading: { level: number; text: string }, cols: number): Content[] {
    const h = this.heading(heading.level, heading.text);
    return [{ ...h, colSpan: cols, border: [false, false, false, false], margin: [-4, (h.margin as number[])[1]!, 0, 0] }, ...Array.from({ length: cols - 1 }, () => ({}))];
  }

  /**
   * A table: the header row repeats across a page break and a row is never split. When its rows
   * point to pages ("Turn to Tab 6, House fire"), a last "Page" column gives each its number.
   */
  private table(header: readonly string[], rows: readonly Inline[][][], width: number, heading?: { level: number; text: string }): Node {
    const cols = Math.max(header.length, ...rows.map((r) => r.length), 1);
    const targets = rows.map((r) => lastLink(r.flat(), this.pages));
    const paged = targets.some((t) => t !== undefined);
    const head: Content[] = Array.from({ length: cols }, (_, i) => ({ text: nfc(header[i] ?? ''), ...this.st('th') }));
    if (paged) head.push({ text: 'Page', ...this.st('th'), alignment: 'right' });
    const body = rows.map((r, k) => {
      const cells: Content[] = Array.from({ length: cols }, (_, i) => {
        const cell = r[i] ?? [];
        // An empty cell is room to write in.
        return cell.length === 0 ? { text: ' ', ...this.st('cell'), margin: [0, this.z(4), 0, this.z(4)] } : { text: this.run(cell), ...this.st('cell') };
      });
      if (paged) cells.push(this.pageCell(targets[k]));
      return cells;
    });
    const widths = columnWidths(header, rows, cols, width - (paged ? PAGE_COL + 8.5 : 0), 9 * this.k);
    const heads: Content[][] = heading ? [this.headingRow(heading, head.length), head] : [head];
    return {
      table: { headerRows: heads.length, keepWithHeaderRows: 1, dontBreakRows: true, widths: paged ? [...widths, PAGE_COL] : widths, body: [...heads, ...body] },
      layout: this.grid(heading ? 1 : 0),
      margin: [0, heading ? 0 : this.z(2), 0, this.z(6)],
    };
  }

  /** A cell holding one page number, filled in by pdfmake once the pages are laid out. */
  private pageCell(dest: string | undefined): Content {
    return dest ? { text: '', pageReference: dest, ...this.st('cell'), alignment: 'right' } : { text: '', ...this.st('cell') };
  }

  /** A boxed note whose kind is a word in its label (STOP, WARNING, DECIDE, NOTE). */
  private callout(kind: CalloutKind, title: string | undefined, blocks: readonly Block[], width: number, page?: Page): Node {
    const s = CALLOUT_STYLES[kind] ?? CALLOUT_STYLES.note;
    const pad = 6;
    const inner = width - 2 * pad - 2 * s.border;
    const label: Node = {
      text: [{ text: ` ${s.word} `, bold: true, color: s.labelInk, background: s.labelFill }, ...(title ? [{ text: `  ${nfc(title)}`, bold: true }] : [])],
      fontSize: this.z(9.5),
      margin: [0, 0, 0, this.z(2)],
    };
    const dash = s.dash ? { dash: { length: 4, space: 3 } } : null;
    return {
      table: { widths: [inner], body: [[{ stack: [label, ...this.blocks(blocks, inner, page)] }]], dontBreakRows: false },
      layout: {
        hLineWidth: () => s.border,
        vLineWidth: () => s.border,
        hLineColor: () => INK,
        vLineColor: () => INK,
        hLineStyle: () => dash,
        vLineStyle: () => dash,
        paddingLeft: () => pad,
        paddingRight: () => pad,
        paddingTop: () => pad - 1,
        paddingBottom: () => pad - 3,
      },
      margin: [0, this.z(2), 0, this.z(6)],
      unbreakable: blocks.length <= 3,
    };
  }

  /** "Leave or stay?" as a two-column "If … / then …" table, with the page to turn to. */
  private decision(question: string, branches: readonly { when: Inline[]; then: Inline[]; go_to?: string }[], width: number): Content[] {
    const paged = branches.some((br) => br.go_to !== undefined && this.pages.has(br.go_to));
    const room = width - (paged ? PAGE_COL + 25 : 16.5);
    const whenW = Math.round(room * 0.4);
    const body = branches.map((br) => {
      const to = br.go_to !== undefined ? this.pages.get(br.go_to) : undefined;
      const then: TextRun = [...this.run(br.then)];
      if (to) then.push(then.length ? ' ' : '', { text: `Turn to Tab ${to.part.tab}, ${nfc(to.page.title)}.`, bold: true, linkToDestination: pageDest(to.page.id) });
      const row: Content[] = [
        { text: this.run(br.when), ...this.st('cell') },
        { text: then.filter((x) => x !== ''), ...this.st('cell') },
      ];
      if (paged) row.push(this.pageCell(to ? pageDest(to.page.id) : undefined));
      return row;
    });
    const head: Content[] = [{ text: 'If', ...this.st('th') }, { text: 'Then', ...this.st('th') }];
    if (paged) head.push({ text: 'Page', ...this.st('th'), alignment: 'right' });
    return [
      {
        unbreakable: true,
        stack: [
          this.heading(1, question),
          {
            table: { headerRows: 1, dontBreakRows: true, widths: paged ? [whenW, room - whenW, PAGE_COL] : [whenW, room - whenW], body: [head, ...body] },
            layout: this.grid(),
            margin: [0, this.z(1), 0, this.z(5)],
          },
        ],
      },
    ];
  }

  // ---- maps -----------------------------------------------------------------------------------

  private map(slot: MapSlot, width: number): Content[] {
    const m = this.opts.maps?.[slot.kind];
    if (!m) {
      // No empty frame: one line says the map is missing and how to add it (planner, 2026-10-01).
      const status = this.opts.mapsStatus ?? 'none';
      const why =
        status === 'stale'
          ? 'the maps need refreshing. Refresh them on the Binder screen of the app, then print this page again.'
          : status === 'unavailable'
            ? 'this browser keeps no maps. Add them in the app on another browser, then print this page again.'
            : 'no map added. Add maps on the Binder screen of the app, then print this page again.';
      return [{ text: [{ text: `Map of ${PLACEHOLDER[slot.kind]}: `, bold: true }, why], ...this.st('small'), margin: [0, 2, 0, 6] }];
    }
    const h = Math.round((width * m.height) / Math.max(1, m.width));
    const small = 8;
    const out: Content[] = [{ unbreakable: true, stack: [{ image: m.dataUrl, width, height: h }, { text: nfc(slot.caption), bold: true, margin: [0, 3, 0, 3] }] }];
    if (m.legend.length) {
      out.push({
        table: {
          headerRows: 1,
          dontBreakRows: true,
          widths: columnWidths(['Mark', 'Name', 'What it is', 'Address or phone'], m.legend.map((r) => [[{ t: r.mark }], [{ t: r.name }], [{ t: r.kind }], [{ t: [r.address, r.phone].filter(Boolean).join(' · ') }]]), 4, width, small),
          body: [
            ['Mark', 'Name', 'What it is', 'Address or phone'].map((t) => ({ text: t, bold: true, fontSize: small })),
            ...m.legend.map((r) => [
              { text: r.mark, bold: true, fontSize: small, alignment: 'center' },
              { text: nfc(r.name) + (r.offMap ? ' (beyond the edge of this map)' : ''), fontSize: small },
              { text: nfc(r.kind), fontSize: small },
              { text: nfc([r.address, r.phone].filter(Boolean).join(' · ') || '—'), fontSize: small },
            ]),
          ],
        },
        layout: this.grid(),
        margin: [0, 2, 0, 4],
      });
    }
    for (const key of m.keys) {
      out.push({ columns: [{ canvas: swatch(key.pattern), width: 30 }, { text: nfc(key.text), fontSize: small, width: '*' }], columnGap: 6, margin: [0, 0, 0, 2] });
    }
    for (const t of [...m.statuses.map((s) => s.text), ...m.notes]) out.push({ text: nfc(t), fontSize: small, margin: [0, 0, 0, 1] });
    out.push({ text: `${m.credits.map(nfc).join(' · ')}. ${nfc(m.scale)}`, ...this.st('tiny'), margin: [0, 2, 0, 8] });
    return out;
  }

  // ---- wallet cards ---------------------------------------------------------------------------

  /** 3.5 × 2 inch cards, two across, each boxed with corner cut marks and never split. */
  private cards(cards: readonly { title: string; lines: Inline[][] }[], width: number): Content[] {
    const W = CARD.width;
    const Hc = CARD.height;
    const gap = Math.max(18, width - 2 * W);
    const rows: Content[] = [];
    for (let i = 0; i < cards.length; i += 2) {
      const pair = cards.slice(i, i + 2);
      rows.push({
        unbreakable: true,
        margin: [0, 12, 0, 14],
        stack: [{ canvas: pair.flatMap((_, j) => cutMarks(j * (W + gap), 0, W, Hc)), relativePosition: { x: 0, y: 0 } }, { columns: pair.map((c) => this.card(c, W, Hc)), columnGap: gap }],
      });
    }
    if (this.opts.doubleSided) {
      // Eight cards fill a sheet; each further sheet starts a right-hand page, so every sheet of
      // cards has a blank back.
      for (let r = 4; r < rows.length; r += 4) (rows[r] as Node).pageBreak = 'beforeEven';
    }
    return [{ text: 'Cut along the marks at the corners. Each card is 3½ by 2 inches, the size of a wallet card.', ...this.st('small'), margin: [0, 0, 0, 2] }, ...rows];
  }

  private card(c: { title: string; lines: Inline[][] }, W: number, Hc: number): Node {
    const size = cardFontSize(c, W - 14);
    return {
      width: W,
      table: {
        widths: [W - 13],
        heights: [Hc - 1],
        body: [[{ stack: [{ text: nfc(c.title), bold: true, fontSize: size + 1.5, margin: [0, 0, 0, 2] }, ...c.lines.map((l) => ({ text: this.run(l), fontSize: size, lineHeight: 1.12 }))] }]],
      },
      layout: { hLineWidth: () => 0.5, vLineWidth: () => 0.5, hLineColor: () => HAIRLINE, vLineColor: () => HAIRLINE, paddingLeft: () => 6, paddingRight: () => 6, paddingTop: () => 5, paddingBottom: () => 3 },
    };
  }

  // ---- logs -----------------------------------------------------------------------------------

  /** An empty ruled table to fill in by hand; its header repeats if it runs on. */
  private log(columns: readonly string[], rows: number, width: number, heading?: { level: number; text: string }): Node {
    const cols = Math.max(1, columns.length);
    const head: Content[] = Array.from({ length: cols }, (_, i) => ({ text: nfc(columns[i] ?? ''), ...this.st('th') }));
    const each = (width - 8 * cols - (cols + 1) * 0.5) / cols;
    const blankRow = () => Array.from({ length: cols }, () => ({ text: ' ', margin: [0, 4, 0, 4] }));
    const heads: Content[][] = heading ? [this.headingRow(heading, cols), head] : [head];
    return {
      table: { headerRows: heads.length, dontBreakRows: true, widths: Array.from({ length: cols }, () => each), body: [...heads, ...Array.from({ length: Math.max(1, rows) }, blankRow)] },
      layout: this.grid(heading ? 1 : 0),
      margin: [0, heading ? 0 : 2, 0, 8],
    };
  }

  // ---- sources --------------------------------------------------------------------------------

  /**
   * The Sources page in small type: the engine's numbered list of sources in two columns, each
   * item an anchor the citation numbers link to (or, when the page has no such list, the list
   * from `Binder.sources`), then whatever else the page holds (the data credits, how the binder
   * was made), and any credit the page does not print itself.
   */
  private sourcesPage(p: Page, width: number): Content[] {
    const k = this.k;
    this.k = Math.min(k, 0.8);
    const at = sourceListIndex(p, this.b);
    const half = (width - 14) / 2;
    const flow: Content[] = [];
    const items = (texts: TextRun[]): Content[] =>
      texts.map((text, i) => ({ id: sourceDest(i + 1), text: [{ text: `${i + 1}. `, bold: true }, ...text], fontSize: 7, margin: [0, 0, 0, 2] }));
    if (at >= 0) {
      const list = p.blocks[at]!;
      flow.push(...this.blocks(p.blocks.slice(0, at), half, p));
      if ('numbered' in list) flow.push(...items(list.numbered.map((it) => this.run(it))));
      flow.push(...this.blocks(p.blocks.slice(at + 1), half, p));
    } else {
      flow.push(...this.blocks(p.blocks, half, p));
      if (this.b.sources.length) {
        flow.push({ text: 'Numbered sources', ...this.st('h1'), headlineLevel: 1 });
        flow.push(
          ...items(
            this.b.sources.map((s) => [
              nfc(`${s.title}. ${s.publisher}${s.year ? `, ${s.year}` : ''}${s.expert ? ', an expert estimate' : ''}.`),
              ...(s.url ? [{ text: ` ${s.url}`, color: MUTED, link: s.url }] : []),
            ]),
          ),
        );
      }
    }
    const credits = creditsNotShown(p, this.b);
    if (credits.length) {
      flow.push({ text: 'Data credits', ...this.st('h1'), headlineLevel: 1 });
      for (const c of credits) flow.push({ text: nfc(c), fontSize: 7, color: MUTED, margin: [0, 0, 0, 3] });
    }
    this.k = k;
    // pdfmake's snaking columns fill the left column, then the right, then the next page.
    return [{ columns: [{ stack: flow, width: half }, { text: '', width: half }], columnGap: 14, snakingColumns: true, fontSize: 7.5 }];
  }

  // ---- running text ---------------------------------------------------------------------------

  /**
   * Inlines as a pdfmake text run. A link reads "(Tab 3, Home)" after other words, or just
   * "Tab 6, House fire" when it is all there is (a "Turn to" cell); in the PDF it jumps to the page.
   */
  run(inlines: readonly Inline[]): TextRun {
    const out: TextRun = [];
    const endsInSpace = () => {
      const last = out[out.length - 1];
      const text = typeof last === 'string' ? last : typeof last?.text === 'string' ? last.text : '';
      return text === '' || /\s$/.test(text);
    };
    const alone = (k: number) => inlines.every((x, j) => j === k || 'cite' in x);
    inlines.forEach((i, k) => {
      if ('t' in i) out.push(nfc(i.t));
      else if ('b' in i) out.push({ text: nfc(i.b), bold: true });
      // A blank is a run of underscores: pdfmake drops the underline of spaces at the end of a line.
      else if ('blank' in i) out.push({ text: '_'.repeat(Math.max(3, Math.min(40, i.blank))), color: INK });
      else if ('cite' in i) out.push(...this.cite(i.cite, endsInSpace()));
      else if ('link' in i) {
        const words = nfc(i.link.text);
        const target = this.pages.has(i.link.to) ? pageDest(i.link.to) : undefined;
        const link: Content = target ? { text: words, linkToDestination: target, decoration: 'underline', decorationStyle: 'dotted' } : words;
        if (alone(k)) out.push(link);
        else out.push(endsInSpace() ? '(' : ' (', link, ')');
      }
    });
    return out.filter((x) => x !== '');
  }

  /** Citation numbers in brackets, each a link to its source on the Sources page. */
  private cite(ns: readonly number[], afterSpace: boolean): TextRun {
    const valid = ns.filter((n) => n >= 1 && n <= this.b.sources.length);
    if (valid.length === 0) return [];
    const st = this.st('cite');
    const parts: TextRun = [{ text: afterSpace ? '[' : `${THIN_SPACE}[`, ...st }];
    valid.forEach((n, i) => {
      if (i) parts.push({ text: ', ', ...st });
      parts.push({ text: `${n}`, ...st, linkToDestination: sourceDest(n) });
    });
    parts.push({ text: ']', ...st });
    return parts;
  }
}

// ---------------------------------------------------------------------------------------------
// Small pieces
// ---------------------------------------------------------------------------------------------

/** A wallet card: 3½ by 2 inches. */
export const CARD = { width: 252, height: 144 } as const;

const THIN_SPACE = '\u2009';

/** The width of a "Page" column: room for pdfmake's five-digit placeholder at table size. */
const PAGE_COL = 34;

const PLACEHOLDER: Record<MapSlotKind, string> = {
  neighbourhood: 'your neighborhood',
  area: 'your city or county',
  region: 'your region and the ways out',
};

/** Composed characters, so an accent typed separately prints on its letter. */
export function nfc(s: string): string {
  return s.normalize('NFC');
}

/** The page the last link in a row points to, when it is in this binder. */
function lastLink(inlines: readonly Inline[], pages: ReadonlyMap<string, PageEntry>): string | undefined {
  for (let i = inlines.length - 1; i >= 0; i--) {
    const x = inlines[i]!;
    if ('link' in x) return pages.has(x.link.to) ? pageDest(x.link.to) : undefined;
  }
  return undefined;
}

/**
 * The two lists that close a checklist page, when blocks `i` to `i + 3` are a heading and its
 * bullets twice over, each list short (at most six items), and nothing follows them.
 */
function closingPair(blocks: readonly Block[], i: number): [[{ level: number; text: string }, Inline[][]], [{ level: number; text: string }, Inline[][]]] | null {
  const [a, b, c, d] = blocks.slice(i, i + 4);
  if (blocks.length !== i + 4 || !a || !b || !c || !d) return null;
  if (!('heading' in a) || !('bullets' in b) || !('heading' in c) || !('bullets' in d)) return null;
  if (b.bullets.length > 6 || d.bullets.length > 6) return null;
  return [
    [a.heading, b.bullets],
    [c.heading, d.bullets],
  ];
}

function cellChars(cell: readonly Inline[]): { all: number; word: number } {
  const text = cell.map((i) => ('t' in i ? i.t : 'b' in i ? i.b : 'link' in i ? i.link.text : 'blank' in i ? '_'.repeat(i.blank) : 'cite' in i ? ' [00]' : '')).join('');
  return { all: [...text].length, word: Math.max(0, ...text.split(/\s+/).map((w) => [...w].length)) };
}

/**
 * Column widths in points, filling the table's width: each column as wide as its longest line
 * when they all fit; otherwise short columns (a phone, a date, "have") keep theirs and the long
 * ones share what is left in proportion to their text, never narrower than their longest word.
 * (Noto Sans sets a little over half an em a character.)
 */
export function columnWidths(header: readonly string[], rows: readonly Inline[][][], cols: number, width: number, size = 9): number[] {
  const em = size * 0.52;
  const avail = width - 8 * cols - (cols + 1) * 0.5;
  const natural: number[] = [];
  const least: number[] = [];
  for (let c = 0; c < cols; c++) {
    const h = header[c] ?? '';
    const cells = [{ all: [...h].length, word: Math.max(0, ...h.split(/\s+/).map((w) => [...w].length)) }, ...rows.map((r) => cellChars(r[c] ?? []))];
    natural.push(Math.max(18, em * Math.max(...cells.map((x) => x.all))));
    least.push(Math.max(18, em * (Math.max(...cells.map((x) => x.word)) + 1)));
  }
  const total = natural.reduce((a, b) => a + b, 0);
  if (total <= avail) return natural.map((n) => (n * avail) / total);
  const short = natural.map((n) => n <= avail * 0.2);
  const fixed = natural.reduce((a, n, i) => a + (short[i] ? n : 0), 0);
  const longTotal = natural.reduce((a, n, i) => a + (short[i] ? 0 : n), 0) || 1;
  const rest = Math.max(0, avail - fixed);
  const widths = natural.map((n, i) => (short[i] ? n : Math.max(least[i]!, (n * rest) / longTotal)));
  const sum = widths.reduce((a, b) => a + b, 0);
  return widths.map((w) => (w * avail) / sum);
}

/** Corner cut marks around a card: short lines just outside each corner. */
function cutMarks(x: number, y: number, w: number, h: number): Node[] {
  const len = 9;
  const off = 3;
  const line = (x1: number, y1: number, x2: number, y2: number): Node => ({ type: 'line', x1, y1, x2, y2, lineWidth: 0.6, lineColor: INK });
  return [
    line(x - off - len, y, x - off, y),
    line(x, y - off - len, x, y - off),
    line(x + w + off, y, x + w + off + len, y),
    line(x + w, y - off - len, x + w, y - off),
    line(x - off - len, y + h, x - off, y + h),
    line(x, y + h + off, x, y + h + off + len),
    line(x + w + off, y + h, x + w + off + len, y + h),
    line(x + w, y + h + off, x + w, y + h + off + len),
  ];
}

/** The largest type (7 to 9 points) at which a card's lines fit its 2-inch height. */
export function cardFontSize(c: { title: string; lines: Inline[][] }, width: number): number {
  const text = (l: Inline[]) => l.map((i) => ('t' in i ? i.t : 'b' in i ? i.b : 'link' in i ? i.link.text : 'blank' in i ? '_'.repeat(i.blank) : '')).join('');
  for (const size of [9, 8.5, 8, 7.5, 7]) {
    const perLine = Math.max(8, Math.floor(width / (size * 0.56)));
    const lines = c.lines.reduce((n, l) => n + Math.max(1, Math.ceil([...text(l)].length / perLine)), 0);
    const height = (size + 1.5) * 1.25 + 2 + lines * size * 1.12 * 1.18;
    if (height <= CARD.height - 10) return size;
  }
  return 7;
}

/** A small drawn sample of a map pattern for its key (no colour). */
function swatch(pattern: string): Node[] {
  if (pattern === 'route-1') return [{ type: 'line', x1: 0, y1: 6, x2: 26, y2: 6, lineWidth: 2.5, lineColor: INK }];
  if (pattern === 'route-2') return [{ type: 'line', x1: 0, y1: 6, x2: 26, y2: 6, lineWidth: 2.5, lineColor: INK, dash: { length: 5, space: 3 } }];
  if (pattern === 'county') return [{ type: 'line', x1: 0, y1: 6, x2: 26, y2: 6, lineWidth: 1.5, lineColor: INK, dash: { length: 4, space: 2 } }];
  const marks: Node[] = [{ type: 'rect', x: 0, y: 1, w: 26, h: 10, lineWidth: 0.6, lineColor: INK }];
  for (let x = -10; x < 26; x += 4) {
    if (pattern === 'stripes' || pattern === 'cross') marks.push({ type: 'line', x1: Math.max(0, x), y1: 11 - Math.max(0, -x), x2: Math.min(26, x + 10), y2: 1 + Math.max(0, x + 10 - 26), lineWidth: 0.5, lineColor: INK });
    if (pattern === 'stripes-back' || pattern === 'cross') marks.push({ type: 'line', x1: Math.max(0, x), y1: 1 + Math.max(0, -x), x2: Math.min(26, x + 10), y2: 11 - Math.max(0, x + 10 - 26), lineWidth: 0.5, lineColor: INK });
  }
  if (pattern === 'dots') for (let x = 3; x < 26; x += 5) for (let y = 3.5; y < 11; y += 4) marks.push({ type: 'ellipse', x, y, r1: 0.8, r2: 0.8, color: INK });
  return marks;
}
