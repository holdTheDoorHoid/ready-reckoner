/**
 * Guidance blocks shown on screen (docs/UI.md "Your family plan"): the plan blocks the content
 * workstream writes, cites and reviews in `content/guidance/plan_*.md` (`plan_communication`,
 * `plan_shelter`), read at build time, so the family-plan screen shows exactly the reviewed text
 * with its sources. The files are found by pattern, so a build before they exist still works:
 * the screen then shows its own short help instead.
 *
 * A block is shown for the household: its conditional spans kept or dropped as the packet keeps
 * them (`lib/conditions.ts`), its "Sources" heading removed (the notes follow the text, each linked
 * to its source), and nothing else changed.
 */
import { applyConditionsFor, type HouseholdFacts } from './conditions';

// awaiting: content2 (content/guidance/plan_communication.md and plan_shelter.md)
const RAW = import.meta.glob('../../../content/guidance/plan_*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

export interface GuidanceBlock {
  id: string;
  title: string;
  citations: string[];
  /** The Markdown after the front matter. */
  body: string;
}

/** A block from its file text: the front matter's id, title and citations, and the body. */
export function parseBlock(raw: string): GuidanceBlock | undefined {
  const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(raw);
  if (!m) return undefined;
  const meta: Record<string, string> = {};
  for (const line of m[1]!.split(/\r?\n/)) {
    const kv = /^([A-Za-z_]+):\s*(.*)$/.exec(line);
    if (kv) meta[kv[1]!] = kv[2]!.trim().replace(/^["']|["']$/g, '');
  }
  if (!meta.id) return undefined;
  const citations = (meta.citations ?? '')
    .replace(/^\[|\]$/g, '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  return { id: meta.id, title: meta.title ?? meta.id, citations, body: raw.slice(m[0].length) };
}

const BLOCKS: Map<string, GuidanceBlock> = new Map(
  Object.values(RAW).flatMap((raw) => {
    const b = parseBlock(raw);
    return b ? [[b.id, b] as const] : [];
  }),
);

/** A plan block by id, or undefined when this build has no such file. */
export function guidanceBlock(id: string, blocks: ReadonlyMap<string, GuidanceBlock> = BLOCKS): GuidanceBlock | undefined {
  return blocks.get(id);
}

/**
 * The block's body for this household: spans kept or dropped, the "Sources" heading removed, and
 * runs of blank lines left by dropped paragraphs closed up. Footnotes stay (the renderer turns
 * them into numbered notes).
 */
export function blockText(block: GuidanceBlock, household: HouseholdFacts | null): string {
  return applyConditionsFor(block.body, household)
    .replace(/^#{1,6}\s+Sources\s*$/gm, '')
    .replace(/^[ \t]+$/gm, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}
