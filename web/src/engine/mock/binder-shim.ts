/**
 * transitional: replaced by the binder workstream (DESIGN-DELTA-v3 §4, §11).
 *
 * The mock engine's binder, built from its packet Markdown exactly the way the engine builds its
 * transitional binder (`crates/rr-plan/src/packet/shim.rs`), so the two stay shape-identical for
 * the parity test:
 *
 * - the text before the first `##` (under the `#` title) is the cover page;
 * - each `##` section is one page of `para` blocks, one per Markdown paragraph, the text kept as
 *   it is;
 * - each page goes in the part of DESIGN-DELTA-v3 §4.2 that will hold that material, and a part
 *   with no pages is left out;
 * - `sources` are the provenance list in order, `credits` the data attributions.
 */
import type { Attribution, Binder, Block, Citation, IsoDate, Page, PageKind, Part, SourceEntry } from '../types';
import { formatDate } from '../../lib/format';

/** The ten parts of DESIGN-DELTA-v3 §4.2: tab, id, title and tab label (as the engine's shim). */
export const SHIM_PARTS: readonly (readonly [number, string, string, string])[] = [
  [1, 'start', 'Start here', 'Start here'],
  [2, 'people', 'People', 'People'],
  [3, 'home_places', 'Home and places', 'Home & places'],
  [4, 'pets_vehicles_documents', 'Pets, vehicles and documents', 'Pets & docs'],
  [5, 'have', 'What you have', 'What you have'],
  [6, 'check_now', 'Checklists: happening now', 'Happening now'],
  [7, 'check_coming', 'Checklists: it is coming', 'It is coming'],
  [8, 'check_ongoing', 'Checklists: it goes on', 'It goes on'],
  [9, 'after', 'After', 'After'],
  [10, 'sources', 'Sources', 'Sources'],
];

/** The engine's v2 `##` sections: title, tab, page id and kind (as the engine's shim). */
const SECTIONS: readonly (readonly [string, number, string, PageKind])[] = [
  ['Summary', 1, 'summary', 'quick_start'],
  ['Your family plan', 1, 'family_plan', 'contacts'],
  ['Wallet cards', 2, 'wallet_cards', 'wallet_cards'],
  ['Access and functional needs', 2, 'access_needs', 'person'],
  ['Special needs', 2, 'special_needs', 'person'],
  ['Your shelter plan', 3, 'shelter_plan', 'home'],
  ['Local help', 3, 'local_help', 'neighbourhood'],
  ['Documents and money', 4, 'documents', 'documents'],
  ['Your risks', 5, 'risks', 'risks_glance'],
  ['Your targets', 5, 'targets', 'inventory'],
  ['Your plan', 5, 'plan', 'inventory'],
  ['Checklists', 5, 'checklists', 'inventory'],
  ['Maintenance calendar', 5, 'maintenance', 'inventory'],
  ['When a storm, freeze or heat wave is forecast', 7, 'forecast', 'checklist'],
  ['After a disaster: the first 30 days', 9, 'after', 'after'],
  ['If it lasts for months', 9, 'months', 'after'],
  ['Sources', 10, 'sources', 'sources'],
];

/** Where a section the table does not name goes: the "What you have" tab. */
const OTHER_TAB = 5;

const DEFAULT_TITLE = 'Your preparedness packet';

/** The engine's status line (rr-plan `packet::STATUS_LINE`). */
export const STATUS_LINE =
  'Ready Reckoner is an independent, open-source planning aid. It is not official emergency guidance, ' +
  'and not medical, legal or financial advice. Follow instructions from your local officials first.';

/** The cover facts the shim takes from the plan. */
export interface ShimFacts {
  generatedOn: IsoDate;
  household: string;
  location: string;
  reviewBy: IsoDate;
  provenance: Citation[];
  attributions: Attribution[];
}

/** The transitional binder for a packet's Markdown. */
export function shimBinder(markdown: string, facts: ShimFacts): Binder {
  const { title, cover, sections } = split(markdown);
  const pages: [number, Page][] = [[1, page('cover', title, 'cover', paragraphs(cover))]];
  const used = ['cover'];
  for (const [heading, body] of sections) {
    const known = SECTIONS.find(([t]) => t === heading);
    const [tab, base, kind]: [number, string, PageKind] = known ? [known[1], known[2], known[3]] : [OTHER_TAB, slug(heading), 'inventory'];
    const id = unique(base, used);
    used.push(id);
    pages.push([tab, page(id, heading, kind, paragraphs(body))]);
  }
  const parts: Part[] = [];
  for (const [tab, id, partTitle, short] of SHIM_PARTS) {
    const mine = pages.filter(([t]) => t === tab).map(([, p]) => p);
    if (mine.length) parts.push({ id, tab, title: partTitle, short_title: short, pages: mine });
  }
  return {
    title,
    generated_on: facts.generatedOn,
    household: facts.household,
    location: facts.location,
    status_line: STATUS_LINE,
    review_by: facts.reviewBy,
    parts,
    sources: facts.provenance.map((c, i) => source(i, c)),
    credits: facts.attributions.map(credit),
  };
}

function page(id: string, title: string, kind: PageKind, blocks: Block[]): Page {
  return { id, title, kind, fit: 'flow', blocks };
}

/** The packet split into its `#` title, the text before the first `##`, and each `##` section. */
function split(markdown: string): { title: string; cover: string; sections: [string, string][] } {
  let title: string | undefined;
  let cover = '';
  const sections: [string, string][] = [];
  for (const line of markdown.split('\n')) {
    if (line.startsWith('## ')) {
      sections.push([line.slice(3).trim(), '']);
      continue;
    }
    if (title === undefined && sections.length === 0 && line.startsWith('# ')) {
      title = line.slice(2).trim();
      continue;
    }
    const last = sections[sections.length - 1];
    if (last) last[1] += `${line}\n`;
    else cover += `${line}\n`;
  }
  return { title: title ?? DEFAULT_TITLE, cover, sections };
}

/** One `para` block per Markdown paragraph, the text as it is. */
function paragraphs(body: string): Block[] {
  return body
    .split('\n\n')
    .map((p) => p.trim())
    .filter((p) => p !== '')
    .map((p) => ({ para: [{ t: p }] }));
}

/** A page id for a heading the table does not name (as the engine's shim). */
function slug(heading: string): string {
  let out = '';
  for (const c of heading) {
    if (/[A-Za-z0-9]/.test(c)) out += c.toLowerCase();
    else if (out !== '' && !out.endsWith('_')) out += '_';
  }
  const trimmed = out.replace(/_+$/, '');
  return trimmed === '' ? 'section' : trimmed;
}

function unique(id: string, used: string[]): string {
  if (!used.includes(id)) return id;
  for (let n = 2; ; n++) {
    const candidate = `${id}_${n}`;
    if (!used.includes(candidate)) return candidate;
  }
}

function source(i: number, c: Citation): SourceEntry {
  const entry: SourceEntry = { n: i + 1, title: c.title, publisher: c.publisher, expert: c.prior ?? false };
  if (c.year !== undefined) entry.year = c.year;
  if (c.url.trim() !== '') entry.url = c.url;
  return entry;
}

/** A data credit as plain text: source, version and access date, then the statement as given. */
function credit(at: Attribution): string {
  const version = at.version ? `, version ${at.version}` : '';
  const url = at.text.includes(at.url) ? '' : ` ${at.url}`;
  return `${at.source}${version}, accessed ${formatDate(at.accessed)}. ${at.text}${url}`;
}
