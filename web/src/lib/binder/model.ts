/**
 * Reading the engine's binder tree (DESIGN-DELTA-v3 §4.1): its pages in order, the table of
 * contents, where `#/binder/<page-id>` leads, the checks `Binder::check` makes, the fit proxy of
 * §5.6 and the PDF's file name. Both renderers (the screen and the PDF) read the tree through
 * these helpers, so the two always agree on page order, ids and numbering.
 *
 * Nothing here parses text: every string in the tree is shown exactly as the engine wrote it
 * (user answers are echoed, never read as Markdown or markup).
 */
import type { Binder, Block, Inline, IsoDate, LocationResolved, Page, PageKind, Part } from '../../engine/types';

/**
 * Said on the Neighborhood page (on screen) and after Download PDF when the county hospital list
 * has not been downloaded (`hospitalListMissing` in engine/loader.ts; verify3 R4-05).
 */
export const HOSPITAL_LIST_MISSING = 'The county hospital list has not been downloaded yet. Open the binder once while online and it will be added.';

/** One page with where it sits: its part, its place in the whole binder and in its part. */
export interface PageEntry {
  part: Part;
  page: Page;
  /** 0-based, across the whole binder. */
  index: number;
  /** 0-based, within its part. */
  inPart: number;
}

/** Every page in reading order (part by part). */
export function binderPages(b: Binder): PageEntry[] {
  const out: PageEntry[] = [];
  for (const part of b.parts) {
    part.pages.forEach((page, inPart) => out.push({ part, page, index: out.length, inPart }));
  }
  return out;
}

/** The pages by id (the first one wins if an id is repeated, which `binderProblems` reports). */
export function pageIndex(b: Binder): Map<string, PageEntry> {
  const map = new Map<string, PageEntry>();
  for (const e of binderPages(b)) if (!map.has(e.page.id)) map.set(e.page.id, e);
  return map;
}

/**
 * The page an address names (`#/binder/<param>`): the page with that id; else the same id
 * written with hyphens or in capitals (`wallet-cards` for `wallet_cards`); else the first page of
 * that kind. The v0.2 request for the wallet cards (`#/packet/wallet-cards`, now
 * `#/binder/wallet-cards`) lands on the wallet cards page whatever its id.
 */
export function findPage(b: Binder, param: string | undefined): PageEntry | undefined {
  if (!param) return undefined;
  const pages = binderPages(b);
  const exact = pages.find((e) => e.page.id === param);
  if (exact) return exact;
  const norm = normalizeId(param);
  return pages.find((e) => normalizeId(e.page.id) === norm) ?? pages.find((e) => normalizeId(e.page.kind) === norm);
}

function normalizeId(s: string): string {
  return s.trim().toLowerCase().replace(/[-\s]+/g, '_');
}

/** A page id made safe for an HTML id attribute (the engine's ids are already plain). */
export function pageDomId(id: string): string {
  return `binder-${id.replace(/[^A-Za-z0-9_-]/g, '_')}`;
}

/** The DOM id of a numbered source on the Sources page. */
export function sourceDomId(n: number): string {
  return `binder-source-${n}`;
}

/** The DOM id of a part's heading. */
export function partDomId(id: string): string {
  return `binder-part-${id.replace(/[^A-Za-z0-9_-]/g, '_')}`;
}

// ---------------------------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------------------------

/** An inline run as plain text: blanks as a line of underscores, citations as "[3]". */
export function inlineText(inlines: readonly Inline[]): string {
  return inlines
    .map((i) => {
      if ('t' in i) return i.t;
      if ('b' in i) return i.b;
      if ('link' in i) return i.link.text;
      if ('cite' in i) return i.cite.length ? `[${i.cite.join(', ')}]` : '';
      return '_'.repeat(Math.max(3, Math.min(40, i.blank)));
    })
    .join('');
}

/** Every piece of text a block shows, in reading order (for search, estimates and font checks). */
export function blockTexts(block: Block): string[] {
  if ('heading' in block) return [block.heading.text];
  if ('para' in block) return [inlineText(block.para)];
  if ('bullets' in block) return block.bullets.map(inlineText);
  if ('numbered' in block) return block.numbered.map(inlineText);
  if ('steps' in block) return block.steps.map((s) => inlineText(s.text));
  if ('fields' in block) return block.fields.flatMap((f) => (f.value !== undefined ? [f.label, f.value] : [f.label]));
  if ('table' in block) return [...block.table.header, ...block.table.rows.flatMap((r) => r.map(inlineText))];
  if ('callout' in block) return [...(block.callout.title ? [block.callout.title] : []), ...block.callout.blocks.flatMap(blockTexts)];
  if ('decision' in block) return [block.decision.question, ...block.decision.branches.flatMap((br) => [inlineText(br.when), inlineText(br.then)])];
  if ('map_slot' in block) return [block.map_slot.caption];
  if ('cards' in block) return block.cards.flatMap((c) => [c.title, ...c.lines.map(inlineText)]);
  if ('log' in block) return block.log.columns;
  return [];
}

