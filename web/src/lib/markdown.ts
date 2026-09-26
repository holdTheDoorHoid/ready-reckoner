/**
 * Markdown to safe HTML for the packet and the Learn articles.
 *
 * Two layers. First, the renderer never lets raw HTML through: HTML in the source is shown as
 * text, images become their description (so a packet can never make the browser fetch anything),
 * and links keep only http, https, mailto and in-page targets. Second, DOMPurify cleans the result
 * against a short allowlist of tags and attributes. Footnotes (`[^id]` with `[^id]: text`) become
 * numbered notes at the end, as the content standard uses them.
 */
import DOMPurify from 'dompurify';
import { Marked, Renderer, type Tokens } from 'marked';

const ALLOWED_TAGS = [
  'h2', 'h3', 'h4', 'h5', 'h6', 'p', 'br', 'hr', 'strong', 'em', 'del', 'code', 'pre', 'blockquote',
  'ul', 'ol', 'li', 'table', 'thead', 'tbody', 'tr', 'th', 'td', 'a', 'sup', 'section', 'input', 'label', 'span', 'div',
];
const ALLOWED_ATTR = ['href', 'id', 'class', 'type', 'checked', 'disabled', 'align', 'start', 'aria-label', 'target', 'rel', 'title', 'tabindex', 'role'];
const SAFE_URL = /^(?:https?:\/\/|mailto:|#)/i;

let purifier: ReturnType<typeof DOMPurify> | undefined;

function purify(): ReturnType<typeof DOMPurify> {
  if (purifier) return purifier;
  const p = DOMPurify(window);
  p.addHook('afterSanitizeAttributes', (node) => {
    if (node.nodeName === 'A') {
      const href = node.getAttribute('href') ?? '';
      if (!SAFE_URL.test(href)) node.removeAttribute('href');
      if (/^https?:/i.test(href)) {
        node.setAttribute('target', '_blank');
        node.setAttribute('rel', 'noopener noreferrer');
      } else {
        node.removeAttribute('target');
        node.removeAttribute('rel');
      }
    }
    if (node.nodeName === 'INPUT') {
      if (node.getAttribute('type') !== 'checkbox') node.remove();
      else node.setAttribute('disabled', '');
    }
    // Only the renderer's own table wrapper may carry a role or be focusable.
    const isTableWrap = node.nodeName === 'DIV' && node.getAttribute('class') === 'table-wrap';
    if (!isTableWrap || node.getAttribute('role') !== 'region') node.removeAttribute('role');
    if (!isTableWrap || node.getAttribute('tabindex') !== '0') node.removeAttribute('tabindex');
  });
  purifier = p;
  return p;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

export interface RenderOptions {
  /** Prefix for generated ids, so two rendered documents on one page never clash. */
  idPrefix?: string;
  /** Added to every heading level: 1 turns `#` into h2 when the page already has its h1. */
  headingOffset?: number;
  /** Tables already rendered earlier in the same document, so region names keep counting. */
  tableStart?: number;
  /** The notes' name for screen readers (default "Notes"); give each set on one page its own. */
  notesLabel?: string;
}

const FOOTNOTE_DEF = /^\[\^([A-Za-z0-9_-]+)\]:[ \t]*(.*)$/gm;

/** Render Markdown to sanitised HTML. */
export function renderMarkdown(source: string, options: RenderOptions = {}): string {
  const prefix = (options.idPrefix ?? 'md').replace(/[^A-Za-z0-9_-]/g, '');
  const offset = options.headingOffset ?? 0;

  const definitions = new Map<string, string>();
  const body = source.replace(FOOTNOTE_DEF, (_m, id: string, text: string) => {
    definitions.set(id, text);
    return '';
  });
  const order: string[] = [];
  const refCount = new Map<string, number>();
  let tables = options.tableStart ?? 0;

  const base = new Renderer();
  const marked = new Marked({ gfm: true, breaks: false });
  marked.use({
    renderer: {
      html({ text }: Tokens.HTML | Tokens.Tag): string {
        return escapeHtml(text);
      },
      image({ text }: Tokens.Image): string {
        return text ? `<span class="md-image-text">[${escapeHtml(text)}]</span>` : '';
      },
      link(this: Renderer, token: Tokens.Link): string {
        const inner = this.parser.parseInline(token.tokens);
        if (!SAFE_URL.test(token.href)) return inner;
        const external = /^https?:/i.test(token.href);
        const hidden = external ? '<span class="visually-hidden"> (opens in a new tab)</span>' : '';
        // A bare address shown as its own text: print should not add it again after the link.
        const bare = token.text === token.href || token.raw.startsWith('<');
        return `<a href="${escapeHtml(token.href)}"${bare ? ' class="bare-url"' : ''}${token.title ? ` title="${escapeHtml(token.title)}"` : ''}>${inner}${hidden}</a>`;
      },
      heading(this: Renderer, token: Tokens.Heading): string {
        const level = Math.min(6, Math.max(2, token.depth + offset));
        return `<h${level} class="md-h${token.depth}">${this.parser.parseInline(token.tokens)}</h${level}>\n`;
      },
      table(this: Renderer, token: Tokens.Table): string {
        // Focusable so keyboard users can scroll a wide table sideways on a phone; numbered so
        // each region has its own name.
        tables += 1;
        const first = token.header[0]?.text.replace(/[^\p{L}\p{N} ,.'-]/gu, '').trim();
        const label = `Table ${tables}${first ? `: ${first}` : ''}`;
        return `<div class="table-wrap" tabindex="0" role="region" aria-label="${escapeHtml(label)}">${base.table.call(this, token)}</div>\n`;
      },
      listitem(this: Renderer, item: Tokens.ListItem): string {
        if (!item.task) return base.listitem.call(this, item);
        // A checklist line: the tick box is named by its own text.
        // marked puts a checkbox token first; checkbox() below renders it as nothing.
        const body = this.parser.parse(item.tokens).trim().replace(/^<p>|<\/p>$/g, '');
        return `<li class="task-item"><label><input type="checkbox" disabled${item.checked ? ' checked' : ''}> ${body}</label></li>\n`;
      },
      checkbox(): string {
        return '';
      },
    },
    extensions: [
      {
        name: 'footnoteRef',
        level: 'inline',
        start(src: string) {
          const i = src.indexOf('[^');
          return i < 0 ? undefined : i;
        },
        tokenizer(src: string) {
          const m = /^\[\^([A-Za-z0-9_-]+)\]/.exec(src);
          if (!m) return undefined;
          return { type: 'footnoteRef', raw: m[0], id: m[1]! };
        },
        renderer(token) {
          const id = String(token.id);
          if (!definitions.has(id)) return escapeHtml(String(token.raw));
          if (!order.includes(id)) order.push(id);
          const n = order.indexOf(id) + 1;
          const k = (refCount.get(id) ?? 0) + 1;
          refCount.set(id, k);
          return `<sup class="fnref"><a href="#${prefix}-fn-${id}" id="${prefix}-fnref-${id}-${k}" aria-label="Note ${n}">${n}</a></sup>`;
        },
      },
    ],
  });

  let html = marked.parse(body, { async: false });
  if (order.length > 0) {
    const items = order
      .map((id) => {
        const text = marked.parseInline(definitions.get(id)!, { async: false });
        return `<li id="${prefix}-fn-${id}">${text} <a href="#${prefix}-fnref-${id}-1" class="fn-back" aria-label="Back to the text">↩</a></li>`;
      })
      .join('');
    html += `<section class="footnotes" aria-label="${escapeHtml(options.notesLabel ?? 'Notes')}"><ol>${items}</ol></section>`;
  }
  // DOMPurify's own URL rule stays in force (it checks every attribute value, so a narrower
  // pattern here would also strip type="checkbox"); the hook above then keeps only SAFE_URL links.
  return purify().sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    ALLOW_DATA_ATTR: false,
  });
}

/** How many tables a rendered document holds (to continue numbering in a later part). */
export function countTables(html: string): number {
  return html.split('class="table-wrap"').length - 1;
}

/** One `##` section of the packet (the part before the first one is the cover). */
export interface PacketSection {
  /** From the heading: `summary`, `your-risks`, … `sources`; `cover` for the title block. */
  slug: string;
  markdown: string;
}

/** The packet cut at its `##` headings, so the app can style sections and put the map in one. */
export function packetSections(markdown: string): PacketSection[] {
  const out: PacketSection[] = [];
  const starts = [...markdown.matchAll(/^## (.+)$/gm)];
  const cover = markdown.slice(0, starts[0]?.index ?? markdown.length);
  if (cover.trim()) out.push({ slug: 'cover', markdown: cover });
  starts.forEach((m, i) => {
    const end = starts[i + 1]?.index ?? markdown.length;
    const slug = m[1]!.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    out.push({ slug, markdown: markdown.slice(m.index, end) });
  });
  return out;
}

/**
 * The section that holds the wallet cards: the one whose heading names them ("Wallet cards"), or
 * else the family plan they are printed with; undefined when the packet has neither.
 */
export function cardsSection(sections: readonly PacketSection[]): string | undefined {
  return sections.find((s) => s.slug.includes('wallet'))?.slug ?? sections.find((s) => s.slug.includes('family-plan'))?.slug;
}

/** A section's heading and opening paragraphs, and the rest (from its first subheading). */
export function splitIntro(markdown: string): { intro: string; rest: string } {
  const sub = markdown.slice(1).search(/^#{2,4} /m);
  if (sub < 0) return { intro: markdown, rest: '' };
  return { intro: markdown.slice(0, sub + 1), rest: markdown.slice(sub + 1) };
}
