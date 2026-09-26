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
  'ul', 'ol', 'li', 'table', 'thead', 'tbody', 'tr', 'th', 'td', 'a', 'sup', 'section', 'input', 'span', 'div',
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
        return `<a href="${escapeHtml(token.href)}"${token.title ? ` title="${escapeHtml(token.title)}"` : ''}>${inner}${hidden}</a>`;
      },
      heading(this: Renderer, token: Tokens.Heading): string {
        const level = Math.min(6, Math.max(2, token.depth + offset));
        return `<h${level} class="md-h${token.depth}">${this.parser.parseInline(token.tokens)}</h${level}>\n`;
      },
      table(this: Renderer, token: Tokens.Table): string {
        // Focusable so keyboard users can scroll a wide table sideways on a phone.
        return `<div class="table-wrap" tabindex="0" role="region" aria-label="Table">${base.table.call(this, token)}</div>\n`;
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
    html += `<section class="footnotes" aria-label="Notes"><ol>${items}</ol></section>`;
  }
  return purify().sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    ALLOW_DATA_ATTR: false,
    ALLOWED_URI_REGEXP: SAFE_URL,
  });
}
