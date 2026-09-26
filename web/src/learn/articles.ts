/**
 * Learn articles (docs/UI.md screen 10): the five topic blocks the content workstream writes,
 * cites and reviews in `content/guidance/topic_*.md`, read at build time, so the site shows exactly
 * the reviewed text with its sources (a missing file fails the build rather than hiding an
 * article). Each keeps the address the site has always used (`#/learn/numbers`); its title comes
 * from the block's front matter, and the one-line summary on the list is the site's own. A file in
 * `content/learn/*.md` with front matter (`title:`, `summary:`) adds an article, or replaces one
 * with the same slug.
 *
 * Until v0.1.0 the site showed stand-in drafts written for the web, labelled "Draft text, still
 * being reviewed". To put that label back on an article while its text is reviewed again, add its
 * slug to `IN_REVIEW`.
 */
import childrenRaw from '../../../content/guidance/topic_talking_with_children.md?raw';
import consequencesRaw from '../../../content/guidance/topic_consequences_not_causes.md?raw';
import mythsRaw from '../../../content/guidance/topic_disaster_myths.md?raw';
import neighboursRaw from '../../../content/guidance/topic_neighbours.md?raw';
import numbersRaw from '../../../content/guidance/topic_how_numbers_are_made.md?raw';
import type { Citation } from '../engine/types';

export interface Article {
  slug: string;
  title: string;
  summary: string;
  body: string;
  draft: boolean;
}

/** Slugs whose text is not fit to publish yet: they show "Draft text, still being reviewed". */
const IN_REVIEW: ReadonlySet<string> = new Set<string>();

const TOPICS: { slug: string; raw: string; summary: string }[] = [
  {
    slug: 'consequences',
    raw: consequencesRaw,
    summary: 'Why the plan is built around no power, no water and no store, instead of a list per disaster.',
  },
  { slug: 'myths', raw: mythsRaw, summary: 'Panic and looting are much rarer than films suggest. Neighbours are the first help.' },
  { slug: 'numbers', raw: numbersRaw, summary: 'From your county and household to days to be ready for, and where each number comes from.' },
  { slug: 'children', raw: childrenRaw, summary: 'Simple facts, a part to play, and practice together.' },
  { slug: 'community', raw: neighboursRaw, summary: 'Knowing the people nearby is one of the strongest protections there is, and it is free.' },
];

/** Articles from `content/learn/*.md`, if the content workstream has added any. */
const FROM_CONTENT = import.meta.glob('../../../content/learn/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

function parseFrontMatter(raw: string): { meta: Record<string, string>; body: string } {
  const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(raw);
  if (!m) return { meta: {}, body: raw };
  const meta: Record<string, string> = {};
  for (const line of m[1]!.split(/\r?\n/)) {
    const kv = /^([A-Za-z_]+):\s*(.*)$/.exec(line);
    if (kv) meta[kv[1]!] = kv[2]!.replace(/^["']|["']$/g, '');
  }
  return { meta, body: raw.slice(m[0].length) };
}

function fromContent(): Article[] {
  return Object.entries(FROM_CONTENT).map(([path, raw]) => {
    const slug = path.replace(/^.*\//, '').replace(/\.md$/, '');
    const { meta, body } = parseFrontMatter(raw);
    return { slug, title: meta.title ?? slug, summary: meta.summary ?? '', body, draft: false };
  });
}

function fromTopics(): Article[] {
  return TOPICS.map(({ slug, raw, summary }) => {
    const { meta, body } = parseFrontMatter(raw);
    return { slug, title: meta.title ?? slug, summary, body, draft: false };
  });
}

export const ARTICLES: Article[] = (() => {
  const real = fromContent();
  const bySlug = new Map(real.map((a) => [a.slug, a]));
  const topics = fromTopics();
  const merged = topics.map((t) => bySlug.get(t.slug) ?? t);
  for (const a of real) if (!topics.some((t) => t.slug === a.slug)) merged.push(a);
  return merged.map((a) => ({ ...a, draft: IN_REVIEW.has(a.slug) }));
})();

const FOOTNOTE_DEF = /^\[\^([A-Za-z0-9_-]+)\]:[ \t]*(.*)$/gm;

/**
 * Each note that names a source in `content/citations.toml` (the note's id is the citation id)
 * gets a link to it, named by its website, so every source in an article can be opened.
 */
export function withSourceLinks(body: string, citations: readonly Citation[] | undefined): string {
  if (!citations?.length) return body;
  const byId = new Map(citations.map((c) => [c.id, c]));
  return body.replace(FOOTNOTE_DEF, (line, id: string, text: string) => {
    const url = byId.get(id)?.url;
    if (!url || !/^https?:\/\/[^\s()<>]+$/.test(url)) return line;
    return `[^${id}]: ${text} [${new URL(url).hostname.replace(/^www\./, '')}](${url})`;
  });
}

export function article(slug: string): Article | undefined {
  return ARTICLES.find((a) => a.slug === slug);
}