/** Every piece of text in the binder, cover facts and sources included. */
export function binderTexts(b: Binder): string[] {
  const out = [b.title, b.household, b.location, b.status_line, ...b.credits];
  for (const part of b.parts) {
    out.push(part.title, part.short_title);
    for (const page of part.pages) out.push(page.title, ...page.blocks.flatMap(blockTexts));
  }
  for (const s of b.sources) out.push(s.title, s.publisher, s.url ?? '');
  return out;
}

/** Every block of a page, callouts opened up (a callout comes before its own blocks). */
export function flatBlocks(blocks: readonly Block[]): Block[] {
  return blocks.flatMap((bl) => ('callout' in bl ? [bl, ...flatBlocks(bl.callout.blocks)] : [bl]));
}

/** What kind of block this is: its one key. */
export type BlockKind = 'heading' | 'para' | 'bullets' | 'numbered' | 'steps' | 'fields' | 'table' | 'callout' | 'decision' | 'map_slot' | 'cards' | 'log' | 'page_break';

export const BLOCK_KINDS: readonly BlockKind[] = ['heading', 'para', 'bullets', 'numbered', 'steps', 'fields', 'table', 'callout', 'decision', 'map_slot', 'cards', 'log', 'page_break'];

export function blockKind(block: Block): BlockKind {
  return Object.keys(block)[0] as BlockKind;
}

/** The inlines a block holds, in reading order. */
export function blockInlines(block: Block): Inline[] {
  if ('para' in block) return block.para;
  if ('bullets' in block) return block.bullets.flat();
  if ('numbered' in block) return block.numbered.flat();
  if ('steps' in block) return block.steps.flatMap((s) => s.text);
  if ('table' in block) return block.table.rows.flat(2);
  if ('callout' in block) return block.callout.blocks.flatMap(blockInlines);
  if ('decision' in block) return block.decision.branches.flatMap((br) => [...br.when, ...br.then]);
  if ('cards' in block) return block.cards.flatMap((c) => c.lines.flat());
  return [];
}

/**
 * On the Sources page, the block that lists the numbered sources: a numbered list with one item
 * per source (the engine prints the list itself; citation numbers point to its items). -1 when
 * the page has none, and the renderers then print the sources from `Binder.sources`.
 */
export function sourceListIndex(page: Page, b: Binder): number {
  if (page.kind !== 'sources' || b.sources.length === 0) return -1;
  return page.blocks.findIndex((bl) => 'numbered' in bl && bl.numbered.length === b.sources.length);
}

/** The data credits a page does not already print (the engine may list them on the Sources page). */
export function creditsNotShown(page: Page, b: Binder): string[] {
  const shown = page.blocks.flatMap(blockTexts);
  return b.credits.filter((c) => !shown.some((s) => s.includes(c)));
}

// ---------------------------------------------------------------------------------------------
// Table of contents
// ---------------------------------------------------------------------------------------------

export interface TocPart {
  id: string;
  tab: number;
  title: string;
  shortTitle: string;
  pages: { id: string; title: string; kind: PageKind; index: number }[];
}

/** Parts and their pages, as both tables of contents list them. */
export function tableOfContents(b: Binder): TocPart[] {
  let index = 0;
  return b.parts.map((part) => ({
    id: part.id,
    tab: part.tab,
    title: part.title,
    shortTitle: part.short_title,
    pages: part.pages.map((p) => ({ id: p.id, title: p.title, kind: p.kind, index: index++ })),
  }));
}

// ---------------------------------------------------------------------------------------------
// Structure (the checks `Binder::check` makes, docs/ENGINE-API.md "The binder")
// ---------------------------------------------------------------------------------------------

/** Tabs are numbered from 1 to this (types.ts `MAX_TABS`). */
const MAX_TABS = 10;
/** Longest tab label (types.ts `SHORT_TITLE_MAX`). */
const SHORT_TITLE_MAX = 14;

/** Every structural problem, in `Binder::check`'s order; empty when the tree is sound. */
export function binderProblems(b: Binder): string[] {
  const out: string[] = [];
  let last = 0;
  const partIds = new Set<string>();
  for (const p of b.parts) {
    if (p.tab < 1 || p.tab > MAX_TABS || p.tab <= last) out.push(`part ${p.id}: tab ${p.tab} after ${last}`);
    last = p.tab;
    if (partIds.has(p.id)) out.push(`part ${p.id} used twice`);
    partIds.add(p.id);
    if (p.pages.length === 0) out.push(`part ${p.id} has no pages`);
    if ([...p.short_title].length > SHORT_TITLE_MAX) out.push(`part ${p.id}: tab label "${p.short_title}" is over ${SHORT_TITLE_MAX} characters`);
  }
  const ids = new Set<string>();
  for (const { page } of binderPages(b)) {
    if (ids.has(page.id)) out.push(`page ${page.id} used twice`);
    ids.add(page.id);
  }
  for (const { page } of binderPages(b)) {
    for (const block of flatBlocks(page.blocks)) {
      if ('decision' in block) for (const br of block.decision.branches) if (br.go_to !== undefined && !ids.has(br.go_to)) out.push(`page ${page.id}: go_to ${br.go_to} names no page`);
      for (const i of blockInlines(block)) {
        if ('link' in i && !ids.has(i.link.to)) out.push(`page ${page.id}: link to ${i.link.to} names no page`);
        if ('cite' in i) for (const n of i.cite) if (n < 1 || n > b.sources.length) out.push(`page ${page.id}: cite ${n} is not a source`);
      }
      if ('heading' in block && (block.heading.level < 1 || block.heading.level > 3)) out.push(`page ${page.id}: heading level ${block.heading.level}`);
    }
  }
  b.sources.forEach((s, i) => {
    if (s.n !== i + 1) out.push(`source ${i + 1} is numbered ${s.n}`);
  });
  return out;
}

// ---------------------------------------------------------------------------------------------
// The fit proxy (DESIGN-DELTA-v3 §5.6), for calibrating it against the real PDF
// ---------------------------------------------------------------------------------------------

/** Word units on one printed Letter page (§5.6, "a starting estimate"). */
export const FIT_CAPACITY = 480;

function words(s: string): number {
  return s.split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

/**
 * A page's load in word units, as rr-plan counts it (§5.6): a heading 6, a step or bullet its
 * words plus 2, a field row 8, a table row 10, a paragraph its words. Blocks the section does
 * not price are counted the nearest way: a numbered item as a bullet, a decision branch as a
 * table row, a log row as a table row, a card as eight field rows, a map as half a page.
 */
export function fitUnits(page: Page): number {
  let units = 0;
  const count = (blocks: readonly Block[]) => {
    for (const bl of blocks) {
      if ('heading' in bl) units += 6;
      else if ('para' in bl) units += words(inlineText(bl.para));
      else if ('bullets' in bl) units += bl.bullets.reduce((n, it) => n + words(inlineText(it)) + 2, 0);
      else if ('numbered' in bl) units += bl.numbered.reduce((n, it) => n + words(inlineText(it)) + 2, 0);
      else if ('steps' in bl) units += bl.steps.reduce((n, s) => n + words(inlineText(s.text)) + 2, 0);
      else if ('fields' in bl) units += bl.fields.length * 8;
      else if ('table' in bl) units += (bl.table.rows.length + 1) * 10;
      else if ('callout' in bl) {
        units += bl.callout.title ? 6 : 0;
        count(bl.callout.blocks);
      } else if ('decision' in bl) units += 6 + bl.decision.branches.length * 10;
      else if ('log' in bl) units += (bl.log.rows + 1) * 10;
      else if ('cards' in bl) units += bl.cards.length * 64;
      else if ('map_slot' in bl) units += FIT_CAPACITY / 2;
    }
  };
  count(page.blocks);
  return units;
}

// ---------------------------------------------------------------------------------------------
// The PDF's name
// ---------------------------------------------------------------------------------------------

/** Lower-case words joined by hyphens, accents dropped: "Doña Ana County" → "dona-ana-county". */
export function slugify(s: string): string {
  return s
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/**
 * The county part of the file name: the county as the binder names it ("Philadelphia County")
 * with the state's two letters when known, so the thirty Washington Counties stay apart:
 * `philadelphia-county-pa`.
 */
export function countySlug(b: Binder, location?: Pick<LocationResolved, 'state_abbr'> | null): string {
  const county = b.location.split(',')[0] ?? '';
  const slug = slugify([county, location?.state_abbr ?? ''].join(' '));
  return slug || 'binder';
}

/** `ready-reckoner-binder-<county-slug>-<generated_on>.pdf` (DESIGN-DELTA-v3 §6). */
export function pdfFileName(b: Binder, location?: Pick<LocationResolved, 'state_abbr'> | null): string {
  return `ready-reckoner-binder-${countySlug(b, location)}-${safeDate(b.generated_on)}.pdf`;
}

function safeDate(d: IsoDate): string {
  return /^\d{4}-\d{2}-\d{2}$/.test(d) ? d : slugify(d) || 'undated';
}
